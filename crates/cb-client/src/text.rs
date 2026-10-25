// Clean-room re-implementation. Behavior derived from publicly documented
// HTML5 Canvas 2D text rendering & Win32 GDI font metrics semantics. No Microsoft or third-party source used.

//! Delegated vector text layout and rendering command queue.
//!
//! Captures `0x07 FRAME_TEXT` operations emitted by the server and transforms them
//! into formatted Canvas 2D `ctx.fillText` draw instructions with CSS colors and font specs.

/// Formatted text rendering command for HTML5 Canvas 2D execution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextDrawCommand {
    pub hwnd: u32,
    pub x: i16,
    pub y: i16,
    pub color_rgb: u32,
    pub color_css: String,
    pub font_css: String,
    pub bk_mode: u8, // 1 = TRANSPARENT, 2 = OPAQUE
    pub font_height: i16,
    pub text: String,
}

impl TextDrawCommand {
    /// Constructs a text draw command from raw Win32 parameters, generating CSS representations.
    pub fn new(
        hwnd: u32,
        x: i16,
        y: i16,
        color_rgb: u32,
        bk_mode: u8,
        font_height: i16,
        text: String,
    ) -> Self {
        // Win32 COLORREF format: 0x00BBGGRR
        let r = (color_rgb & 0xFF) as u8;
        let g = ((color_rgb >> 8) & 0xFF) as u8;
        let b = ((color_rgb >> 16) & 0xFF) as u8;
        let color_css = format!("rgb({}, {}, {})", r, g, b);

        let size_px = if font_height != 0 {
            font_height.abs()
        } else {
            13 // Default standard Windows dialog font size
        };
        let font_css = format!("{}px sans-serif", size_px);

        Self {
            hwnd,
            x,
            y,
            color_rgb,
            color_css,
            font_css,
            bk_mode,
            font_height,
            text,
        }
    }
}

/// Queue of pending text draw commands to be flushed by the JavaScript shim.
#[derive(Debug, Default, Clone)]
pub struct TextQueue {
    commands: Vec<TextDrawCommand>,
}

impl TextQueue {
    pub fn new() -> Self {
        Self {
            commands: Vec::new(),
        }
    }

    #[inline]
    pub fn push(&mut self, cmd: TextDrawCommand) {
        self.commands.push(cmd);
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.commands.len()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.commands.is_empty()
    }

    #[inline]
    pub fn peek(&self) -> &[TextDrawCommand] {
        &self.commands
    }

    #[inline]
    pub fn drain(&mut self) -> Vec<TextDrawCommand> {
        std::mem::take(&mut self.commands)
    }

    #[inline]
    pub fn clear(&mut self) {
        self.commands.clear();
    }
}
