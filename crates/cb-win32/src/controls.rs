// Clean-room re-implementation. Behavior derived from publicly documented
// Win32 Button Control (BS_PUSHBUTTON) and Visual Basic 6.0 CommandButton specification.
// No Microsoft or third-party source used.

//! Win32 `BS_PUSHBUTTON` / `ThunderRT6CommandButton` control state machine and 3D rasterizer.
//!
//! Implements complete standard Win32 push button lifecycle:
//! - Mouse capture and hit testing between `WM_LBUTTONDOWN` and `WM_LBUTTONUP`.
//! - Sunken / Pushed visual state tracking (`BST_PUSHED`) with +1px tactile text displacement.
//! - `BN_CLICKED` event dispatch strictly upon `WM_LBUTTONUP` when mouse is released within bounds.
//! - Keyboard activation (`VK_SPACE` press/release and `VK_RETURN`).

/// Standard Win32 Button States (`BST_*`)
pub const BST_UNCHECKED: u32 = 0x0000;
pub const BST_CHECKED: u32 = 0x0001;
pub const BST_INDETERMINATE: u32 = 0x0002;
pub const BST_PUSHED: u32 = 0x0004;
pub const BST_FOCUS: u32 = 0x0008;

/// Button notification codes sent via `WM_COMMAND` in `HIWORD(wParam)`
pub const BN_CLICKED: u16 = 0;
pub const BN_PAINT: u16 = 1;
pub const BN_HILITE: u16 = 2; // BN_PUSHED
pub const BN_UNHILITE: u16 = 3; // BN_UNPUSHED
pub const BN_DISABLE: u16 = 4;
pub const BN_DOUBLECLICKED: u16 = 5;
pub const BN_SETFOCUS: u16 = 6;
pub const BN_KILLFOCUS: u16 = 7;

/// Win32 Push Button control state and rendering coordinator.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Win32ButtonControl {
    pub hwnd: u32,
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub caption: String,
    pub state: u32,
    pub has_capture: bool,
    pub enabled: bool,
    pub visible: bool,
}

impl Win32ButtonControl {
    /// Constructs a new push button control at specified coordinates.
    pub fn new(hwnd: u32, x: i32, y: i32, width: u32, height: u32, caption: &str) -> Self {
        Self {
            hwnd,
            x,
            y,
            width,
            height,
            caption: caption.to_string(),
            state: BST_UNCHECKED,
            has_capture: false,
            enabled: true,
            visible: true,
        }
    }

    /// Returns whether the button is currently in the pushed/depressed state.
    #[inline]
    pub fn is_pushed(&self) -> bool {
        (self.state & BST_PUSHED) != 0
    }

    /// Sets the pushed state, returning true if the visual state actually changed.
    pub fn set_pushed(&mut self, pushed: bool) -> bool {
        let old = self.is_pushed();
        if pushed {
            self.state |= BST_PUSHED;
        } else {
            self.state &= !BST_PUSHED;
        }
        old != self.is_pushed()
    }

    /// Tests if a point (px, py) lies within the button's screen bounding box.
    #[inline]
    pub fn hit_test(&self, px: i32, py: i32) -> bool {
        px >= self.x
            && px < self.x + self.width as i32
            && py >= self.y
            && py < self.y + self.height as i32
    }

    /// Handles `WM_LBUTTONDOWN`: captures mouse and depresses button if within bounds.
    /// Returns true if button state changed and redraw is required.
    pub fn on_lbutton_down(&mut self, px: i32, py: i32) -> bool {
        if !self.enabled || !self.visible {
            return false;
        }
        if self.hit_test(px, py) {
            self.has_capture = true;
            self.set_pushed(true)
        } else {
            false
        }
    }

    /// Handles `WM_MOUSEMOVE`: while mouse is captured, tracking inside/outside updates pushed state.
    /// Returns true if visual pushed state toggled and redraw is required.
    pub fn on_mouse_move(&mut self, px: i32, py: i32) -> bool {
        if !self.has_capture {
            return false;
        }
        let inside = self.hit_test(px, py);
        self.set_pushed(inside)
    }

    /// Handles `WM_LBUTTONUP`: releases capture, restores unpressed state, and evaluates click.
    /// Returns `(needs_redraw, click_fired)`.
    pub fn on_lbutton_up(&mut self, px: i32, py: i32) -> (bool, bool) {
        if !self.has_capture {
            return (false, false);
        }
        self.has_capture = false;
        let was_pushed = self.is_pushed();
        let redraw = self.set_pushed(false) || was_pushed;
        let click_fired = was_pushed && self.hit_test(px, py);
        (redraw, click_fired)
    }

    /// Handles keyboard `WM_KEYDOWN`: spacebar depresses button, Enter immediately clicks.
    /// Returns `(needs_redraw, click_fired)`.
    pub fn on_key_down(&mut self, vk: usize) -> (bool, bool) {
        if !self.enabled || !self.visible {
            return (false, false);
        }
        match vk {
            32 => {
                // VK_SPACE: depress
                let redraw = self.set_pushed(true);
                (redraw, false)
            }
            13 => {
                // VK_RETURN: immediate click
                (false, true)
            }
            _ => (false, false),
        }
    }

    /// Handles keyboard `WM_KEYUP`: releasing spacebar triggers click.
    /// Returns `(needs_redraw, click_fired)`.
    pub fn on_key_up(&mut self, vk: usize) -> (bool, bool) {
        if !self.enabled || !self.visible {
            return (false, false);
        }
        if vk == 32 {
            let was_pushed = self.is_pushed();
            let redraw = self.set_pushed(false);
            (redraw, was_pushed)
        } else {
            (false, false)
        }
    }

    /// Calculates tactile text rendering offset: `(0, 0)` for normal, `(1, 1)` for pushed.
    #[inline]
    pub fn text_offset(&self) -> (i16, i16) {
        if self.is_pushed() {
            (1, 1)
        } else {
            (0, 0)
        }
    }

    /// Renders the button as a 32-bit BGRA rectangular pixel buffer of size `width * height * 4`.
    ///
    /// Implements authentic Win32 2-level 3D beveled borders:
    /// - Normal (Raised):
    ///   - Fill: `COLOR_BTNFACE` (#D4D0C8 / B:200, G:208, R:212)
    ///   - Outer Top/Left: #FFFFFF, Outer Bottom/Right: #000000
    ///   - Inner Top/Left: #DFDFDF, Inner Bottom/Right: #808080
    /// - Pushed (Sunken):
    ///   - Fill: `COLOR_BTNFACE` (#D4D0C8 / B:200, G:208, R:212)
    ///   - Outer Top/Left: #000000 (or #808080), Outer Bottom/Right: #FFFFFF
    ///   - Inner Top/Left: #808080, Inner Bottom/Right: #D4D0C8
    pub fn render_pixels(&self) -> Vec<u8> {
        let w = self.width as usize;
        let h = self.height as usize;
        let mut pixels = vec![0u8; w * h * 4];

        // 1. Fill background with COLOR_BTNFACE (#D4D0C8: B=0xC8, G=0xD0, R=0xD4, A=0xFF)
        for chunk in pixels.as_chunks_mut::<4>().0 {
            chunk[0] = 0xC8;
            chunk[1] = 0xD0;
            chunk[2] = 0xD4;
            chunk[3] = 0xFF;
        }

        let set_px = |buf: &mut [u8], px: usize, py: usize, b: u8, g: u8, r: u8| {
            if px < w && py < h {
                let idx = (py * w + px) * 4;
                buf[idx] = b;
                buf[idx + 1] = g;
                buf[idx + 2] = r;
                buf[idx + 3] = 0xFF;
            }
        };

        if !self.is_pushed() {
            // Unpressed / Raised 3D Button
            // Outer Top/Left: White (#FFFFFF), Outer Bottom/Right: Black (#000000)
            for x in 0..w {
                set_px(&mut pixels, x, 0, 0xFF, 0xFF, 0xFF);
                set_px(&mut pixels, x, h - 1, 0x00, 0x00, 0x00);
            }
            for y in 0..h {
                set_px(&mut pixels, 0, y, 0xFF, 0xFF, 0xFF);
                set_px(&mut pixels, w - 1, y, 0x00, 0x00, 0x00);
            }

            // Inner Top/Left: Light Gray (#DFDFDF), Inner Bottom/Right: Dark Shadow (#808080)
            if w > 2 && h > 2 {
                for x in 1..(w - 1) {
                    set_px(&mut pixels, x, 1, 0xDF, 0xDF, 0xDF);
                    set_px(&mut pixels, x, h - 2, 0x80, 0x80, 0x80);
                }
                for y in 1..(h - 2) {
                    set_px(&mut pixels, 1, y, 0xDF, 0xDF, 0xDF);
                    set_px(&mut pixels, w - 2, y, 0x80, 0x80, 0x80);
                }
            }
        } else {
            // Pressed / Sunken 3D Button
            // Outer Top/Left: Black (#000000), Outer Bottom/Right: White (#FFFFFF)
            for x in 0..w {
                set_px(&mut pixels, x, 0, 0x00, 0x00, 0x00);
                set_px(&mut pixels, x, h - 1, 0xFF, 0xFF, 0xFF);
            }
            for y in 0..h {
                set_px(&mut pixels, 0, y, 0x00, 0x00, 0x00);
                set_px(&mut pixels, w - 1, y, 0xFF, 0xFF, 0xFF);
            }

            // Inner Top/Left: Dark Shadow (#808080), Inner Bottom/Right: Face/Light (#C8D0D4)
            if w > 2 && h > 2 {
                for x in 1..(w - 1) {
                    set_px(&mut pixels, x, 1, 0x80, 0x80, 0x80);
                    set_px(&mut pixels, x, h - 2, 0xC8, 0xD0, 0xD4);
                }
                for y in 1..(h - 2) {
                    set_px(&mut pixels, 1, y, 0x80, 0x80, 0x80);
                    set_px(&mut pixels, w - 2, y, 0xC8, 0xD0, 0xD4);
                }
            }
        }

        pixels
    }
}
