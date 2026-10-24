# Milestone 3 — Win32 GUI via Web-Streamed Canvas (Detailed Design Specification)

## Status

- **Phase**: Approved — Detailed Design (Architecture Revision 3)
- **Engine**: ClassicBasic (`msvbvm60` + `vb6-run`)
- **Current Baseline Version**: `v0.2.17` (Milestone 1 at 100% + Milestone 2 at 100% $\to$ **80.0%** overall completion toward `v1.0.0`)
- **Milestone 3 Version Range**: `v0.3.1` $\to$ `v1.0.0` (10 Phases, each $+2.0\%$ overall engine progress)
- **Runtime Target**: `x86_64-alpine-linux-musl`
- **Client Target**: `wasm32-unknown-unknown`
- **Hard Constraints**: Clean-room implementation, zero new third-party dependencies in `Cargo.toml`, containerization-first, no Wine, no X11/Wayland/Xvfb, no React/Node in runtime path.

---

## 1. Architectural Adjustments & Critical Directives

### 1.1 Text Rendering Pipeline (Server-Client Split)
- **Design Rule**: The server-side GDI engine (`crates/cb-win32/src/gdi32.rs`) **never** rasterizes glyphs into the BGRA framebuffer.
- **Server Responsibility**: Maintains DC text state (`SetTextColor`, `SetBkColor`, `SetBkMode`, current font metadata). Intercepts `TextOutA`/`TextOutW`/`ExtTextOutA`/`DrawTextA` and emits a structured binary frame (`0x07 FRAME_TEXT`). Satisfies `GetTextExtentPoint32A` and `GetTextMetricsA` using an embedded, static 96 DPI font metric lookup table (`MS Sans Serif` / `Tahoma` / `Arial` / `Courier New` canonical widths) so VB6 layout logic executes deterministically.
- **Client Responsibility**: The WASM client (`crates/cb-client/`) consumes `FRAME_TEXT` and dispatches drawing directly to the HTML5 Canvas via `ctx.fillText()` / `ctx.strokeText()`, leveraging client-side system fonts.

### 1.2 Coordinate System (Twips to Pixels)
- **Design Rule**: All internal VB6 form definitions (`.rsrc`, `GUITable`, `ThunderRT6FormDC`, control properties) store dimensions in **Twips** ($1 \text{ pixel} = 15 \text{ Twips}$ at 96 DPI standard, $1440 \text{ Twips/inch}$).
- **Conversion Equation**:
  $$\text{pixels} = \left\lfloor \frac{\text{twips}}{15} \right\rfloor, \quad \text{twips} = \text{pixels} \times 15$$
- `crates/msvbvm60/` converts all form/control bounds from Twips to physical virtual pixels prior to inserting nodes into `VirtualWindowTree` (`crates/cb-win32/`). Win32 APIs querying coordinates (`GetClientRect`, `GetWindowRect`) report virtual pixels, while VB6 COM properties (`Form.Width`, `Control.Left`) report Twips (unless `ScaleMode = vbPixels`).

### 1.3 Atomic Frame Protocol (Elimination of Tearing)
- Merged dirty region notifications and raw pixel payloads into a single atomic frame type (`0x02 FRAME_BITMAP_RECT` in `crates/cb-net/`), removing race conditions and multi-packet frame tearing.

### 1.4 Non-Blocking Message Loop & Thread Synchronization
- The `VirtualMessageQueue` in `crates/cb-win32/src/user32.rs` uses a thread-safe MPSC queue synchronized via `std::sync::Mutex<VecDeque<Win32Msg>>` and `std::sync::Condvar`.
- `GetMessageA` blocks the UI execution thread on the `Condvar`. When WebSocket input frames (`mouse`, `key`, `resize`, `dialog.result`) arrive via `crates/cb-net/`, the network thread pushes a `Win32Msg` and signals the condition variable, waking `GetMessageA` without CPU spin-locking.
- `PeekMessageA` inspects the queue non-blockingly and returns immediately.

### 1.5 Control Event Sink & Callback Dispatch (Native x86 & P-Code)
- `crates/msvbvm60/` binds deserialized controls to their respective VB6 event handlers compiled in `.text`.
- The parser inspects `VBHeader -> lp_gui_table / ObjectTable -> ObjectInfo -> ControlTable` to resolve event sink offsets (e.g., `Form_Load`, `Command1_Click`, `Text1_Change`).
- When a `VirtualWindow` receives `WM_LBUTTONUP`, `WM_CHAR`, or `WM_TIMER`, its window procedure maps the control ID to the event handler address and invokes it through either `cb_x86::X86Cpu` (for Native PE32 binaries) or `PCodeVm` (for P-Code PE32 binaries).

---

## 2. Subsystem & Module Decomposition (`crates/*` Workspace)

| Workspace Crate | Kind | Role |
| :--- | :--- | :--- |
| `crates/cb-x86` | rlib | Clean-room 32-bit x86 CPU (`X86Cpu`), EFLAGS, x87 FPU, 32-bit linear virtual memory (`X86Memory`), Win32 `FS:[0]` TIB & SEH unwinding. |
| `crates/cb-pe` | rlib | PE32 headers (`ImageDosHeader`, `ImageOptionalHeader32`, etc.), `VBHeader` / `ProjectInfo` / `ObjectTable`, and canonical 635-entry `MSVBVM60.DLL` ordinal table. |
| `crates/cb-vfs` | rlib | Sandboxed Virtual Registry (`registry.json`), 128-bit COM `Guid`/`Clsid`/`Iid`, Windows-to-POSIX virtual path resolver, and `SandboxProvider` trait. |
| `crates/cb-win32` | rlib | Clean-room `KERNEL32` + `ADVAPI32` + `USER32` + `GDI32`: 32-bit handle table (`HWND`, `HDC`, `HGDIOBJ`), `VirtualWindowTree`, `Condvar` message queue, 32-bit RGBA `Surface32`, pens/brushes, 96 DPI font metrics, dirty-rect tracker. |
| `crates/cb-net` | rlib | Hand-rolled RFC 6455 WebSocket server, HTTP/1.1 handshake, atomic binary frame encoder (`0x02`..`0x07`), client JSON input decoder, static asset server. |
| `crates/msvbvm60` | cdylib + rlib | Core VB6 VM ABI (635/635 ordinals), BSTR/VARIANT/SAFEARRAY, `rtc*`, `__vba*`, COM/ActiveX, TypeLib, PE32 loader/resolver, `PCodeVm`, `X86CpuVbExt` IAT bridge, and `ThunderRT6*` form/control runtime. |
| `crates/vb6-run` | bin | Unified standalone runner supporting `--headless` and `--gui-web` execution modes. |
| `crates/cb-session` | rlib + bin | Multi-tenant session broker: container/process provisioning, TCP/WebSocket reverse proxy, and isolated sandbox orchestration. |
| `crates/cb-client` | rlib (`wasm32`) | Zero-dependency WebAssembly Canvas renderer, BGRA/RGBA blitter, `FRAME_TEXT` dispatcher, DOM event bridge, minimal JS shim ($\le 200$ lines). |

---

## 3. Win32 API Surface Specifications

### 3.1 USER32 — Virtual Windowing & Synchronization
- **Window Class & Lifecycle**: `RegisterClassA`, `RegisterClassW`, `UnregisterClassA`, `CreateWindowExA`, `CreateWindowExW`, `DestroyWindow`, `ShowWindow`, `UpdateWindow`, `MoveWindow`, `SetWindowPos`, `IsWindow`, `IsWindowVisible`, `EnableWindow`, `SetFocus`, `GetFocus`.
- **Message Pipeline**:
  - `GetMessageA`, `GetMessageW`: Thread-blocking read from `VirtualMessageQueue` backed by `Condvar`.
  - `PeekMessageA`, `PeekMessageW`: Non-blocking read (`PM_NOREMOVE`, `PM_REMOVE`).
  - `TranslateMessage`, `DispatchMessageA`, `DefWindowProcA`, `PostQuitMessage`, `SendMessageA`, `PostMessageA`.
- **Geometry, Text & Painting**: `SetWindowTextA`, `GetWindowTextA`, `GetWindowTextLengthA`, `GetClientRect`, `GetWindowRect`, `InvalidateRect`, `BeginPaint`, `EndPaint`, `GetDC`, `ReleaseDC`, `SetTimer`, `KillTimer`.
- **System Metrics**: `GetSystemMetrics` returning deterministic constants: `SM_CXSCREEN = 1920`, `SM_CYSCREEN = 1080`, 96 DPI.

### 3.2 GDI32 — Surface & Text Abstractions
- **Lifecycle & Device Contexts**: `CreateCompatibleDC`, `DeleteDC`, `CreateCompatibleBitmap`, `CreateDIBSection`, `DeleteObject`, `SelectObject`, `GetStockObject`.
- **Primitives**: `CreateSolidBrush`, `CreatePen`, `Rectangle`, `Ellipse`, `RoundRect`, `MoveToEx`, `LineTo`, `SetPixel`, `GetPixel`, `FillRect`, `FrameRect`, `PatBlt`, `SetROP2`.
- **Text Subsystem**:
  - `CreateFontA`, `CreateFontIndirectA`, `SetTextColor`, `GetTextColor`, `SetBkColor`, `GetBkColor`, `SetBkMode`, `GetBkMode`.
  - `GetTextExtentPoint32A`, `GetTextExtentPoint32W`, `GetTextMetricsA`: Computes bounding boxes via static embedded 96 DPI font-width table without rasterizing.
  - `TextOutA`, `TextOutW`, `ExtTextOutA`, `DrawTextA`: Emits `0x07 FRAME_TEXT` structured commands.
- **Blitting & Buffers**: `BitBlt`, `StretchBlt` targeting internal 32-bit BGRA byte slices and recording dirty rectangles.

---

## 4. Wire Protocol Specification (Runtime ↔ Client)

### 4.1 Server to Client (Binary Frames)

Binary frames follow: `type: u8 | len: u32 (LE) | payload`.

| Opcode | Identifier | Layout / Payload | Description |
| :---: | :--- | :--- | :--- |
| `0x02` | `FRAME_BITMAP_RECT` | `x:u16, y:u16, w:u16, h:u16, raw_bytes:[u8]` | Atomic dirty rectangle bitmap in BGRA. |
| `0x03` | `FRAME_FORM` | Serialized JSON metadata of virtual controls. | Structural hierarchy for hit-testing & a11y. |
| `0x04` | `FRAME_DIALOG` | `style:u32, title_len:u16, title:str, msg:str` | Trigger client-rendered native modal dialog (`MsgBox`/`InputBox`). |
| `0x05` | `FRAME_CURSOR` | `cursor_id:u16` | Pointer style updates (arrow, hourglass, caret). |
| `0x06` | `FRAME_CLIPBOARD` | `text_len:u32, text:str` | Bidirectional clipboard synchronization. |
| `0x07` | `FRAME_TEXT` | `x:u16, y:u16, fg:u32, bg:u32, font_id:u8, text:str` | Delegated text rendering command for client Canvas. |

### 4.2 Client to Server (Text JSON Frames)
- `{"event": "mouse.move", "x": 100, "y": 200}` $\to$ maps to `WM_MOUSEMOVE`
- `{"event": "mouse.down", "x": 100, "y": 200, "btn": 1}` $\to$ maps to `WM_LBUTTONDOWN`
- `{"event": "mouse.up", "x": 100, "y": 200, "btn": 1}` $\to$ maps to `WM_LBUTTONUP`
- `{"event": "key.down", "code": 13, "mods": 0}` $\to$ maps to `WM_KEYDOWN`
- `{"event": "key.up", "code": 13, "mods": 0}` $\to$ maps to `WM_KEYUP`
- `{"event": "char", "char": 65}` $\to$ maps to `WM_CHAR`
- `{"event": "resize", "w": 1024, "h": 768}` $\to$ maps to `WM_SIZE`
- `{"event": "dialog.result", "button": 1}` $\to$ unblocks modal `rtcMsgBox` (`vbOK`, `vbCancel`, etc.)

---

## 5. Mathematical Versioning Model (Roadmap to `v1.0.0`)

Version calculation strictly models weighted completion across the three core architectural milestones toward `v1.0.0`:

$$\text{Total Progress}(p_1, p_2, p_3) = (0.45 \times p_1) + (0.35 \times p_2) + (0.20 \times p_3)$$

With **Milestone 1** at $p_1 = 1.00$ (`v0.1.11`, 11/11 phases) and **Milestone 2** at $p_2 = 1.00$ (`v0.2.17`, 17/17 phases):

$$\text{Baseline at } \texttt{v0.2.17} = (0.45 \times 1.00) + (0.35 \times 1.00) + (0.20 \times 0.00) = 0.45 + 0.35 = \mathbf{0.800} \text{ (80.0\%)}$$

**Milestone 3** ($20\%$ weight) is decomposed into **10 sequential phases** ($k \in \{1..10\}$, each contributing $\Delta p_3 = 0.10 \implies \mathbf{+2.0\%}$ total progress):

$$\text{Progress}(3.k) = 80.0\% + (k \times 2.0\%) \quad \text{for } k = 1, 2, \dots, 10$$

| Phase | SemVer Tag | M3 Completion ($p_3$) | Overall Completion | Architectural Layer / Deliverable |
| :--- | :---: | :---: | :---: | :--- |
| **Baseline (M1+M2)** | **`v0.2.17` (Current)** | **0%** | **80.0%** | 100% Core Language + 100% COM/ActiveX + Native x86 & P-Code VMs (59/59 binaries) |
| **Phase 3.1** | `v0.3.1` | 10% | 82.0% | `USER32` Handle Table, `VirtualWindowTree` & `Condvar` Message Pump |
| **Phase 3.2** | `v0.3.2` | 20% | 84.0% | `GDI32` 32-Bit BGRA `VirtualSurface`, Primitives, Pens/Brushes & `DirtyRectTracker` |
| **Phase 3.3** | `v0.3.3` | 30% | 86.0% | Delegated Text Pipeline (`0x07 FRAME_TEXT`) & Static 96 DPI Font Metrics Engine |
| **Phase 3.4** | `v0.3.4` | 40% | 88.0% | PE32 `.rsrc` & `GUITable` Form Deserializer (`ThunderRT6FormDC`) & Twips-to-Pixels |
| **Phase 3.5** | `v0.3.5` | 50% | 90.0% | `ThunderRT6*` Control COM Vtables & Event Sink Dispatch (`X86Cpu` + `PCodeVm`) |
| **Phase 3.6** | `v0.3.6` | 60% | 92.0% | Atomic Wire Protocol Codec (`0x02`..`0x07`) & Hand-Rolled RFC 6455 WebSocket Server |
| **Phase 3.7** | `v0.3.7` | 70% | 94.0% | `wasm32-unknown-unknown` Canvas Client Engine & Minimal JS Bridge |
| **Phase 3.8** | `v0.3.8` | 80% | 96.0% | Integrated `vb6-run --gui-web` Runner & End-to-End Interactive Form Lifecycle |
| **Phase 3.9** | `v0.3.9` | 90% | 98.0% | `classicbasic-session` Multi-Tenant Session Broker & Container Reverse Proxy |
| **Phase 3.10 (GA)** | **`v1.0.0`** | **100%** | **100.0%** | `SandboxProvider` (`--registry=local` / `--registry=remote`) & Production GA |

---

## 6. Milestone 3 Detailed Execution Phases

### Phase 3.1 (`v0.3.1`) — Win32 Handle Table, Virtual Window Tree & Condvar Message Pump (`USER32` Core)
- **Foundations**:
  - Implement 32-bit handle table (`crates/cb-win32/src/handles.rs`) allocating non-zero 32-bit handles (`HWND`, `HDC`, `HGDIOBJ`, `HMENU`, `HCURSOR`) safe for 32-bit `X86Cpu` GPRs and stack frames.
  - Implement `VirtualWindowTree` (`crates/cb-win32/src/user32.rs`) storing hierarchical window nodes (`hwnd`, `parent`, `children`, `class_name`, `caption`, `style`, `ex_style`, `window_rect`, `client_rect`, `visible`, `enabled`, `wndproc`, `user_data`).
  - Implement `VirtualMessageQueue` (`crates/cb-win32/src/user32.rs`) using `Mutex<VecDeque<Win32Msg>>` + `Condvar`.
- **Win32 Exports & x86 Shims**:
  - `RegisterClassA/W`, `UnregisterClassA`, `CreateWindowExA/W`, `DestroyWindow`, `ShowWindow`, `UpdateWindow`, `MoveWindow`, `SetWindowPos`, `SetWindowTextA`, `GetWindowTextA`, `GetWindowTextLengthA`, `GetClientRect`, `GetWindowRect`, `IsWindow`, `IsWindowVisible`, `EnableWindow`, `SetFocus`, `GetFocus`, `GetSystemMetrics`.
  - `GetMessageA/W` (blocking on `Condvar`), `PeekMessageA/W` (non-blocking), `PostMessageA`, `SendMessageA`, `PostQuitMessage`, `TranslateMessage`, `DispatchMessageA`, `DefWindowProcA`.
- **Verification**: `tests/win32_gui_gdi/virtual_windowing.rs` + `App35-win32-user32-windowing` (Native & P-Code).

### Phase 3.2 (`v0.3.2`) — GDI32 Virtual Surface, Primitives, Object Table & Dirty-Rect Tracker (`GDI32` Core)
- **Foundations**:
  - Implement 32-bit BGRA/RGBA framebuffer (`Surface32` in `crates/cb-win32/src/gdi32.rs`) and device context state (`VirtualDC`: selected `HPEN`, `HBRUSH`, `HBITMAP`, `HFONT`, current pen position `(cx, cy)`, `rop2`, `bk_color`, `bk_mode`, `text_color`).
  - Implement `DirtyRectTracker` coalescing modified `(x, y, w, h)` regions on every pixel/primitive write.
- **Win32 Exports & x86 Shims**:
  - `GetDC`, `ReleaseDC`, `BeginPaint`, `EndPaint`, `InvalidateRect`, `CreateCompatibleDC`, `DeleteDC`, `CreateCompatibleBitmap`, `CreateDIBSection`, `SelectObject`, `DeleteObject`, `GetStockObject`.
  - `CreatePen`, `CreateSolidBrush`, `SetPixel`, `GetPixel`, `MoveToEx`, `LineTo` (Bresenham line rasterizer), `Rectangle`, `FillRect`, `FrameRect`, `Ellipse`, `RoundRect`, `PatBlt`, `BitBlt`, `StretchBlt`, `SetROP2`.
- **Verification**: `tests/win32_gui_gdi/gdi_surface_primitives.rs` + `App36-win32-gdi32-surface`.

### Phase 3.3 (`v0.3.3`) — Delegated Text Pipeline (`0x07 FRAME_TEXT`) & Static 96 DPI Font Metrics Engine
- **Foundations**:
  - Build static 96 DPI glyph width and metric tables (`crates/cb-win32/src/font_metrics.rs`) for `MS Sans Serif`, `Tahoma`, `Arial`, and `Courier New` (8pt–24pt, Regular/Bold).
  - Record structured `TextCommand` entries (`x, y, fg, bg, bk_mode, font_id, clip_rect, text`) without rasterizing pixels into the surface.
- **Win32 Exports & x86 Shims**:
  - `CreateFontA`, `CreateFontIndirectA`, `SetTextColor`, `GetTextColor`, `SetBkColor`, `GetBkColor`, `SetBkMode`, `GetBkMode`.
  - `GetTextExtentPoint32A/W`, `GetTextMetricsA/W`, `TextOutA/W`, `ExtTextOutA/W`, `DrawTextA/W`.
- **Verification**: `tests/win32_gui_gdi/text_metrics_pipeline.rs` + `App37-win32-text-metrics`.

### Phase 3.4 (`v0.3.4`) — PE32 `.rsrc` & `GUITable` Deserializer (`ThunderRT6FormDC`) & Twips-to-Pixels Engine
- **Foundations**:
  - Implement PE32 `.rsrc` resource directory traversal and `VBHeader.lp_gui_table` (`GUITable` -> `FormDescriptor`) binary stream reader (`crates/msvbvm60/src/forms/deserializer.rs`).
  - Implement exact Twips-to-Pixels conversion ($\lfloor \text{twips} / 15 \rfloor$) and instantiate `VirtualWindowTree` nodes for `ThunderRT6FormDC` and child controls (`CommandButton`, `TextBox`, `Label`, `Frame`, `CheckBox`, `OptionButton`, `ListBox`, `ComboBox`, `Timer`, `PictureBox`).
- **Verification**: `tests/win32_gui_gdi/form_deserializer.rs` + `App38-vb6-form-deserializer`.

### Phase 3.5 (`v0.3.5`) — Thunder Control Runtime, Property COM Vtables & Event Sink Dispatch
- **Foundations**:
  - Implement COM vtables and property dispatch (`Caption`, `Text`, `Value`, `Visible`, `Enabled`, `BackColor`, `ForeColor`, `Left`, `Top`, `Width`, `Height`) on `Form` and `ThunderRT6*` controls (`crates/msvbvm60/src/forms/controls.rs`).
  - Resolve event sink handlers from `ObjectTable -> ObjectInfo -> ControlTable` (`crates/msvbvm60/src/forms/events.rs`) and dispatch `Form_Load`, `Form_Unload`, `Command_Click`, `Text_Change`, `Check_Click`, `Timer_Timer` into both `cb_x86::X86Cpu` and `PCodeVm`.
  - Wire `rtcMsgBox`, `rtcInputBox`, and `rtcDoEvents` to the virtual windowing/dialog queue.
- **Verification**: `tests/win32_gui_gdi/control_event_sinks.rs` + `App39-vb6-form-events`.

### Phase 3.6 (`v0.3.6`) — Atomic Wire Protocol Codec (`0x02`..`0x07`) & Hand-Rolled RFC 6455 WebSocket Server
- **Foundations**:
  - Implement clean-room SHA-1 + Base64 + RFC 6455 WebSocket server (`crates/cb-net/src/websocket.rs`) over `std::net::TcpListener`.
  - Implement binary frame serializer (`0x02 FRAME_BITMAP_RECT`, `0x03 FRAME_FORM`, `0x04 FRAME_DIALOG`, `0x05 FRAME_CURSOR`, `0x06 FRAME_CLIPBOARD`, `0x07 FRAME_TEXT` in `crates/cb-net/src/protocol.rs`).
  - Implement client JSON input parser (`mouse.*`, `key.*`, `char`, `resize`, `dialog.result`) pushing `Win32Msg` into `VirtualMessageQueue` and notifying `Condvar`.
- **Verification**: `tests/win32_gui_gdi/websocket_wire_protocol.rs` + `App40-gui-websocket-stream`.

### Phase 3.7 (`v0.3.7`) — WebAssembly Canvas Client (`cb-client`) & Zero-Dependency JS Shim
- **Foundations**:
  - Implement `crates/cb-client/` targeting `wasm32-unknown-unknown` ($< 100\text{ KB}$ `.wasm`) with zero external dependencies.
  - Decode `0x02 FRAME_BITMAP_RECT` (BGRA $\to$ RGBA linear memory conversion) and `0x07 FRAME_TEXT`, driving HTML5 Canvas `putImageData` and `fillText` via a minimal $\le 200$-line JS shim.
  - Capture browser pointer/keyboard/resize events and transmit JSON input frames back over WebSocket.
- **Verification**: `tests/win32_gui_gdi/wasm_client_codec.rs`.

### Phase 3.8 (`v0.3.8`) — Integrated `vb6-run --gui-web` Runner & End-to-End Interactive Form Lifecycle
- **Foundations**:
  - Integrate HTTP static asset server + `/ws` upgrade + `ThunRTMain` Form startup + `Condvar` message loop + dirty-rect/text delta streaming into `crates/vb6-run/src/main.rs` (`--gui-web`).
- **Verification**: `tests/win32_gui_gdi/e2e_gui_web_runner.rs` + `App41-gui-calculator-e2e`.

### Phase 3.9 (`v0.3.9`) — Multi-Tenant Session Broker (`cb-session`) & Container Reverse Proxy
- **Foundations**:
  - Implement `crates/cb-session/` orchestrator spawning isolated `vb6-run --gui-web` container/process instances per authenticated WebSocket connection and proxying frames with zero cross-tenant state leakage.
- **Verification**: `tests/win32_gui_gdi/session_broker_isolation.rs`.

### Phase 3.10 (`v1.0.0`) — Sandboxed System Providers (`local` & `remote`) & General Availability (`v1.0.0`)
- **Foundations**:
  - Implement `SandboxProvider` trait in `crates/cb-vfs/src/fs.rs` supporting `--registry=local` and `--registry=remote:<endpoint>` over hand-rolled JSON-RPC.
  - Complete end-to-end production validation and tag `v1.0.0`.
- **Verification**: `tests/win32_gui_gdi/sandbox_providers_ga.rs` + `App42-enterprise-sandbox-ga`.
