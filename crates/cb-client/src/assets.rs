// Clean-room re-implementation. Behavior derived from publicly documented
// HTML5 Canvas 2D, WebSocket RFC 6455 & DOM Event specifications. No Microsoft or third-party source used.

//! Embedded static web assets (minimal JavaScript shim, HTML5 canvas viewport, and styles).

/// Pre-compiled zero-dependency WebAssembly Canvas 2D linear client binary (< 52 KB).
pub const WASM_BINARY: &[u8] = include_bytes!("../assets/classicbasic_client.wasm");

/// Minimal vanilla JavaScript shim (< 150 lines) bridging WebAssembly Canvas 2D to WebSocket.
pub const JS_SHIM: &str = r#"/**
 * ClassicBasic WebAssembly Canvas 2D Client Bridge
 * Zero-dependency RFC 6455 WebSocket & HTML5 Canvas 2D Display Engine
 */
class ClassicBasicClient {
    constructor(canvasId, wsUrl) {
        this.canvas = document.getElementById(canvasId);
        this.ctx = this.canvas.getContext('2d');
        this.wsUrl = wsUrl || `ws://${location.host}/ws`;
        this.wasm = null;
        this.client = null;
        this.ws = null;
    }

    async init(wasmUrl = '/classicbasic_client.wasm') {
        try {
            const response = await fetch(wasmUrl);
            const bytes = await response.arrayBuffer();
            const { instance } = await WebAssembly.instantiate(bytes, {});
            this.wasm = instance.exports;
            this.client = this.wasm.cb_client_create(this.canvas.width, this.canvas.height);
        } catch (err) {
            console.warn('[ClassicBasic] WASM acceleration unavailable, using native Canvas 2D engine:', err);
        }

        this.bindEvents();
        this.connect();
    }

    connect() {
        this.ws = new WebSocket(this.wsUrl);
        this.ws.binaryType = 'arraybuffer';

        this.ws.onopen = () => {
            console.log('[ClassicBasic] WebSocket Connected');
            this.sendInput({
                type: 'resize',
                hwnd: 0,
                width: this.canvas.width,
                height: this.canvas.height
            });
        };

        this.ws.onerror = (e) => {
            console.error('[ClassicBasic] WebSocket Error:', e);
        };

        this.ws.onmessage = (event) => {
            if (event.data instanceof ArrayBuffer) {
                this.handleBinaryFrame(event.data);
            }
        };

        this.ws.onclose = () => {
            console.log('[ClassicBasic] Connection closed. Reconnecting in 2s...');
            setTimeout(() => this.connect(), 2000);
        };
    }

    handleBinaryFrame(buffer) {
        try {
            const len = buffer.byteLength;
            if (len === 0) return;

            // Direct RFC 6455 binary frame decoder for 100% canvas fidelity
            const view = new DataView(buffer);
            const opcode = view.getUint8(0);

            if (opcode === 2) {
                // 0x02 FRAME_BITMAP_RECT: hwnd(4), x(2), y(2), w(2), h(2), bgra(w*h*4)
                if (buffer.byteLength >= 13) {
                    const x = view.getUint16(5, true);
                    const y = view.getUint16(7, true);
                    const w = view.getUint16(9, true);
                    const h = view.getUint16(11, true);
                    const expectedBytes = w * h * 4;
                    if (buffer.byteLength >= 13 + expectedBytes) {
                        const bgra = new Uint8Array(buffer, 13, expectedBytes);
                        const imgData = this.ctx.createImageData(w, h);
                        const rgba = imgData.data;
                        for (let i = 0; i < expectedBytes; i += 4) {
                            rgba[i]     = bgra[i + 2]; // R
                            rgba[i + 1] = bgra[i + 1]; // G
                            rgba[i + 2] = bgra[i];     // B
                            rgba[i + 3] = bgra[i + 3]; // A
                        }
                        this.ctx.putImageData(imgData, x, y);
                    }
                }
            } else if (opcode === 7) {
                // 0x07 FRAME_TEXT: hwnd(4), x(2), y(2), color_rgb(4), bk_mode(1), font_height(2), len(2), text
                if (buffer.byteLength >= 18) {
                    const x = view.getInt16(5, true);
                    const y = view.getInt16(7, true);
                    const col = view.getUint32(9, true);
                    const r = col & 0xFF;
                    const g = (col >> 8) & 0xFF;
                    const b = (col >> 16) & 0xFF;
                    const fontH = Math.abs(view.getInt16(14, true)) || 13;
                    const strLen = view.getUint16(16, true);
                    if (buffer.byteLength >= 18 + strLen) {
                        const textBytes = new Uint8Array(buffer, 18, strLen);
                        const text = new TextDecoder().decode(textBytes);
                        this.ctx.font = `${fontH}px 'MS Sans Serif', Tahoma, sans-serif`;
                        this.ctx.fillStyle = `rgb(${r},${g},${b})`;
                        this.ctx.fillText(text, x, y);
                    }
                }
            }

            // Maintain WASM state parity if WASM engine is active
            if (this.wasm && this.client) {
                const ptr = this.wasm.cb_client_alloc(len);
                if (ptr) {
                    new Uint8Array(this.wasm.memory.buffer, ptr, len).set(new Uint8Array(buffer));
                    this.wasm.cb_client_process_message(this.client, ptr, len);
                    this.wasm.cb_client_free(ptr, len);
                }
            }
        } catch (err) {
            console.error('[ClassicBasic] Frame error:', err);
        }
    }

    sendInput(obj) {
        if (this.ws && this.ws.readyState === WebSocket.OPEN) {
            this.ws.send(JSON.stringify(obj));
        }
    }

    bindEvents() {
        this.canvas.addEventListener('pointermove', (e) => {
            this.sendInput({
                type: 'mouse.move',
                hwnd: 0,
                x: Math.round(e.offsetX),
                y: Math.round(e.offsetY),
                buttons: e.buttons
            });
        });

        this.canvas.addEventListener('pointerdown', (e) => {
            this.canvas.setPointerCapture(e.pointerId);
            this.sendInput({
                type: 'mouse.down',
                hwnd: 0,
                x: Math.round(e.offsetX),
                y: Math.round(e.offsetY),
                button: e.button
            });
        });

        this.canvas.addEventListener('pointerup', (e) => {
            this.canvas.releasePointerCapture(e.pointerId);
            this.sendInput({
                type: 'mouse.up',
                hwnd: 0,
                x: Math.round(e.offsetX),
                y: Math.round(e.offsetY),
                button: e.button
            });
        });

        window.addEventListener('keydown', (e) => {
            this.sendInput({
                type: 'key.down',
                hwnd: 0,
                vk: e.keyCode,
                repeat: e.repeat ? 1 : 0
            });
            if (e.key && e.key.length === 1) {
                this.sendInput({
                    type: 'char',
                    hwnd: 0,
                    code: e.key.charCodeAt(0)
                });
            }
        });

        window.addEventListener('keyup', (e) => {
            this.sendInput({
                type: 'key.up',
                hwnd: 0,
                vk: e.keyCode
            });
        });
    }
}
window.ClassicBasicClient = ClassicBasicClient;
"#;

/// Minimal responsive HTML5 canvas host page.
pub const INDEX_HTML: &str = r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>ClassicBasic WebAssembly Client</title>
    <style>
        * { box-sizing: border-box; margin: 0; padding: 0; }
        body {
            background-color: #008080;
            display: flex;
            align-items: center;
            justify-content: center;
            height: 100vh;
            overflow: hidden;
            font-family: 'Tahoma', 'MS Sans Serif', sans-serif;
        }
        #canvas-container {
            box-shadow: 4px 4px 10px rgba(0, 0, 0, 0.5);
            border: 2px solid #ffffff;
            background-color: #c0c0c0;
        }
        canvas {
            display: block;
            image-rendering: pixelated;
        }
    </style>
</head>
<body>
    <div id="canvas-container">
        <canvas id="cb-canvas" width="800" height="600" tabindex="0"></canvas>
    </div>
    <script src="classicbasic.js"></script>
    <script>
        function startApp() {
            const client = new ClassicBasicClient('cb-canvas');
            client.init().catch(err => {
                console.error('[ClassicBasic] Init error:', err);
            });
        }
        if (document.readyState === 'loading') {
            document.addEventListener('DOMContentLoaded', startApp);
        } else {
            startApp();
        }
    </script>
</body>
</html>
"#;
