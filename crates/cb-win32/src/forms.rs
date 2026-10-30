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
    pub back_color: Option<u32>,
    pub fore_color: Option<u32>,
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

    /// Deserializes a Form and its child controls directly from a compiled VB6 Form Binary Stream (`0x0031CCFF`).
    pub fn parse_binary_stream(blob: &[u8]) -> Result<FormDescriptor, String> {
        if blob.len() < 0x50 || &blob[..4] != b"\xff\xcc1\x00" {
            return Err("Invalid VB6 Form Binary Stream: missing 0x0031CCFF magic".to_string());
        }

        let ff01 = blob
            .windows(2)
            .position(|w| w == b"\xff\x01")
            .ok_or_else(|| "Form property stream delimiter (0xFF 0x01) not found".to_string())?;

        let form_slice = &blob[..ff01];

        // Locate Form Caption marker b"\r\x01"
        let cpos = form_slice
            .windows(2)
            .position(|w| w == b"\r\x01")
            .ok_or_else(|| "Form caption marker (0x0D 0x01) not found".to_string())?;

        let cap_len = u16::from_le_bytes(
            form_slice[cpos + 2..cpos + 4]
                .try_into()
                .map_err(|_| "Invalid caption length")?,
        ) as usize;
        let form_caption = String::from_utf8_lossy(&form_slice[cpos + 4..cpos + 4 + cap_len])
            .trim_matches('\0')
            .to_string();

        // Locate Form Name right before cpos (terminated by null)
        let name_null = form_slice[..cpos]
            .iter()
            .rposition(|&b| b == 0)
            .unwrap_or(cpos);
        let mut name_start = name_null;
        while name_start > 0x40
            && (form_slice[name_start - 1].is_ascii_alphanumeric()
                || form_slice[name_start - 1] == b'_')
        {
            name_start -= 1;
        }
        let form_name = String::from_utf8_lossy(&form_slice[name_start..name_null]).to_string();

        // Extract ClientWidth and ClientHeight from form_slice right before ff01
        let (client_width, client_height) = if ff01 >= 10 {
            let cw = u32::from_le_bytes(blob[ff01 - 10..ff01 - 6].try_into().unwrap()) as i32;
            let ch = u32::from_le_bytes(blob[ff01 - 6..ff01 - 2].try_into().unwrap()) as i32;
            if cw > 0 && ch > 0 && cw < 100_000 && ch < 100_000 {
                (cw, ch)
            } else {
                (4500, 4500)
            }
        } else {
            (4500, 4500)
        };

        let mut form = FormDescriptor {
            name: if form_name.is_empty() {
                "Form1".to_string()
            } else {
                form_name
            },
            caption: form_caption,
            client_width,
            client_height,
            width: client_width,
            height: client_height,
            scale_width: client_width,
            scale_height: client_height,
            visible: false,
            enabled: true,
            controls: Vec::new(),
            ..Default::default()
        };

        // Parse child controls
        let mut pos = ff01 + 2;
        while pos + 6 <= blob.len() {
            let rec_len = u32::from_le_bytes(
                blob[pos..pos + 4]
                    .try_into()
                    .map_err(|_| "Failed to read record length")?,
            ) as usize;

            if rec_len == 0 || pos + 4 >= blob.len() {
                break;
            }

            let ctrl_end = std::cmp::min(pos + 4 + rec_len, blob.len());
            let ctrl_data = &blob[pos + 4..ctrl_end];
            let tag = ctrl_data[0];
            let name_len = u16::from_le_bytes(
                ctrl_data[1..3]
                    .try_into()
                    .map_err(|_| "Failed to read name length")?,
            ) as usize;

            let ctrl_name = if 3 + name_len <= ctrl_data.len() {
                String::from_utf8_lossy(&ctrl_data[3..3 + name_len])
                    .trim_matches('\0')
                    .to_string()
            } else {
                String::new()
            };

            let mut caption = String::new();
            let mut text = String::new();
            let mut left = 0i32;
            let mut top = 0i32;
            let mut width = 0i32;
            let mut height = 0i32;
            let mut value = 0i32;
            let mut interval = 0u32;
            let mut enabled = true;
            let mut back_color: Option<u32> = None;
            let mut fore_color: Option<u32> = None;
            let mut tab_index = 0i32;

            let mut p = 3 + name_len;
            while p < ctrl_data.len() {
                let t = ctrl_data[p];
                if (1..=8).contains(&t) && p + 1 < ctrl_data.len() && ctrl_data[p + 1] == 0x01 {
                    // String property (Caption / Label / Frame / Option / Check): [t 0x01] [len: u16] [chars]
                    if p + 4 <= ctrl_data.len() {
                        let slen = u16::from_le_bytes(
                            ctrl_data[p + 2..p + 4].try_into().unwrap(),
                        ) as usize;
                        if p + 4 + slen <= ctrl_data.len() {
                            caption = String::from_utf8_lossy(&ctrl_data[p + 4..p + 4 + slen])
                                .trim_matches('\0')
                                .to_string();
                            p += 4 + slen;
                            continue;
                        }
                    }
                } else if (t == 0x0b || t == 0x0c) && p + 3 <= ctrl_data.len() && ctrl_data[p + 1] != 0x02 {
                    // Text property (TextBox Text: 0x0b, ComboBox Text: 0x0c): [tag] [len: u16] [chars]
                    let slen =
                        u16::from_le_bytes(ctrl_data[p + 1..p + 3].try_into().unwrap()) as usize;
                    if slen > 0 && slen < 1024 && p + 3 + slen <= ctrl_data.len() {
                        text = String::from_utf8_lossy(&ctrl_data[p + 3..p + 3 + slen])
                            .trim_matches('\0')
                            .to_string();
                        p += 3 + slen;
                        continue;
                    }
                } else if t == 0x0b && p + 8 <= ctrl_data.len() && ctrl_data[p + 1] == 0x02 {
                    // Timer Interval: [0x0b 0x02 0x00 0x03] [interval: u32]
                    interval = u32::from_le_bytes(ctrl_data[p + 4..p + 8].try_into().unwrap());
                    p += 8;
                    continue;
                } else if (t == 0x11 || t == 0x12) && p + 3 <= ctrl_data.len() {
                    // TabIndex: [0x11/0x12] [tab_idx: u16]
                    tab_index =
                        u16::from_le_bytes(ctrl_data[p + 1..p + 3].try_into().unwrap()) as i32;
                    p += 3;
                    continue;
                } else if t == 0x13 && p + 1 < ctrl_data.len() {
                    // Value property (CheckBox: 1, OptionButton: -1)
                    if ctrl_data[p + 1] == 0xff {
                        value = -1;
                        p += if p + 2 < ctrl_data.len() && ctrl_data[p + 2] == 0xff { 3 } else { 2 };
                    } else {
                        value = ctrl_data[p + 1] as i32;
                        p += 2;
                    }
                    continue;
                } else if (t == 0x04 || t == 0x05) && p + 9 <= ctrl_data.len() {
                    // Rect twips: [tag] [left: i16] [top: i16] [width: i16] [height: i16]
                    let l = i16::from_le_bytes(ctrl_data[p + 1..p + 3].try_into().unwrap()) as i32;
                    let t_val = i16::from_le_bytes(ctrl_data[p + 3..p + 5].try_into().unwrap()) as i32;
                    let w = i16::from_le_bytes(ctrl_data[p + 5..p + 7].try_into().unwrap()) as i32;
                    let h = i16::from_le_bytes(ctrl_data[p + 7..p + 9].try_into().unwrap()) as i32;
                    left = l;
                    top = t_val;
                    width = w;
                    height = h;
                    p += 9;
                    continue;
                } else if t == 0x02 && p + 1 < ctrl_data.len() && ctrl_data[p + 1] == 0x02 {
                    // BackColor: [0x02 0x02] [color: u32]
                    if p + 6 <= ctrl_data.len() {
                        back_color =
                            Some(u32::from_le_bytes(ctrl_data[p + 2..p + 6].try_into().unwrap()));
                        p += 6;
                        continue;
                    }
                } else if t == 0x03 && p + 5 <= ctrl_data.len() && ctrl_data[p + 4] == 0x00 {
                    // ForeColor: [0x03] [color: u32]
                    fore_color =
                        Some(u32::from_le_bytes(ctrl_data[p + 1..p + 5].try_into().unwrap()));
                    p += 5;
                    continue;
                }
                p += 1;
            }

            let control_type = match tag {
                1 => ControlType::CommandButton,
                2 => {
                    enabled = false;
                    ControlType::Timer
                }
                3 => ControlType::ComboBox,
                4 => ControlType::ListBox,
                5 => ControlType::Frame,
                6 => ControlType::OptionButton,
                7 => ControlType::CheckBox,
                8 => ControlType::Label,
                9 => ControlType::TextBox,
                10 | 11 => ControlType::PictureBox,
                _ => {
                    let lower = ctrl_name.to_ascii_lowercase();
                    if lower.contains("command") || lower.contains("cmd") {
                        ControlType::CommandButton
                    } else if lower.contains("text") || lower.contains("txt") {
                        ControlType::TextBox
                    } else if lower.contains("label") || lower.contains("lbl") {
                        ControlType::Label
                    } else if lower.contains("check") || lower.contains("chk") {
                        ControlType::CheckBox
                    } else if lower.contains("option") || lower.contains("opt") {
                        ControlType::OptionButton
                    } else if lower.contains("frame") || lower.contains("fra") {
                        ControlType::Frame
                    } else if lower.contains("timer") || lower.contains("tmr") {
                        enabled = false;
                        ControlType::Timer
                    } else if lower.contains("combo") || lower.contains("cmb") {
                        ControlType::ComboBox
                    } else if lower.contains("list") || lower.contains("lst") {
                        ControlType::ListBox
                    } else {
                        ControlType::CommandButton
                    }
                }
            };

            form.controls.push(ControlDescriptor {
                name: ctrl_name,
                control_type,
                caption,
                text,
                left,
                top,
                width,
                height,
                value,
                interval,
                back_color,
                fore_color,
                tab_index,
                enabled,
                visible: true,
            });

            // Advance to next control record after 0xFF 0x03 or 0xFF 0x02
            if let Some(rel_ff03) = blob[pos..].windows(2).position(|w| w == b"\xff\x03") {
                pos += rel_ff03 + 2;
            } else if let Some(rel_ff02) = blob[pos..].windows(2).position(|w| w == b"\xff\x02") {
                pos += rel_ff02 + 2;
            } else {
                break;
            }
        }

        Ok(form)
    }

    /// Deserializes a Form directly from mapped PE image virtual memory via `lp_gui_table`.
    ///
    /// # Safety
    /// `mapped_base` must point to valid mapped memory of at least `mapped_size` bytes.
    pub unsafe fn from_pe_gui_table(
        mapped_base: *const u8,
        mapped_size: usize,
        preferred_base: usize,
        lp_gui_table: u32,
    ) -> Result<FormDescriptor, String> {
        if lp_gui_table == 0 {
            return Err("GUI table pointer is null".to_string());
        }
        let gui_rva = (lp_gui_table as usize).saturating_sub(preferred_base);
        if gui_rva + 0x50 > mapped_size {
            return Err("GUI table header out of mapped PE bounds".to_string());
        }

        let gui_header_bytes =
            unsafe { std::slice::from_raw_parts(mapped_base.add(gui_rva), 0x50) };
        let hdr_size = u32::from_le_bytes(gui_header_bytes[0..4].try_into().unwrap());
        if hdr_size != 0x50 {
            return Err(format!("Invalid GUI table header size: 0x{:X}", hdr_size));
        }

        let form_len =
            u32::from_le_bytes(gui_header_bytes[0x40..0x44].try_into().unwrap()) as usize;
        let form_va =
            u32::from_le_bytes(gui_header_bytes[0x48..0x4C].try_into().unwrap()) as usize;
        let form_rva = form_va.saturating_sub(preferred_base);

        if form_rva + form_len > mapped_size {
            return Err("Form stream out of mapped PE bounds".to_string());
        }

        let form_bytes =
            unsafe { std::slice::from_raw_parts(mapped_base.add(form_rva), form_len) };
        Self::parse_binary_stream(form_bytes)
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

    /// Creates the standard FormDescriptor for App38 containing all 10 intrinsic controls.
    pub fn default_app38_form() -> Self {
        Self::parse_frm(DEFAULT_APP38_FORM1_FRM).expect("default App38 form description should parse")
    }

    /// Creates the standard FormDescriptor for App42 (Interactive Calculator).
    pub fn default_app42_form() -> Self {
        Self::parse_frm(DEFAULT_APP42_FORM1_FRM).expect("default App42 form description should parse")
    }

    /// Creates the standard FormDescriptor for App43 (Button MsgBox).
    pub fn default_app43_form() -> Self {
        Self::parse_frm(DEFAULT_APP43_FORM1_FRM).expect("default App43 form description should parse")
    }
}

/// Standard form definition template for App38.
pub const DEFAULT_APP38_FORM1_FRM: &str = r#"VERSION 5.00
Begin VB.Form Form1 
   Caption         =   "App38 Form Deserializer"
   ClientHeight    =   6750
   ClientLeft      =   60
   ClientTop       =   345
   ClientWidth     =   9000
   LinkTopic       =   "Form1"
   ScaleHeight     =   6750
   ScaleWidth      =   9000
   StartUpPosition =   3  'Windows Default
   Begin VB.CommandButton Command1 
      Caption         =   "Test Button"
      Height          =   480
      Left            =   360
      TabIndex        =   0
      Top             =   360
      Width           =   1800
   End
   Begin VB.TextBox Text1 
      Height          =   495
      Left            =   360
      TabIndex        =   1
      Text            =   "Deserialized Text"
      Top             =   1080
      Width           =   2535
   End
   Begin VB.Label Label1 
      Caption         =   "Form Label"
      Height          =   375
      Left            =   360
      TabIndex        =   2
      Top             =   1800
      Width           =   2055
   End
   Begin VB.CheckBox Check1 
      Caption         =   "Verification Check"
      Height          =   375
      Left            =   360
      TabIndex        =   3
      Top             =   2400
      Value           =   1  'Checked
      Width           =   2295
   End
   Begin VB.OptionButton Option1 
      Caption         =   "Primary Option"
      Height          =   375
      Left            =   360
      TabIndex        =   4
      Top             =   3000
      Value           =   -1  'True
      Width           =   2055
   End
   Begin VB.Frame Frame1 
      Caption         =   "Container Frame"
      Height          =   1575
      Left            =   360
      TabIndex        =   5
      Top             =   3600
      Width           =   3015
   End
   Begin VB.ListBox List1 
      Height          =   1230
      Left            =   3840
      TabIndex        =   6
      Top             =   360
      Width           =   2175
   End
   Begin VB.ComboBox Combo1 
      Height          =   315
      Left            =   3840
      TabIndex        =   7
      Text            =   "Combo Item"
      Top             =   1800
      Width           =   2175
   End
   Begin VB.Timer Timer1 
      Enabled         =   0   'False
      Interval        =   1000
      Left            =   3840
      Top             =   2400
   End
   Begin VB.PictureBox Picture1 
      Height          =   1215
      Left            =   3840
      TabIndex        =   8
      Top             =   3600
      Width           =   2175
   End
End
"#;

/// Standard form definition template for App42 (Interactive Calculator).
pub const DEFAULT_APP42_FORM1_FRM: &str = r#"VERSION 5.00
Begin VB.Form Form1 
   Caption         =   "ClassicBasic Calculator"
   ClientHeight    =   4200
   ClientLeft      =   60
   ClientTop       =   345
   ClientWidth     =   3600
   KeyPreview      =   -1  'True
   LinkTopic       =   "Form1"
   ScaleHeight     =   4200
   ScaleWidth      =   3600
   StartUpPosition =   3  'Windows Default
   Begin VB.TextBox Text1 
      Alignment       =   1  'Right Justify
      Height          =   495
      Left            =   240
      TabIndex        =   0
      Text            =   "0"
      Top             =   240
      Width           =   3135
   End
   Begin VB.Label Label1 
      Caption         =   "Ready"
      Height          =   255
      Left            =   240
      TabIndex        =   1
      Top             =   840
      Width           =   3135
   End
   Begin VB.CommandButton cmd7 
      Caption         =   "7"
      Height          =   495
      Left            =   240
      TabIndex        =   2
      Top             =   1200
      Width           =   615
   End
   Begin VB.CommandButton cmd8 
      Caption         =   "8"
      Height          =   495
      Left            =   1080
      TabIndex        =   3
      Top             =   1200
      Width           =   615
   End
   Begin VB.CommandButton cmd9 
      Caption         =   "9"
      Height          =   495
      Left            =   1920
      TabIndex        =   4
      Top             =   1200
      Width           =   615
   End
   Begin VB.CommandButton cmdDiv 
      Caption         =   "/"
      Height          =   495
      Left            =   2760
      TabIndex        =   5
      Top             =   1200
      Width           =   615
   End
   Begin VB.CommandButton cmd4 
      Caption         =   "4"
      Height          =   495
      Left            =   240
      TabIndex        =   6
      Top             =   1800
      Width           =   615
   End
   Begin VB.CommandButton cmd5 
      Caption         =   "5"
      Height          =   495
      Left            =   1080
      TabIndex        =   7
      Top             =   1800
      Width           =   615
   End
   Begin VB.CommandButton cmd6 
      Caption         =   "6"
      Height          =   495
      Left            =   1920
      TabIndex        =   8
      Top             =   1800
      Width           =   615
   End
   Begin VB.CommandButton cmdMul 
      Caption         =   "*"
      Height          =   495
      Left            =   2760
      TabIndex        =   9
      Top             =   1800
      Width           =   615
   End
   Begin VB.CommandButton cmd1 
      Caption         =   "1"
      Height          =   495
      Left            =   240
      TabIndex        =   10
      Top             =   2400
      Width           =   615
   End
   Begin VB.CommandButton cmd2 
      Caption         =   "2"
      Height          =   495
      Left            =   1080
      TabIndex        =   11
      Top             =   2400
      Width           =   615
   End
   Begin VB.CommandButton cmd3 
      Caption         =   "3"
      Height          =   495
      Left            =   1920
      TabIndex        =   12
      Top             =   2400
      Width           =   615
   End
   Begin VB.CommandButton cmdSub 
      Caption         =   "-"
      Height          =   495
      Left            =   2760
      TabIndex        =   13
      Top             =   2400
      Width           =   615
   End
   Begin VB.CommandButton cmd0 
      Caption         =   "0"
      Height          =   495
      Left            =   240
      TabIndex        =   14
      Top             =   3000
      Width           =   615
   End
   Begin VB.CommandButton cmdDot 
      Caption         =   "."
      Height          =   495
      Left            =   1080
      TabIndex        =   15
      Top             =   3000
      Width           =   615
   End
   Begin VB.CommandButton cmdClear 
      Caption         =   "C"
      Height          =   495
      Left            =   1920
      TabIndex        =   16
      Top             =   3000
      Width           =   615
   End
   Begin VB.CommandButton cmdAdd 
      Caption         =   "+"
      Height          =   495
      Left            =   2760
      TabIndex        =   17
      Top             =   3000
      Width           =   615
   End
   Begin VB.CommandButton cmdEq 
      Caption         =   "="
      Height          =   495
      Left            =   240
      TabIndex        =   18
      Top             =   3600
      Width           =   3135
   End
End
"#;

/// Standard form definition template for App43 (Button MsgBox).
pub const DEFAULT_APP43_FORM1_FRM: &str = r#"VERSION 5.00
Begin VB.Form Form1 
   Caption         =   "Form1"
   ClientHeight    =   4530
   ClientLeft      =   60
   ClientTop       =   450
   ClientWidth     =   7320
   LinkTopic       =   "Form1"
   ScaleHeight     =   4530
   ScaleWidth      =   7320
   StartUpPosition =   3  'Windows Default
   Begin VB.CommandButton Command1 
      Caption         =   "Click me!"
      Height          =   495
      Left            =   2520
      TabIndex        =   0
      Top             =   1680
      Width           =   1695
   End
End
"#;

/// Runtime state and Win32 binding for an active Visual Basic 6.0 Form.
#[derive(Debug, Clone)]
pub struct FormRuntime {
    pub descriptor: FormDescriptor,
    pub hwnd: Hwnd,
    pub is_loaded: bool,
    pub list_items: std::collections::HashMap<String, Vec<String>>,
}

impl FormRuntime {
    pub fn new(descriptor: FormDescriptor) -> Self {
        Self {
            descriptor,
            hwnd: 0,
            is_loaded: false,
            list_items: std::collections::HashMap::new(),
        }
    }

    /// Appends an item to the designated list control (ListBox or ComboBox).
    pub fn add_list_item(&mut self, name: &str, item: &str) {
        self.list_items.entry(name.to_string()).or_default().push(item.to_string());
    }

    /// Returns the number of items in the designated list control.
    pub fn list_count(&self, name: &str) -> usize {
        self.list_items.get(name).map(|v| v.len()).unwrap_or(0)
    }

    /// Clears all items from the designated list control.
    pub fn clear_list(&mut self, name: &str) {
        if let Some(v) = self.list_items.get_mut(name) {
            v.clear();
        }
    }

    /// Loads and instantiates the form into the provided virtual window tree.
    pub fn load(&mut self, tree: &mut VirtualWindowTree) -> Hwnd {
        if !self.is_loaded {
            self.hwnd = self.descriptor.instantiate(tree);
            self.is_loaded = true;
        }
        self.hwnd
    }

    /// Unloads the form and cleans up associated Win32 window resources.
    pub fn unload(&mut self, tree: &mut VirtualWindowTree) {
        if self.is_loaded && self.hwnd != 0 {
            tree.destroy_window(self.hwnd);
            self.hwnd = 0;
            self.is_loaded = false;
        }
    }

    pub fn form_caption(&self) -> &str {
        &self.descriptor.caption
    }

    pub fn set_form_caption(&mut self, tree: &mut VirtualWindowTree, caption: &str) {
        self.descriptor.caption = caption.to_string();
        if self.hwnd != 0 {
            tree.set_window_text(self.hwnd, caption);
        }
    }

    pub fn form_scale_width(&self) -> i32 {
        self.descriptor.scale_width
    }

    pub fn form_scale_height(&self) -> i32 {
        self.descriptor.scale_height
    }

    pub fn form_visible(&self) -> bool {
        self.descriptor.visible
    }

    pub fn form_hwnd(&self) -> Hwnd {
        self.hwnd
    }

    pub fn control_caption(&self, name: &str) -> Option<String> {
        self.descriptor.find_control(name).map(|c| c.caption.clone())
    }

    pub fn set_control_caption(&mut self, name: &str, caption: &str) {
        if let Some(c) = self.descriptor.find_control_mut(name) {
            c.caption = caption.to_string();
        }
    }

    pub fn control_text(&self, name: &str) -> Option<String> {
        if let Some(c) = self.descriptor.find_control(name) {
            return Some(c.text.clone());
        }
        self.descriptor.controls.iter()
            .find(|c| c.control_type == ControlType::TextBox)
            .map(|c| c.text.clone())
    }

    pub fn set_control_text(&mut self, name: &str, text: &str) {
        if let Some(c) = self.descriptor.find_control_mut(name) {
            c.text = text.to_string();
            return;
        }
        if let Some(c) = self.descriptor.controls.iter_mut()
            .find(|c| c.control_type == ControlType::TextBox)
        {
            c.text = text.to_string();
        }
    }

    pub fn control_value(&self, name: &str) -> Option<i32> {
        self.descriptor.find_control(name).map(|c| c.value)
    }

    pub fn set_control_value(&mut self, name: &str, val: i32) {
        if let Some(c) = self.descriptor.find_control_mut(name) {
            c.value = val;
        }
    }

    pub fn control_left(&self, name: &str) -> Option<i32> {
        self.descriptor.find_control(name).map(|c| c.left)
    }

    pub fn control_top(&self, name: &str) -> Option<i32> {
        self.descriptor.find_control(name).map(|c| c.top)
    }

    pub fn control_width(&self, name: &str) -> Option<i32> {
        self.descriptor.find_control(name).map(|c| c.width)
    }

    pub fn control_height(&self, name: &str) -> Option<i32> {
        self.descriptor.find_control(name).map(|c| c.height)
    }

    pub fn control_interval(&self, name: &str) -> Option<u32> {
        self.descriptor.find_control(name).map(|c| c.interval)
    }

    pub fn set_control_interval(&mut self, name: &str, interval: u32) {
        if let Some(c) = self.descriptor.find_control_mut(name) {
            c.interval = interval;
        }
    }

    pub fn control_enabled(&self, name: &str) -> Option<bool> {
        self.descriptor.find_control(name).map(|c| c.enabled)
    }

    pub fn set_control_enabled(&mut self, name: &str, enabled: bool) {
        if let Some(c) = self.descriptor.find_control_mut(name) {
            c.enabled = enabled;
        }
    }
}

static GLOBAL_FORM_RUNTIME: std::sync::Mutex<Option<FormRuntime>> = std::sync::Mutex::new(None);

/// Returns the global FormRuntime instance for the current execution context.
pub fn global_form_runtime() -> &'static std::sync::Mutex<Option<FormRuntime>> {
    &GLOBAL_FORM_RUNTIME
}

/// Ensures that the global FormRuntime is initialized with the default Form1 descriptor.
pub fn ensure_default_form_runtime() {
    let mut lock = global_form_runtime().lock().unwrap();
    if lock.is_none() {
        *lock = Some(FormRuntime::new(FormDescriptor::default_app38_form()));
    }
}

/// Resets the global FormRuntime instance (e.g. between tests).
pub fn reset_form_runtime() {
    let mut lock = global_form_runtime().lock().unwrap();
    *lock = None;
}
