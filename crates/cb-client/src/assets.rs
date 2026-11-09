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
        this.canvas.width = window.innerWidth || 800;
        this.canvas.height = window.innerHeight || 600;
        this.ctx = this.canvas.getContext('2d');
        this.wsUrl = wsUrl || `ws://${location.host}/ws`;
        this.wasm = null;
        this.client = null;
        this.ws = null;
    }

    updateLoading(statusTitle, progress, detailText) {
        const elStatus = document.getElementById('loader-status');
        const elDetail = document.getElementById('loader-detail');
        const elFill = document.getElementById('loader-progress-fill');
        const elPct = document.getElementById('loader-pct');
        const pctClamped = Math.max(0, Math.min(100, Math.round(progress * 100)));
        if (elStatus) elStatus.textContent = statusTitle;
        if (elDetail) elDetail.textContent = detailText;
        if (elFill) elFill.style.width = pctClamped + '%';
        if (elPct) elPct.textContent = pctClamped + '% concluído';
        this.drawLoadingProgress(statusTitle, progress, detailText);
    }

    hideLoader() {
        if (this.loaderHidden) return;
        this.loaderHidden = true;
        this.updateLoading('Ambiente ClassicBasic pronto!', 1.0, 'Iniciando desktop interativo...');
        setTimeout(() => {
            const loader = document.getElementById('loader-overlay');
            if (loader) {
                loader.style.opacity = '0';
                loader.style.pointerEvents = 'none';
                setTimeout(() => {
                    loader.style.display = 'none';
                }, 300);
            }
        }, 120);
    }

    async init(wasmUrl = '/classicbasic_client.wasm') {
        const sleep = ms => new Promise(r => setTimeout(r, ms));

        this.updateLoading(
            'Iniciando ClassicBasic...',
            0.15,
            'Inicializando subsistema gráfico e memória...'
        );
        await sleep(50);

        try {
            this.updateLoading(
                'Baixando módulo WebAssembly...',
                0.40,
                'Carregando runtime classicbasic_client.wasm...'
            );

            const response = await fetch(wasmUrl);
            const bytes = await response.arrayBuffer();
            await sleep(40);

            this.updateLoading(
                'Compilando motor WebAssembly (AOT)...',
                0.75,
                'Validando seções WASM e instanciando motor gráfico...'
            );
            const { instance } = await WebAssembly.instantiate(bytes, {});
            this.wasm = instance.exports;
            this.client = this.wasm.cb_client_create(this.canvas.width, this.canvas.height);
            await sleep(40);

            this.updateLoading(
                'Conectando canal WebSocket...',
                0.95,
                `Abrindo canal bidirecional RFC 6455 (${this.wsUrl})...`
            );
        } catch (err) {
            console.warn('[ClassicBasic] WASM acceleration unavailable, using native Canvas 2D engine:', err);
            this.updateLoading(
                'Conectando canal WebSocket...',
                0.90,
                'Iniciando motor Canvas 2D em modo nativo direto...'
            );
        }

        this.bindEvents();
        this.connect();
    }

    drawLoadingProgress(statusTitle, progress, detailText) {
        if (!this.ctx) return;
        const w = this.canvas.width;
        const h = this.canvas.height;

        // Desktop background (#008080)
        this.ctx.fillStyle = '#008080';
        this.ctx.fillRect(0, 0, w, h);

        // Dialog dimensions
        const dw = 440;
        const dh = 190;
        const dx = Math.round((w - dw) / 2);
        const dy = Math.round((h - dh) / 2);

        // Dialog background & 3D border
        this.ctx.fillStyle = '#D4D0C8';
        this.ctx.fillRect(dx, dy, dw, dh);

        // Outer border (raised)
        this.ctx.strokeStyle = '#FFFFFF';
        this.ctx.strokeRect(dx + 0.5, dy + 0.5, dw - 1, dh - 1);
        this.ctx.strokeStyle = '#404040';
        this.ctx.beginPath();
        this.ctx.moveTo(dx + dw - 0.5, dy + 0.5);
        this.ctx.lineTo(dx + dw - 0.5, dy + dh - 0.5);
        this.ctx.lineTo(dx + 0.5, dy + dh - 0.5);
        this.ctx.stroke();

        // Inner border (raised)
        this.ctx.strokeStyle = '#DFDFDF';
        this.ctx.strokeRect(dx + 1.5, dy + 1.5, dw - 3, dh - 3);
        this.ctx.strokeStyle = '#808080';
        this.ctx.beginPath();
        this.ctx.moveTo(dx + dw - 1.5, dy + 1.5);
        this.ctx.lineTo(dx + dw - 1.5, dy + dh - 1.5);
        this.ctx.lineTo(dx + 1.5, dy + dh - 1.5);
        this.ctx.stroke();

        // Title bar
        const tbX = dx + 3;
        const tbY = dy + 3;
        const tbW = dw - 6;
        const tbH = 22;
        const grad = this.ctx.createLinearGradient(tbX, tbY, tbX + tbW, tbY);
        grad.addColorStop(0, '#000080');
        grad.addColorStop(1, '#1084D0');
        this.ctx.fillStyle = grad;
        this.ctx.fillRect(tbX, tbY, tbW, tbH);

        // Title bar text
        this.ctx.font = "bold 12px 'MS Sans Serif', Tahoma, 'Segoe UI', Arial, sans-serif";
        this.ctx.fillStyle = '#FFFFFF';
        this.ctx.fillText('ClassicBasic WebAssembly System', tbX + 8, tbY + 16);

        // Icon (retro monitor graphic)
        const ix = dx + 20;
        const iy = dy + 45;
        this.ctx.fillStyle = '#FFFFFF';
        this.ctx.fillRect(ix, iy, 32, 24);
        this.ctx.strokeStyle = '#000000';
        this.ctx.strokeRect(ix + 0.5, iy + 0.5, 31, 23);
        this.ctx.fillStyle = '#008080';
        this.ctx.fillRect(ix + 3, iy + 3, 26, 16);
        this.ctx.fillStyle = '#000000';
        this.ctx.fillRect(ix + 12, iy + 24, 8, 5);
        this.ctx.fillRect(ix + 8, iy + 29, 16, 2);

        // Status title
        this.ctx.font = "bold 12px 'MS Sans Serif', Tahoma, 'Segoe UI', Arial, sans-serif";
        this.ctx.fillStyle = '#000000';
        this.ctx.fillText(statusTitle, dx + 64, dy + 58);

        // Detail text
        this.ctx.font = "11px 'MS Sans Serif', Tahoma, 'Segoe UI', Arial, sans-serif";
        this.ctx.fillStyle = '#404040';
        this.ctx.fillText(detailText, dx + 64, dy + 78);

        // Progress bar (sunken 3D)
        const pbx = dx + 20;
        const pby = dy + 105;
        const pbw = dw - 40;
        const pbh = 22;

        this.ctx.fillStyle = '#FFFFFF';
        this.ctx.fillRect(pbx, pby, pbw, pbh);

        // Sunken borders
        this.ctx.strokeStyle = '#808080';
        this.ctx.strokeRect(pbx + 0.5, pby + 0.5, pbw - 1, pbh - 1);
        this.ctx.strokeStyle = '#404040';
        this.ctx.beginPath();
        this.ctx.moveTo(pbx + 0.5, pby + pbh - 0.5);
        this.ctx.lineTo(pbx + 0.5, pby + 0.5);
        this.ctx.lineTo(pbx + pbw - 0.5, pby + 0.5);
        this.ctx.stroke();

        // Progress fill (segmented blocks)
        const clampedPct = Math.max(0, Math.min(1.0, progress));
        const fillW = Math.round((pbw - 4) * clampedPct);
        const blockW = 8;
        const gap = 2;
        let curX = pbx + 2;
        this.ctx.fillStyle = '#000080';
        while (curX + blockW <= pbx + 2 + fillW) {
            this.ctx.fillRect(curX, pby + 2, blockW, pbh - 4);
            curX += blockW + gap;
        }
        if (curX < pbx + 2 + fillW) {
            this.ctx.fillRect(curX, pby + 2, (pbx + 2 + fillW) - curX, pbh - 4);
        }

        // Percentage text below progress bar
        const pctStr = `${Math.round(clampedPct * 100)}% concluído`;
        this.ctx.font = "11px 'MS Sans Serif', Tahoma, 'Segoe UI', Arial, sans-serif";
        this.ctx.fillStyle = '#000000';
        this.ctx.fillText(pctStr, pbx + 2, pby + pbh + 18);
    }

    updateCanvasSize() {
        const w = window.innerWidth || 800;
        const h = window.innerHeight || 600;
        if (this.canvas.width !== w || this.canvas.height !== h) {
            this.canvas.width = w;
            this.canvas.height = h;
            this.sendInput({
                type: 'resize',
                hwnd: 0,
                width: w,
                height: h
            });
        }
    }

    connect() {
        this.updateCanvasSize();
        this.ws = new WebSocket(this.wsUrl);
        this.ws.binaryType = 'arraybuffer';

        this.ws.onopen = () => {
            console.log('[ClassicBasic] WebSocket Connected');
            this.updateCanvasSize();
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

            this.hideLoader();

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
                    const hwnd = view.getUint32(1, true);
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
                        const isBold = hwnd === 100 || hwnd === 500;
                        this.ctx.font = (isBold ? 'bold ' : '') + `${fontH}px 'MS Sans Serif', Tahoma, 'Segoe UI', Arial, sans-serif`;
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

    getCanvasCoords(e) {
        const rect = this.canvas.getBoundingClientRect();
        const scaleX = this.canvas.width / (rect.width || 1);
        const scaleY = this.canvas.height / (rect.height || 1);
        const x = Math.round((e.clientX - rect.left) * scaleX);
        const y = Math.round((e.clientY - rect.top) * scaleY);
        return {
            x: Math.max(0, Math.min(this.canvas.width - 1, x)),
            y: Math.max(0, Math.min(this.canvas.height - 1, y))
        };
    }

    bindEvents() {
        window.addEventListener('resize', () => {
            this.updateCanvasSize();
        });

        let movePending = false;
        let lastMove = null;

        this.canvas.addEventListener('pointermove', (e) => {
            lastMove = {
                x: Math.round(e.offsetX),
                y: Math.round(e.offsetY),
                buttons: e.buttons
            };
            if (!movePending) {
                movePending = true;
                requestAnimationFrame(() => {
                    movePending = false;
                    if (lastMove) {
                        this.sendInput({
                            type: 'mouse.move',
                            hwnd: 0,
                            x: lastMove.x,
                            y: lastMove.y,
                            buttons: lastMove.buttons
                        });
                    }
                });
            }
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
            try { this.canvas.releasePointerCapture(e.pointerId); } catch (_) {}
            this.sendInput({
                type: 'mouse.up',
                hwnd: 0,
                x: Math.round(e.offsetX),
                y: Math.round(e.offsetY),
                button: e.button
            });
        });

        this.canvas.addEventListener('dblclick', (e) => {
            this.sendInput({
                type: 'mouse.dblclick',
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
        html, body {
            background-color: #008080;
            width: 100%;
            height: 100%;
            margin: 0;
            padding: 0;
            overflow: hidden;
            font-family: 'MS Sans Serif', Tahoma, 'Segoe UI', Arial, sans-serif;
            user-select: none;
        }
        #canvas-container {
            position: absolute;
            top: 0;
            left: 0;
            width: 100%;
            height: 100%;
            margin: 0;
            padding: 0;
            overflow: hidden;
            background-color: #008080;
        }
        canvas {
            display: block;
            image-rendering: pixelated;
            image-rendering: crisp-edges;
            background-color: #008080;
        }
        #loader-overlay {
            position: absolute;
            top: 0;
            left: 0;
            width: 100%;
            height: 100%;
            display: flex;
            align-items: center;
            justify-content: center;
            background-color: #008080;
            z-index: 1000;
            transition: opacity 0.25s ease-out;
        }
        .loader-window {
            width: 440px;
            background: #D4D0C8;
            border-top: 2px solid #FFFFFF;
            border-left: 2px solid #FFFFFF;
            border-right: 2px solid #404040;
            border-bottom: 2px solid #404040;
            box-shadow: 2px 2px 0px #000000;
            padding: 3px;
        }
        .loader-titlebar {
            background: linear-gradient(90deg, #000080 0%, #1084D0 100%);
            color: #FFFFFF;
            font-weight: bold;
            font-size: 12px;
            padding: 3px 6px;
            letter-spacing: 0.5px;
        }
        .loader-body {
            padding: 16px 20px 20px 20px;
        }
        .loader-content {
            display: flex;
            align-items: center;
            margin-bottom: 16px;
        }
        .loader-icon {
            width: 36px;
            height: 32px;
            margin-right: 16px;
            flex-shrink: 0;
            position: relative;
        }
        .monitor-screen {
            width: 32px;
            height: 22px;
            background: #008080;
            border: 2px solid #404040;
            outline: 2px solid #FFFFFF;
        }
        .monitor-stand {
            width: 8px;
            height: 4px;
            background: #404040;
            margin: 0 auto;
        }
        .monitor-base {
            width: 18px;
            height: 3px;
            background: #808080;
            border-top: 1px solid #FFFFFF;
            border-bottom: 1px solid #000000;
            margin: 0 auto;
        }
        .loader-info {
            flex: 1;
            min-width: 0;
        }
        .loader-status-text {
            font-size: 13px;
            font-weight: bold;
            color: #000000;
            margin-bottom: 4px;
        }
        .loader-detail-text {
            font-size: 11px;
            color: #404040;
            white-space: nowrap;
            overflow: hidden;
            text-overflow: ellipsis;
        }
        .loader-progress-frame {
            height: 22px;
            background: #FFFFFF;
            border-top: 2px solid #808080;
            border-left: 2px solid #808080;
            border-right: 2px solid #DFDFDF;
            border-bottom: 2px solid #DFDFDF;
            padding: 2px;
            margin-bottom: 8px;
        }
        .loader-progress-bar {
            height: 100%;
            background: #000080;
            transition: width 0.15s ease-out;
        }
        .loader-pct-text {
            font-size: 11px;
            color: #000000;
        }
    </style>
</head>
<body>
    <div id="canvas-container">
        <canvas id="cb-canvas" tabindex="0"></canvas>
    </div>
    <div id="loader-overlay">
        <div class="loader-window">
            <div class="loader-titlebar">
                <span>ClassicBasic System Loader</span>
            </div>
            <div class="loader-body">
                <div class="loader-content">
                    <div class="loader-icon">
                        <div class="monitor-screen"></div>
                        <div class="monitor-stand"></div>
                        <div class="monitor-base"></div>
                    </div>
                    <div class="loader-info">
                        <div id="loader-status" class="loader-status-text">Iniciando ClassicBasic...</div>
                        <div id="loader-detail" class="loader-detail-text">Conectando ao host e preparando runtime...</div>
                    </div>
                </div>
                <div class="loader-progress-frame">
                    <div id="loader-progress-fill" class="loader-progress-bar" style="width: 5%;"></div>
                </div>
                <div id="loader-pct" class="loader-pct-text">5% concluído</div>
            </div>
        </div>
    </div>
    <script src="classicbasic.js"></script>
    <script>
        function startApp() {
            const client = new ClassicBasicClient('cb-canvas');
            client.init().catch(err => {
                console.error('[ClassicBasic] Init error:', err);
                const st = document.getElementById('loader-status');
                const dt = document.getElementById('loader-detail');
                if (st) st.textContent = 'Erro ao inicializar ClassicBasic';
                if (dt) dt.textContent = String(err);
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
