# Milestone 3 — Win32 GUI via Web-Streamed Canvas (Detailed Design Specification)

## Status

- **Phase**: Draft — Detailed Design (Architecture Revision 2)
- **Engine**: ClassicBasic
- **Target Version**: `0.70.0` (Milestone 1 at 100% + Milestone 2 at 80% $\to$ ~70% overall completion toward `1.0.0`)
- **Runtime Target**: `x86_64-alpine-linux-musl`
- **Client Target**: `wasm32-unknown-unknown`
- **Hard Constraints**: Clean-room implementation, zero new third-party dependencies in `Cargo.toml`, containerization-first, no Wine, no X11/Wayland/Xvfb, no React/Node in runtime path.

---

## 1. Architectural Adjustments & Critical Directives

### 1.1 Text Rendering Pipeline (Server-Client Split)
- **Design Rule**: The server-side GDI engine (`classicbasic-gdi`) **never** rasterizes glyphs into the BGRA framebuffer.
- **Server Responsibility**: Maintains DC text state (`SetTextColor`, `SetBkColor`, `SetBkMode`, current font metadata). Intercepts `TextOutA`/`TextOutW`/`ExtTextOutA` and emits a structured binary frame (`0x07 FRAME_TEXT`). Satisfies `GetTextExtentPoint32A` using an embedded, static 96 DPI font metric lookup table (MS Sans Serif / Tahoma canonical widths) so VB6 layout logic executes deterministically.
- **Client Responsibility**: The WASM client consumes `FRAME_TEXT` and dispatches drawing directly to the HTML5 Canvas via `ctx.fillText()` / `ctx.strokeText()`, leveraging client-side system fonts.

### 1.2 Coordinate System (Twips to Pixels)
- **Design Rule**: All internal VB6 form definitions (`.rsrc`, `ThunderRT6FormDC`, control properties) store dimensions in **Twips** ($1 \text{ pixel} = 15 \text{ Twips}$ at 96 DPI standard).
- **Conversion Equation**:
  $$\text{pixels} = \left\lfloor \frac{\text{twips}}{15} \right\rfloor, \quad \text{twips} = \text{pixels} \times 15$$
- `classicbasic-forms` converts all form/control bounds from Twips to physical virtual pixels prior to inserting nodes into `VirtualWindowTree`. APIs querying coordinates (`GetClientRect`, `GetWindowRect`) report virtual pixels.

### 1.3 Atomic Frame Protocol (Elimination of Tearing)
- Merged dirty region notifications and raw pixel payloads into a single atomic frame type (`0x02 FRAME_BITMAP_RECT`), removing race conditions and multi-packet frame tearing.

### 1.4 Non-Blocking Message Loop & Thread Synchronization
- The `VirtualMessageQueue` in `classicbasic-gdi` uses a thread-safe MPSC queue synchronized via `std::sync::Condvar`.
- `GetMessageA` blocks the UI execution thread on the `Condvar`. When WebSocket input frames (`mouse`, `key`, `resize`) arrive via `classicbasic-net`, the network thread pushes a `Win32Msg` and signals the condition variable, waking `GetMessageA` without CPU spin-locking.
- `PeekMessageA` inspects the queue non-blockingly and returns immediately.

### 1.5 Control Event Sink & Callback Dispatch
- `classicbasic-forms` binds deserialized controls to their respective VB6 event handlers compiled in `.text`.
- The parser inspects `VBHeader -> ObjectTable -> ControlTable` to resolve event sink offsets (e.g., `_Click`, `_Change`).
- When a `VirtualWindow` receives `WM_LBUTTONUP`, its window procedure maps the control ID to the event offset and invokes the compiled handler via `ThunRTMain` dispatch helpers.

---

## 2. Subsystem Decomposition

| Crate / Component | Kind | Role |
| :--- | :--- | :--- |
| `classicbasic-runtime` (`msvbvm60`) | cdylib + rlib | VB6 VM ABI, BSTR/VARIANT/SAFEARRAY, `rtc*`, `__vba*`, COM, Registry, PE loader. |
| `classicbasic-gdi` | rlib | Clean-room USER32 + GDI32: Window tree, DCs, pens/brushes, font metrics, atomic dirty blits. |
| `classicbasic-forms` | rlib | `.rsrc` deserializer (`ThunderRT6FormDC`), Twips-to-Pixels converter, COM control event wiring. |
| `classicbasic-net` | rlib | RFC 6455 WebSocket server, HTTP handshake, session frame codec, static WASM asset server. |
| `classicbasic-gui` | bin | Entrypoint for `vb6-run --gui-web`. Merges runtime + gdi + forms + net into one runner. |
| `classicbasic-session` | bin | Multi-tenant broker: container provisioning, reverse-proxy, sandbox volume orchestration. |
| `classicbasic-client` | wasm32-unknown-unknown | Canvas renderer, WebSocket client, DOM event bridge. Compiled to static `.wasm`. |

---

## 3. Win32 API Surface Specifications

### 3.1 USER32 — Virtual Windowing & Synchronization
- **Lifecycle**: `RegisterClassA`, `RegisterClassW`, `UnregisterClassA`, `CreateWindowExA`, `CreateWindowExW`, `DestroyWindow`, `ShowWindow`, `UpdateWindow`.
- **Message Pipeline**:
  - `GetMessageA`: Thread-blocking read from virtual queue backed by `Condvar`.
  - `PeekMessageA`: Non-blocking read.
  - `TranslateMessage`, `DispatchMessageA`, `DefWindowProcA`, `PostQuitMessage`.
  - `SendMessageA`, `PostMessageA`.
- **Geometry & Painting**: `SetWindowTextA`, `GetWindowTextA`, `GetClientRect`, `GetWindowRect`, `InvalidateRect`, `BeginPaint`, `EndPaint`, `GetDC`, `ReleaseDC`.
- **System Metrics**: `GetSystemMetrics` returning static constants: `SM_CXSCREEN = 1920`, `SM_CYSCREEN = 1080`, 96 DPI.

### 3.2 GDI32 — Surface & Text Abstractions
- **Lifecycle & Device Contexts**: `CreateCompatibleDC`, `DeleteDC`, `CreateCompatibleBitmap`, `DeleteObject`, `SelectObject`.
- **Primitives**: `CreateSolidBrush`, `CreatePen`, `Rectangle`, `Ellipse`, `RoundRect`, `MoveToEx`, `LineTo`, `SetPixel`, `GetPixel`, `FillRect`, `FrameRect`, `SetROP2`.
- **Text Subsystem**:
  - `SetTextColor`, `SetBkColor`, `SetBkMode`.
  - `GetTextExtentPoint32A`: Computes bounding boxes via static embedded font-width table without rasterizing.
  - `TextOutA`, `TextOutW`, `ExtTextOutA`: Generates `0x07 FRAME_TEXT` structured messages.
- **Blitting & Buffers**: `BitBlt`, `StretchBlt` targeting internal 32-bit BGRA byte slices.

---

## 4. Wire Protocol Specification (Runtime ↔ Client)

### 4.1 Server to Client (Binary Frames)

Binary frames follow: `type: u8 | len: u32 (LE) | payload`.

| Opcode | Identifier | Layout / Payload | Description |
| :---: | :--- | :--- | :--- |
| `0x02` | `FRAME_BITMAP_RECT` | `x:u16, y:u16, w:u16, h:u16, raw_bytes:[u8]` | Atomic dirty rectangle bitmap in BGRA. |
| `0x03` | `FRAME_FORM` | Serialized JSON-like metadata of virtual controls. | Structural hierarchy for hit-testing & a11y. |
| `0x04` | `FRAME_DIALOG` | `style:u32, title_len:u16, title:str, msg:str` | Trigger client-rendered native modal dialog. |
| `0x05` | `FRAME_CURSOR` | `cursor_id:u16` | Pointer style updates (arrow, hourglass, caret). |
| `0x06` | `FRAME_CLIPBOARD` | `text_len:u32, text:str` | Bidirectional clipboard synchronization. |
| `0x07` | `FRAME_TEXT` | `x:u16, y:u16, fg:u32, bg:u32, font_id:u8, text:str` | Delegated text rendering command for client Canvas. |

### 4.2 Client to Server (Text JSON Frames)
- `{"event": "mouse.move", "x": 100, "y": 200}` $\to$ maps to `WM_MOUSEMOVE`
- `{"event": "mouse.down", "x": 100, "y": 200, "btn": 1}` $\to$ maps to `WM_LBUTTONDOWN`
- `{"event": "mouse.up", "x": 100, "y": 200, "btn": 1}` $\to$ maps to `WM_LBUTTONUP`
- `{"event": "key.down", "code": 13, "mods": 0}` $\to$ maps to `WM_KEYDOWN`
- `{"event": "char", "char": 65}` $\to$ maps to `WM_CHAR`
- `{"event": "resize", "w": 1024, "h": 768}` $\to$ maps to `WM_SIZE`

---

## 5. Versioning Model (Roadmap to 1.0.0)

Version calculation reflects progress across the three core architecture milestones toward production release (`1.0.0`):

$$\begin{aligned} \text{Total Progress} &= (\text{M1 Weight} \times 1.00) + (\text{M2 Weight} \times 0.80) + (\text{M3 Weight} \times 0.00) \\ &= (0.45 \times 1.00) + (0.35 \times 0.80) + (0.20 \times 0.00) \\ &= 0.45 + 0.28 + 0.00 = 0.73 \implies \mathbf{0.70.0} \text{ (SemVer)} \end{aligned}$$

| Milestone | Scope / Deliverable | Weight | Status | Version Range |
| :--- | :--- | :---: | :---: | :---: |
| **Milestone 1** | Console, CLI & Core Language (PE32, Arrays, File I/O, Primitives) | 45% | **100% Complete** | `0.1.0` $\to$ `0.45.0` |
| **Milestone 2** | ActiveX & COM (Registry VFS, TypeLib, IDispatch, Shims) | 35% | **80% Complete** | `0.45.0` $\to$ **`0.70.0` (Current)** |
| **Milestone 3** | Win32 GUI via Web-Streamed Canvas (Phases 3.1–3.7) | 20% | In Progress | `0.70.0` $\to$ `1.0.0` |

---

## 6. Milestone 3 Execution Phases

Execution is structured into 7 sequential phases:

### Phase 3.1 — Virtual Windowing & Synchronization Core
- Implement `VirtualWindowTree`, `VirtualDC`, and the thread-safe `VirtualMessageQueue` backed by `std::sync::Condvar`.
- Implement `USER32` window lifecycle: `RegisterClassA`, `CreateWindowExA`, `DestroyWindow`, `ShowWindow`, `UpdateWindow`.
- Deliver non-blocking `PeekMessageA` and blocking `GetMessageA` synchronized with external input notifications.
- **Verification**: Headless unit test verifying window insertion, hierarchy traversal, and message queue unblocking without busy-waiting.

### Phase 3.2 — GDI Drawing Surface & Font Metrics Engine
- Implement 32-bit virtual BGRA memory surface for GDI rasterization (`classicbasic-gdi`).
- Implement basic geometric drawing: `Rectangle`, `Ellipse`, `LineTo`, `FillRect`, `FrameRect`, pens, and solid brushes.
- Build the static 96 DPI font metric lookup table satisfying `GetTextExtentPoint32A`.
- Implement `TextOutA`/`ExtTextOutA` interception converting text draw calls into structured metadata.
- **Verification**: Byte-level assertion of rendered BGRA rectangles and verification of exact character width calculation for standard MS Sans Serif strings.

### Phase 3.3 — Form Deserializer (`.rsrc`) & Control Event Wiring
- Extend PE32 loader to parse the `.rsrc` tree and locate `ThunderRT6FormDC` resource structures.
- Implement Twips-to-Pixels coordinate normalizer ($15:1$ ratio).
- Build deserializer instantiating virtual control nodes (`CommandButton`, `TextBox`, `Label`, `Frame`).
- Wire control interaction events: map `WM_LBUTTONUP` to the control's compiled event handler offset in `ObjectTable`.
- **Verification**: Parse a compiled VB6 Form executable and assert that all controls are instantiated with correct pixel coordinates, z-order, and callback pointers.

### Phase 3.4 — Atomic Wire Protocol & WebSocket Engine
- Implement clean-room RFC 6455 WebSocket framing, server handshake, and connection upgrade in `classicbasic-net` (no external network crates).
- Implement binary serialization for atomic `0x02 FRAME_BITMAP_RECT` and `0x07 FRAME_TEXT`.
- Implement JSON-like deserializer for client incoming text frames (`mouse`, `key`, `resize`).
- **Verification**: Automated TCP test validating WebSocket handshake completion, binary frame round-trip, and input message injection into `VirtualMessageQueue`.

### Phase 3.5 — WASM Canvas Client Engine
- Build `classicbasic-client` targeting `wasm32-unknown-unknown` without `wasm-bindgen` or `web-sys`.
- Provide the minimal JS runtime shim (`~200 lines`) handling WebSocket connection and Canvas 2D context binding.
- Implement frame render loop: blit `FRAME_BITMAP_RECT` directly via `ctx.putImageData()` and draw `FRAME_TEXT` commands via `ctx.fillText()`.
- Bridge browser DOM events (`mousemove`, `mousedown`, `keydown`) back over the WebSocket as text frames.
- **Verification**: Headless WASM compilation test asserting output binary size $< 100\text{ KB}$ and validating message serialization logic.

### Phase 3.6 — Multi-Tenant Session Broker
- Implement `classicbasic-session` orchestrator handling incoming user WebSocket connections.
- Automate container instantiation running `classicbasic-gui --gui-web` for each authenticated session.
- Establish internal TCP reverse proxy bridging client WebSocket traffic directly to the dedicated container instance.
- Handle session teardown and graceful cleanup on client disconnect.
- **Verification**: Test spawning two concurrent sessions and validating complete process and virtual memory isolation between users.

### Phase 3.7 — Sandboxed System Providers (Local & Remote)
- Formalize `SandboxProvider` trait unifying Registry (`registry.json`), Filesystem (`files/`), COM overrides, and Environment.
- Implement `--registry=local` (isolated file volume inside container).
- Implement `--registry=remote:<endpoint>` (centralized microservice provider over hand-rolled RPC).
- **Verification**: Swap local and remote sandbox providers at startup via CLI flag without runtime code changes.
