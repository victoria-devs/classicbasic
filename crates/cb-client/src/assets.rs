// Clean-room re-implementation. Behavior derived from publicly documented
// HTML5 Canvas 2D, WebSocket RFC 6455 & DOM Event specifications. No Microsoft or third-party source used.

//! Embedded static web assets (minimal JavaScript shim, HTML5 canvas viewport, and styles).

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
        const response = await fetch(wasmUrl);
        const bytes = await response.arrayBuffer();
        const { instance } = await WebAssembly.instantiate(bytes, {});
        this.wasm = instance.exports;
        this.client = this.wasm.cb_client_create(this.canvas.width, this.canvas.height);

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
        const len = buffer.byteLength;
        const ptr = this.wasm.cb_client_alloc(len);
        new Uint8Array(this.wasm.memory.buffer, ptr, len).set(new Uint8Array(buffer));

        const res = this.wasm.cb_client_process_message(this.client, ptr, len);
        this.wasm.cb_client_free(ptr, len);
        if (res !== 0) return;

        // Blit dirty rectangles to HTML5 Canvas
        const rectPtr = this.wasm.cb_client_alloc(16);
        const hasDirty = this.wasm.cb_client_dirty_rect(this.client, rectPtr);
        if (hasDirty) {
            const u32 = new Uint32Array(this.wasm.memory.buffer, rectPtr, 4);
            const [dx, dy, dw, dh] = [u32[0], u32[1], u32[2], u32[3]];
            const fbPtr = this.wasm.cb_client_framebuffer_ptr(this.client);
            const fbLen = this.wasm.cb_client_framebuffer_len(this.client);
            const fullRgba = new Uint8ClampedArray(this.wasm.memory.buffer, fbPtr, fbLen);
            const imgData = new ImageData(fullRgba, this.canvas.width, this.canvas.height);
            this.ctx.putImageData(imgData, 0, 0, dx, dy, dw, dh);
        }
        this.wasm.cb_client_free(rectPtr, 16);

        // Render delegated vector text commands
        const textCount = this.wasm.cb_client_text_command_count(this.client);
        if (textCount > 0) {
            const bufSize = 65536;
            const textBuf = this.wasm.cb_client_alloc(bufSize);
            const textLen = this.wasm.cb_client_drain_text_json(this.client, textBuf, bufSize);
            if (textLen > 0) {
                const jsonStr = new TextDecoder().decode(new Uint8Array(this.wasm.memory.buffer, textBuf, textLen));
                const cmds = JSON.parse(jsonStr);
                for (const cmd of cmds) {
                    this.ctx.font = cmd.font;
                    this.ctx.fillStyle = cmd.color;
                    this.ctx.fillText(cmd.text, cmd.x, cmd.y);
                }
            }
            this.wasm.cb_client_free(textBuf, bufSize);
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
        window.addEventListener('DOMContentLoaded', async () => {
            const client = new ClassicBasicClient('cb-canvas');
            await client.init();
        });
    </script>
</body>
</html>
"#;
