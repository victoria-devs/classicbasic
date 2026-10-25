// Clean-room re-implementation. Behavior derived from publicly documented
// VB6 / COM / Win32 ABI semantics. No Microsoft or third-party source used.

//! Win32 `ThunderRT6FormDC` Form Deserializer, Control Hierarchy & Twips-to-Pixels Engine (Milestone 3 Phase 3.4).

use crate::user32::{RegisteredClass, VirtualWindowTree, Hwnd, WS_CHILD, WS_OVERLAPPEDWINDOW, WS_VISIBLE};

/// Standard screen metrics: 1440 twips per logical inch.
pub const TWIPS_PER_INCH: i32 = 1440;

/// Standard display DPI: 96 pixels per logical inch.
pub const DEFAULT_DPI: i32 = 96;

/// Standard conversion ratio: 15 twips per display pixel at 96 DPI (1440 / 96 = 15).
pub const TWIPS_PER_PIXEL: i32 = 15;

/// Converts a dimension in twips to physical display pixels at standard 96 DPI.
#[inline]
pub fn twips_to_pixels(twips: i32) -> i32 {
    twips / TWIPS_PER_PIXEL
}

/// Converts a dimension in display pixels to twips at standard 96 DPI.
#[inline]
pub fn pixels_to_twips(pixels: i32) -> i32 {
    pixels * TWIPS_PER_PIXEL
}

/// Intrinsic Visual Basic 6.0 control types supported by the Thunder engine.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ControlType {
    #[default]
    Form,
    CommandButton,
    TextBox,
    Label,
    CheckBox,
    OptionButton,
    Frame,
    ListBox,
    ComboBox,
    Timer,
    PictureBox,
}

impl ControlType {
    /// Returns the standard Win32 window class name for this Thunder control.
    pub fn class_name(&self) -> &'static str {
        match self {
            ControlType::Form => "ThunderRT6FormDC",
            ControlType::CommandButton => "ThunderRT6CommandButton",
            ControlType::TextBox => "ThunderRT6TextBox",
            ControlType::Label => "ThunderRT6Label",
            ControlType::CheckBox => "ThunderRT6CheckBox",
            ControlType::OptionButton => "ThunderRT6OptionButton",
            ControlType::Frame => "ThunderRT6Frame",
            ControlType::ListBox => "ThunderRT6ListBox",
            ControlType::ComboBox => "ThunderRT6ComboBox",
            ControlType::Timer => "ThunderRT6Timer",
            ControlType::PictureBox => "ThunderRT6PictureBox",
        }
    }

    /// Parses a VB control type name (e.g. `"VB.CommandButton"` or `"CommandButton"`).
    pub fn from_vb_type(name: &str) -> Option<Self> {
        let stripped = name.strip_prefix("VB.").unwrap_or(name);
        match stripped {
            "Form" => Some(ControlType::Form),
            "CommandButton" => Some(ControlType::CommandButton),
            "TextBox" => Some(ControlType::TextBox),
            "Label" => Some(ControlType::Label),
            "CheckBox" => Some(ControlType::CheckBox),
            "OptionButton" => Some(ControlType::OptionButton),
            "Frame" => Some(ControlType::Frame),
            "ListBox" => Some(ControlType::ListBox),
            "ComboBox" => Some(ControlType::ComboBox),
            "Timer" => Some(ControlType::Timer),
            "PictureBox" => Some(ControlType::PictureBox),
            _ => None,
        }
    }
}

/// Metadata describing a child control inside a Visual Basic 6.0 Form.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ControlDescriptor {
    pub name: String,
    pub control_type: ControlType,
    pub caption: String,
    pub text: String,
    pub left: i32,
    pub top: i32,
    pub width: i32,
    pub height: i32,
    pub value: i32,
    pub interval: u32,
    pub enabled: bool,
    pub visible: bool,
    pub tab_index: i32,
}

impl ControlDescriptor {
    pub fn pixel_x(&self) -> i32 {
        twips_to_pixels(self.left)
    }

    pub fn pixel_y(&self) -> i32 {
        twips_to_pixels(self.top)
    }

    pub fn pixel_width(&self) -> i32 {
        twips_to_pixels(self.width)
    }

    pub fn pixel_height(&self) -> i32 {
        twips_to_pixels(self.height)
    }
}

/// Metadata describing a complete Visual Basic 6.0 Form and its hierarchy of controls.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FormDescriptor {
    pub name: String,
    pub caption: String,
    pub left: i32,
    pub top: i32,
    pub width: i32,
    pub height: i32,
    pub client_left: i32,
    pub client_top: i32,
    pub client_width: i32,
    pub client_height: i32,
    pub scale_width: i32,
    pub scale_height: i32,
    pub visible: bool,
    pub enabled: bool,
    pub controls: Vec<ControlDescriptor>,
}

impl FormDescriptor {
    pub fn pixel_x(&self) -> i32 {
        twips_to_pixels(self.left)
    }

    pub fn pixel_y(&self) -> i32 {
        twips_to_pixels(self.top)
    }

    pub fn pixel_width(&self) -> i32 {
        twips_to_pixels(self.width)
    }

    pub fn pixel_height(&self) -> i32 {
        twips_to_pixels(self.height)
    }

    pub fn pixel_client_width(&self) -> i32 {
        twips_to_pixels(self.client_width)
    }

    pub fn pixel_client_height(&self) -> i32 {
        twips_to_pixels(self.client_height)
    }

    pub fn find_control(&self, name: &str) -> Option<&ControlDescriptor> {
        self.controls.iter().find(|c| c.name.eq_ignore_ascii_case(name))
    }

    pub fn find_control_mut(&mut self, name: &str) -> Option<&mut ControlDescriptor> {
        self.controls.iter_mut().find(|c| c.name.eq_ignore_ascii_case(name))
    }

    /// Parses a VB6 form definition from standard `.frm` text format.
    pub fn parse_frm(content: &str) -> Result<FormDescriptor, String> {
        let mut form = FormDescriptor {
            enabled: true,
            visible: false,
            ..Default::default()
        };

        let mut current_control: Option<ControlDescriptor> = None;
        let mut in_form = false;

        for raw_line in content.lines() {
            let line = raw_line.trim();
            if line.is_empty() {
                continue;
            }

            if line.starts_with("Begin VB.Form ") {
                in_form = true;
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 3 {
                    form.name = parts[2].to_string();
                }
                continue;
            }

            if line.starts_with("Begin VB.") {
                // Child control start
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 3 {
                    let ctype_str = parts[1];
                    let cname = parts[2].to_string();
                    if let Some(ctype) = ControlType::from_vb_type(ctype_str) {
                        current_control = Some(ControlDescriptor {
                            name: cname,
                            control_type: ctype,
                            enabled: true,
                            visible: true,
                            ..Default::default()
                        });
                    }
                }
                continue;
            }

            if line == "End" {
                if let Some(ctrl) = current_control.take() {
                    form.controls.push(ctrl);
                } else if in_form {
                    // End of form description
                    break;
                }
                continue;
            }

            // Property assignment: Key = Value
            if let Some((key_raw, val_raw)) = line.split_once('=') {
                let key = key_raw.trim();
                let mut val = val_raw.trim();

                // Strip trailing comment (e.g. `'Checked`)
                if let Some(pos) = val.find('\'') {
                    val = val[..pos].trim();
                }

                // Strip surrounding quotes
                let val_str = if val.starts_with('"') && val.ends_with('"') && val.len() >= 2 {
                    &val[1..val.len() - 1]
                } else {
                    val
                };

                let val_i32 = val_str.parse::<i32>().unwrap_or(0);

                if let Some(ctrl) = current_control.as_mut() {
                    match key {
                        "Caption" => ctrl.caption = val_str.to_string(),
                        "Text" => ctrl.text = val_str.to_string(),
                        "Left" => ctrl.left = val_i32,
                        "Top" => ctrl.top = val_i32,
                        "Width" => ctrl.width = val_i32,
                        "Height" => ctrl.height = val_i32,
                        "Value" => ctrl.value = val_i32,
                        "Interval" => ctrl.interval = val_i32.max(0) as u32,
                        "Enabled" => ctrl.enabled = val_i32 != 0 && !val_str.eq_ignore_ascii_case("false"),
                        "Visible" => ctrl.visible = val_i32 != 0 && !val_str.eq_ignore_ascii_case("false"),
                        "TabIndex" => ctrl.tab_index = val_i32,
                        _ => {}
                    }
                } else if in_form {
                    match key {
                        "Caption" => form.caption = val_str.to_string(),
                        "ClientLeft" => form.client_left = val_i32,
                        "ClientTop" => form.client_top = val_i32,
                        "ClientWidth" => form.client_width = val_i32,
                        "ClientHeight" => form.client_height = val_i32,
                        "ScaleWidth" => form.scale_width = val_i32,
                        "ScaleHeight" => form.scale_height = val_i32,
                        "Left" => form.left = val_i32,
                        "Top" => form.top = val_i32,
                        "Width" => form.width = val_i32,
                        "Height" => form.height = val_i32,
                        "Visible" => form.visible = val_i32 != 0 && !val_str.eq_ignore_ascii_case("false"),
                        "Enabled" => form.enabled = val_i32 != 0 && !val_str.eq_ignore_ascii_case("false"),
                        _ => {}
                    }
                }
            }
        }

        if form.width == 0 && form.client_width != 0 {
            form.width = form.client_width;
        }
        if form.height == 0 && form.client_height != 0 {
            form.height = form.client_height;
        }
        if form.scale_width == 0 && form.client_width != 0 {
            form.scale_width = form.client_width;
        }
        if form.scale_height == 0 && form.client_height != 0 {
            form.scale_height = form.client_height;
        }

        Ok(form)
    }

    /// Instantiates the Form and all child controls into the `VirtualWindowTree`.
    ///
    /// Registers the standard Thunder window classes (`ThunderRT6FormDC`, `ThunderRT6CommandButton`, etc.),
    /// creates the root Form window, and populates its child controls with converted pixel coordinates.
    pub fn instantiate(&self, tree: &mut VirtualWindowTree) -> Hwnd {
        // Register ThunderRT6FormDC class
        tree.register_class(RegisteredClass {
            name: "ThunderRT6FormDC".to_string(),
            style: 0,
            wndproc: 0,
            cb_cls_extra: 0,
            cb_wnd_extra: 0,
            h_instance: 0x0040_0000,
            atom: 0,
        });

        // Register child control classes
        for ctrl_type in [
            ControlType::CommandButton,
            ControlType::TextBox,
            ControlType::Label,
            ControlType::CheckBox,
            ControlType::OptionButton,
            ControlType::Frame,
            ControlType::ListBox,
            ControlType::ComboBox,
            ControlType::Timer,
            ControlType::PictureBox,
        ] {
            tree.register_class(RegisteredClass {
                name: ctrl_type.class_name().to_string(),
                style: 0,
                wndproc: 0,
                cb_cls_extra: 0,
                cb_wnd_extra: 0,
                h_instance: 0x0040_0000,
                atom: 0,
            });
        }

        // Form dimensions in pixels (at 96 DPI: 1 pixel = 15 twips)
        let form_x = self.pixel_x();
        let form_y = self.pixel_y();
        let form_w = (if self.width > 0 { self.pixel_width() } else { self.pixel_client_width() }).max(100);
        let form_h = (if self.height > 0 { self.pixel_height() } else { self.pixel_client_height() }).max(100);

        let hwnd_form = tree.create_window(
            0,
            "ThunderRT6FormDC".to_string(),
            self.caption.clone(),
            WS_OVERLAPPEDWINDOW,
            form_x,
            form_y,
            form_w,
            form_h,
            0, // root window
            0,
            0x0040_0000,
        );

        // Instantiate child controls
        for ctrl in &self.controls {
            let cx = ctrl.pixel_x();
            let cy = ctrl.pixel_y();
            let cw = ctrl.pixel_width().max(10);
            let ch = ctrl.pixel_height().max(10);
            let text = if !ctrl.caption.is_empty() {
                &ctrl.caption
            } else {
                &ctrl.text
            };

            tree.create_window(
                0,
                ctrl.control_type.class_name().to_string(),
                text.to_string(),
                WS_CHILD | WS_VISIBLE,
                cx,
                cy,
                cw,
                ch,
                hwnd_form, // child of form
                0,
                0x0040_0000,
            );
        }

        hwnd_form
    }
}
