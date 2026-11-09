// Clean-room re-implementation. Behavior derived from publicly documented
// VB6 / COM / Win32 ABI semantics. No Microsoft or third-party source used.

//! Unit tests for Phase 3.4: VB6 Form Deserializer & Twips-to-Pixels Engine.

use cb_win32::*;

#[test]
fn test_twips_to_pixels_conversion() {
    assert_eq!(TWIPS_PER_INCH, 1440);
    assert_eq!(DEFAULT_DPI, 96);
    assert_eq!(TWIPS_PER_PIXEL, 15);

    // Standard control positions in twips
    assert_eq!(twips_to_pixels(360), 24);
    assert_eq!(twips_to_pixels(1800), 120);
    assert_eq!(twips_to_pixels(480), 32);
    assert_eq!(twips_to_pixels(9000), 600);
    assert_eq!(twips_to_pixels(6750), 450);

    // Roundtrip
    assert_eq!(pixels_to_twips(24), 360);
    assert_eq!(pixels_to_twips(120), 1800);
    assert_eq!(pixels_to_twips(600), 9000);
}

#[test]
fn test_control_type_and_class_names() {
    assert_eq!(ControlType::Form.class_name(), "ThunderRT6FormDC");
    assert_eq!(ControlType::CommandButton.class_name(), "ThunderRT6CommandButton");
    assert_eq!(ControlType::TextBox.class_name(), "ThunderRT6TextBox");
    assert_eq!(ControlType::Label.class_name(), "ThunderRT6Label");
    assert_eq!(ControlType::CheckBox.class_name(), "ThunderRT6CheckBox");
    assert_eq!(ControlType::OptionButton.class_name(), "ThunderRT6OptionButton");
    assert_eq!(ControlType::Frame.class_name(), "ThunderRT6Frame");
    assert_eq!(ControlType::ListBox.class_name(), "ThunderRT6ListBox");
    assert_eq!(ControlType::ComboBox.class_name(), "ThunderRT6ComboBox");
    assert_eq!(ControlType::Timer.class_name(), "ThunderRT6Timer");
    assert_eq!(ControlType::PictureBox.class_name(), "ThunderRT6PictureBox");

    assert_eq!(ControlType::from_vb_type("VB.CommandButton"), Some(ControlType::CommandButton));
    assert_eq!(ControlType::from_vb_type("VB.TextBox"), Some(ControlType::TextBox));
    assert_eq!(ControlType::from_vb_type("VB.Timer"), Some(ControlType::Timer));
    assert_eq!(ControlType::from_vb_type("UnknownControl"), None);
}

#[test]
fn test_frm_text_parsing() {
    let frm_content = r#"
VERSION 5.00
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
      Height          =   420
      Left            =   360
      TabIndex        =   2
      Text            =   "Deserialized Text"
      Top             =   1080
      Width           =   2700
   End
   Begin VB.CheckBox Check1 
      Caption         =   "Check Feature"
      Height          =   360
      Left            =   360
      TabIndex        =   4
      Top             =   2400
      Value           =   1  'Checked
      Width           =   2100
   End
   Begin VB.Timer Timer1 
      Enabled         =   0   'False
      Interval        =   1000
      Left            =   3300
      Top             =   3600
   End
End
"#;

    let form = FormDescriptor::parse_frm(frm_content).expect("Failed to parse .frm");
    assert_eq!(form.name, "Form1");
    assert_eq!(form.caption, "App38 Form Deserializer");
    assert_eq!(form.client_width, 9000);
    assert_eq!(form.client_height, 6750);
    assert_eq!(form.pixel_client_width(), 600);
    assert_eq!(form.pixel_client_height(), 450);

    assert_eq!(form.controls.len(), 4);

    let cmd = form.find_control("Command1").expect("Command1 missing");
    assert_eq!(cmd.caption, "Test Button");
    assert_eq!(cmd.control_type, ControlType::CommandButton);
    assert_eq!(cmd.pixel_x(), 24);
    assert_eq!(cmd.pixel_y(), 24);
    assert_eq!(cmd.pixel_width(), 120);
    assert_eq!(cmd.pixel_height(), 32);

    let txt = form.find_control("Text1").expect("Text1 missing");
    assert_eq!(txt.text, "Deserialized Text");
    assert_eq!(txt.control_type, ControlType::TextBox);

    let chk = form.find_control("Check1").expect("Check1 missing");
    assert_eq!(chk.value, 1);

    let tmr = form.find_control("Timer1").expect("Timer1 missing");
    assert_eq!(tmr.interval, 1000);
    assert!(!tmr.enabled);
}

#[test]
fn test_form_instantiation_into_window_tree() {
    let mut tree = VirtualWindowTree::new();

    let mut form = FormDescriptor {
        name: "TestForm".to_string(),
        caption: "Instantiated Form".to_string(),
        client_width: 9000,
        client_height: 6750,
        scale_width: 9000,
        scale_height: 6750,
        ..Default::default()
    };

    form.controls.push(ControlDescriptor {
        name: "BtnSubmit".to_string(),
        control_type: ControlType::CommandButton,
        caption: "Submit".to_string(),
        left: 360,
        top: 360,
        width: 1800,
        height: 480,
        enabled: true,
        visible: true,
        ..Default::default()
    });

    let hwnd = form.instantiate(&mut tree);
    assert_ne!(hwnd, 0);

    // Verify root form window
    let win = tree.get_window(hwnd).expect("Form window not found");
    assert_eq!(win.class_name, "ThunderRT6FormDC");
    assert_eq!(win.caption, "Instantiated Form");
    assert_eq!(win.width, 600);
    assert_eq!(win.height, 450);
    assert_eq!(win.children.len(), 1);

    // Verify child control window
    let child_hwnd = win.children[0];
    let child_win = tree.get_window(child_hwnd).expect("Child button not found");
    assert_eq!(child_win.class_name, "ThunderRT6CommandButton");
    assert_eq!(child_win.caption, "Submit");
    assert_eq!(child_win.x, 24);
    assert_eq!(child_win.y, 24);
    assert_eq!(child_win.width, 120);
    assert_eq!(child_win.height, 32);
    assert_eq!(child_win.parent, hwnd);
}

#[test]
fn test_app42_calculator_form_deserializer() {
    let form = FormDescriptor::default_app42_form();
    assert_eq!(form.name, "Form1");
    assert_eq!(form.caption, "ClassicBasic Calculator");
    assert_eq!(form.client_width, 3600);
    assert_eq!(form.client_height, 4200);
    assert_eq!(form.pixel_client_width(), 240);
    assert_eq!(form.pixel_client_height(), 280);

    // 1 TextBox + 1 Label + 17 CommandButtons = 19 controls
    assert_eq!(form.controls.len(), 19);

    let txt = form.find_control("Text1").expect("Text1 missing");
    assert_eq!(txt.text, "0");
    assert_eq!(txt.control_type, ControlType::TextBox);

    let lbl = form.find_control("Label1").expect("Label1 missing");
    assert_eq!(lbl.caption, "Ready");
    assert_eq!(lbl.control_type, ControlType::Label);

    for digit in 0..=9 {
        let name = format!("cmd{}", digit);
        let btn = form.find_control(&name).unwrap_or_else(|| panic!("{} missing", name));
        assert_eq!(btn.caption, digit.to_string());
        assert_eq!(btn.control_type, ControlType::CommandButton);
    }

    assert!(form.find_control("cmdAdd").is_some());
    assert!(form.find_control("cmdSub").is_some());
    assert!(form.find_control("cmdMul").is_some());
    assert!(form.find_control("cmdDiv").is_some());
    assert!(form.find_control("cmdEq").is_some());
    assert!(form.find_control("cmdClear").is_some());
    assert!(form.find_control("cmdDot").is_some());
}

#[test]
fn test_app42_calculator_runtime_lifecycle() {
    let mut tree = VirtualWindowTree::new();
    let desc = FormDescriptor::default_app42_form();
    let mut runtime = FormRuntime::new(desc);

    runtime.load(&mut tree);
    assert!(runtime.is_loaded);
    assert_ne!(runtime.form_hwnd(), 0);
    assert_eq!(runtime.form_caption(), "ClassicBasic Calculator");
    assert_eq!(runtime.control_text("Text1"), Some("0".to_string()));
    assert_eq!(runtime.control_caption("Label1"), Some("Ready".to_string()));

    // Simulate input mutation
    runtime.set_control_text("Text1", "46");
    runtime.set_control_caption("Label1", "12 + 34 = 46");
    assert_eq!(runtime.control_text("Text1"), Some("46".to_string()));
    assert_eq!(runtime.control_caption("Label1"), Some("12 + 34 = 46".to_string()));

    runtime.unload(&mut tree);
    assert!(!runtime.is_loaded);
}

#[test]
fn test_app43_button_msgbox_form_parsing() {
    let form = FormDescriptor::default_app43_form();
    assert_eq!(form.name, "Form1");
    assert_eq!(form.caption, "Form1");
    assert_eq!(form.client_width, 7320);
    assert_eq!(form.client_height, 4530);
    assert_eq!(form.pixel_client_width(), 488);
    assert_eq!(form.pixel_client_height(), 302);

    assert_eq!(form.controls.len(), 1);
    let btn = form.find_control("Command1").expect("Command1 missing");
    assert_eq!(btn.caption, "Click me!");
    assert_eq!(btn.control_type, ControlType::CommandButton);
    assert_eq!(btn.left, 2520);
    assert_eq!(btn.top, 1680);
    assert_eq!(btn.width, 1695);
    assert_eq!(btn.height, 495);
    assert_eq!(btn.pixel_x(), 168);
    assert_eq!(btn.pixel_y(), 112);
    assert_eq!(btn.pixel_width(), 113);
    assert_eq!(btn.pixel_height(), 33);
}

#[test]
fn test_app43_runtime_lifecycle() {
    let mut tree = VirtualWindowTree::new();
    let desc = FormDescriptor::default_app43_form();
    let mut runtime = FormRuntime::new(desc);

    runtime.load(&mut tree);
    assert!(runtime.is_loaded);
    assert_ne!(runtime.form_hwnd(), 0);
    assert_eq!(runtime.form_caption(), "Form1");
    assert_eq!(runtime.control_caption("Command1"), Some("Click me!".to_string()));

    runtime.unload(&mut tree);
    assert!(!runtime.is_loaded);
}

#[test]
fn test_parse_binary_form_stream_from_pe() {
    // Test parsing App43 binary form directly from disk
    if let Ok(data) = std::fs::read("artifacts/bin/App43.exe") {
        let vb5_pos = data.windows(4).position(|w| w == b"VB5!").expect("VB5! signature");
        let lp_gui = u32::from_le_bytes(data[vb5_pos + 0x4C..vb5_pos + 0x50].try_into().unwrap());
        assert_ne!(lp_gui, 0);

        let form = unsafe { FormDescriptor::from_pe_gui_table(data.as_ptr(), data.len(), 0x00400000, lp_gui) }
            .expect("Failed to deserialize App43 binary form");

        assert_eq!(form.caption, "XCalculator");
        assert_eq!(form.controls.len(), 16);

        // Verify TextBox
        let tb = form.find_control("Text_calc_display").expect("Text_calc_display not found");
        assert_eq!(tb.control_type, ControlType::TextBox);
        assert_eq!(tb.pixel_width(), 217);
        assert_eq!(tb.pixel_height(), 39);
        assert_eq!(tb.back_color, Some(0x00FFC0FF));

        // Verify buttons
        let btn_div = form.find_control("Command_divise").expect("Command_divise not found");
        assert_eq!(btn_div.control_type, ControlType::CommandButton);
        assert_eq!(btn_div.caption, "/");
        assert_eq!(btn_div.pixel_x(), 176);
        assert_eq!(btn_div.pixel_y(), 240);
        assert_eq!(btn_div.pixel_width(), 49);
        assert_eq!(btn_div.pixel_height(), 41);

        let btn_eq = form.find_control("Command_equals").expect("Command_equals not found");
        assert_eq!(btn_eq.control_type, ControlType::CommandButton);
        assert_eq!(btn_eq.caption, "=");
        assert_eq!(btn_eq.pixel_x(), 120);
        assert_eq!(btn_eq.pixel_y(), 240);
        assert_eq!(btn_eq.pixel_width(), 49);
        assert_eq!(btn_eq.pixel_height(), 41);
    }

    // Test parsing App42 calculator binary form directly from disk
    if let Ok(data) = std::fs::read("artifacts/bin/App42-gui-calculator-e2e-native.exe") {
        let vb5_pos = data.windows(4).position(|w| w == b"VB5!").expect("VB5! signature");
        let lp_gui = u32::from_le_bytes(data[vb5_pos + 0x4C..vb5_pos + 0x50].try_into().unwrap());
        assert_ne!(lp_gui, 0);

        let form = unsafe { FormDescriptor::from_pe_gui_table(data.as_ptr(), data.len(), 0x00400000, lp_gui) }
            .expect("Failed to deserialize App42 binary form");

        assert_eq!(form.caption, "ClassicBasic Calculator");
        assert_eq!(form.controls.len(), 19);

        let cmd7 = form.find_control("cmd7").expect("cmd7 not found");
        assert_eq!(cmd7.caption, "7");
        assert_eq!(cmd7.control_type, ControlType::CommandButton);
    }

    // Test parsing Alarm binary form geometry and embedded icon
    if let Ok(data) = std::fs::read("artifacts/bin/Samples/Alarm/Alarm-native.exe") {
        let vb5_pos = data.windows(4).position(|w| w == b"VB5!").expect("VB5! signature");
        let lp_gui = u32::from_le_bytes(data[vb5_pos + 0x4C..vb5_pos + 0x50].try_into().unwrap());
        assert_ne!(lp_gui, 0);

        let form = unsafe { FormDescriptor::from_pe_gui_table(data.as_ptr(), data.len(), 0x00400000, lp_gui) }
            .expect("Failed to deserialize Alarm binary form");

        assert_eq!(form.caption, "Alarm");
        assert_eq!(form.client_width, 3135);
        assert_eq!(form.client_height, 780);
        assert_eq!(form.pixel_client_width(), 209);
        assert_eq!(form.pixel_client_height(), 52);
        assert_eq!(form.max_button, false);
        assert_eq!(form.min_button, true);

        // Verify embedded icon is present and decodes to a 32x32 image
        assert!(form.icon_data.is_some());
        let (w, h, pixels) = form.decode_icon().expect("Failed to decode embedded icon");
        assert_eq!(w, 32);
        assert_eq!(h, 32);
        assert_eq!(pixels.len(), 32 * 32 * 4);

        // Downsampling produces a 16x16 icon for the title bar
        let downsampled = downsample_32_to_16(&pixels);
        assert_eq!(downsampled.len(), 16 * 16 * 4);

        // Verify controls
        assert_eq!(form.controls.len(), 3);
        let lbl_time = form.find_control("lblTime").expect("lblTime control not found");
        assert_eq!(lbl_time.control_type, ControlType::Label);
        assert_eq!(lbl_time.pixel_x(), 8);
        assert_eq!(lbl_time.pixel_y(), 8);
        assert_eq!(lbl_time.pixel_width(), 193);
        assert_eq!(lbl_time.pixel_height(), 35);
    }
}

#[test]
fn test_desktop_themes_and_control_panel() {
    // 1. Verify WindowTheme enum values and client background colors
    assert_eq!(WindowTheme::Windows311.name(), "Windows 3.11");
    assert_eq!(WindowTheme::Windows98.name(), "Windows 98");
    assert_eq!(WindowTheme::WindowsXp.name(), "Windows XP");
    assert_eq!(WindowTheme::Windows311.client_bg_color(), (255, 255, 255));
    assert_eq!(WindowTheme::Windows98.client_bg_color(), (200, 208, 212));
    assert_eq!(WindowTheme::WindowsXp.client_bg_color(), (216, 233, 236));

    // 2. Verify Windows 3.11 Control Panel icon generation
    let cp_icon = generate_win311_control_panel_icon();
    assert_eq!(cp_icon.len(), 32 * 32 * 4);
    let cp_16 = downsample_32_to_16(&cp_icon);
    assert_eq!(cp_16.len(), 16 * 16 * 4);

    // 3. Verify dynamic 24h <-> 12h time string conversion
    let t24 = "17:35:42";
    let t12 = format_time_label(t24, true);
    assert_eq!(t12, "05:35:42 PM");
    let back_to_24 = format_time_label(&t12, false);
    assert_eq!(back_to_24, "17:35:42");

    let t23 = "23:29:00";
    let t23_12 = format_time_label(t23, true);
    assert_eq!(t23_12, "11:29:00 PM");
    let t23_already_pm = format_time_label("11:29:00 PM", true);
    assert_eq!(t23_already_pm, "11:29:00 PM");
    let t23_back_to_24 = format_time_label(&t23_12, false);
    assert_eq!(t23_back_to_24, "23:29:00");

    let t_morning = "09:05:01";
    assert_eq!(format_time_label(t_morning, true), "09:05:01 AM");

    let t_midnight = "00:15:30";
    assert_eq!(format_time_label(t_midnight, true), "12:15:30 AM");

    let t_noon = "12:00:00";
    assert_eq!(format_time_label(t_noon, true), "12:00:00 PM");
}

#[test]
fn test_form_max_min_button_and_theme_properties() {
    let frm_content = r#"
VERSION 5.00
Begin VB.Form FormAlarm
   Caption         =   "Alarm"
   ClientHeight    =   780
   ClientWidth     =   3135
   MaxButton       =   0   'False
   MinButton       =   1   'True
   ScaleHeight     =   780
   ScaleWidth      =   3135
End
"#;
    let form = FormDescriptor::parse_frm(frm_content).expect("Failed to parse frm");
    assert_eq!(form.max_button, false);
    assert_eq!(form.min_button, true);

    // Default when omitted should be true
    let default_frm = r#"
VERSION 5.00
Begin VB.Form FormDefault
   Caption         =   "Default"
   ClientHeight    =   1000
   ClientWidth     =   1000
End
"#;
    let def_form = FormDescriptor::parse_frm(default_frm).expect("Failed to parse default frm");
    assert_eq!(def_form.max_button, true);
    assert_eq!(def_form.min_button, true);

    // Active vs Inactive title text colors across themes
    assert_eq!(WindowTheme::Windows311.title_text_color(true), 0x00FFFFFF);
    assert_eq!(WindowTheme::Windows311.title_text_color(false), 0x00000000);

    assert_eq!(WindowTheme::Windows98.title_text_color(true), 0x00FFFFFF);
    assert_eq!(WindowTheme::Windows98.title_text_color(false), 0x00D4D0C8);

    assert_eq!(WindowTheme::WindowsXp.title_text_color(true), 0x00FFFFFF);
    assert_eq!(WindowTheme::WindowsXp.title_text_color(false), 0x00D8E0F0);

    // Theme metrics and geometry
    assert_eq!(WindowTheme::Windows311.border_width(), 5);
    assert_eq!(WindowTheme::Windows311.title_bar_height(), 24);
    assert_eq!(WindowTheme::Windows311.client_offset(), (5, 29));
    assert_eq!(WindowTheme::Windows311.window_size(200, 50), (210, 84));

    assert_eq!(WindowTheme::Windows98.border_width(), 3);
    assert_eq!(WindowTheme::Windows98.title_bar_height(), 18);
    assert_eq!(WindowTheme::Windows98.client_offset(), (3, 21));
    assert_eq!(WindowTheme::Windows98.window_size(200, 50), (206, 74));

    assert_eq!(WindowTheme::WindowsXp.border_width(), 3);
    assert_eq!(WindowTheme::WindowsXp.title_bar_height(), 26);
    assert_eq!(WindowTheme::WindowsXp.client_offset(), (3, 29));
    assert_eq!(WindowTheme::WindowsXp.window_size(200, 50), (206, 82));
}

#[test]
fn test_windows_311_title_bar_layout_and_drag_math() {
    let theme = WindowTheme::Windows311;
    let (off_x, off_y) = theme.client_offset();
    assert_eq!(off_x, 5);
    assert_eq!(off_y, 29); // 5px border + 24px title bar

    // Title text centering:
    let win_w: usize = 400;
    let caption = "Alarm";
    let approx_text_w: usize = caption.len() * 7; // 35px
    let center_x: usize = win_w / 2; // 200
    let title_x = center_x.saturating_sub(approx_text_w / 2);
    assert_eq!(title_x, 183);

    // Drag tracker offset & bounds clamping:
    let mouse_down_x: i32 = 100;
    let mouse_down_y: i32 = 10;
    let drag_offset_x = mouse_down_x - 50; // win_x was 50
    let drag_offset_y = mouse_down_y - 20; // win_y was 20
    assert_eq!(drag_offset_x, 50);
    assert_eq!(drag_offset_y, -10);

    let mouse_move_x: i32 = 180;
    let mouse_move_y: i32 = 90;
    let new_win_x = mouse_move_x - drag_offset_x;
    let new_win_y = mouse_move_y - drag_offset_y;
    assert_eq!(new_win_x, 130);
    assert_eq!(new_win_y, 100);

    // Delta applied to controls:
    let dx = new_win_x - 50;
    let dy = new_win_y - 20;
    assert_eq!(dx, 80);
    assert_eq!(dy, 80);
}

#[test]
fn test_blanker_form_stream_and_menu_parsing() {
    let path = if std::path::Path::new("artifacts/bin/Samples/Blanker/Blanker-native.exe").exists() {
        "artifacts/bin/Samples/Blanker/Blanker-native.exe"
    } else {
        "../../artifacts/bin/Samples/Blanker/Blanker-native.exe"
    };
    if !std::path::Path::new(path).exists() {
        panic!("Blanker-native.exe not found at {}", path);
    }
    let data = std::fs::read(path).expect("Failed to read Blanker-native.exe");
    let magic_pos = data.windows(4).position(|w| w == b"\xff\xcc1\x00").expect("Magic not found");
    let form_slice = &data[magic_pos..];
    let form = FormDescriptor::parse_binary_stream(form_slice).expect("Failed to parse binary stream");

    let vb5_pos = data.windows(4).position(|w| w == b"VB5!").expect("VB5! signature");
    let lp_gui = u32::from_le_bytes(data[vb5_pos + 0x4C..vb5_pos + 0x50].try_into().unwrap());
    println!("Blanker lp_gui: 0x{:08X}", lp_gui);
    let _res = unsafe {
        FormDescriptor::from_pe_gui_table(
            data.as_ptr(),
            data.len(),
            0x00400000,
            lp_gui,
        )
    };
    println!("Blanker controls:");
    for c in &form.controls {
        println!("  - {} ({:?}): left={} top={} w={} h={} caption=\"{}\"", c.name, c.control_type, c.left, c.top, c.width, c.height, c.caption);
    }
    println!("Blanker menus: {}", form.menus.len());
    for m in &form.menus {
        println!("  Menu: {} \"{}\", sub_items={}", m.name, m.caption, m.sub_items.len());
        for sub in &m.sub_items {
            println!("    Sub: {} \"{}\" (checked={})", sub.name, sub.caption, sub.checked);
        }
    }

    assert_eq!(form.name, "DemoForm");
    assert_eq!(form.caption, "Screen Blanker Demo");
    assert!(form.max_button, "Screen Blanker must support maximize");

    // Check menu tree deserialization
    assert_eq!(form.menus.len(), 1);
    let opt_menu = &form.menus[0];
    assert_eq!(opt_menu.name, "mnuOption");
    assert_eq!(opt_menu.caption, "&Options");
    assert_eq!(opt_menu.sub_items.len(), 10);

    let jumpy = &opt_menu.sub_items[0];
    assert_eq!(jumpy.name, "mnuLineCtlDemo");
    assert_eq!(jumpy.caption, "&Jumpy Line");
    assert!(jumpy.checked, "Default demo should be checked");

    let sep = &opt_menu.sub_items[8];
    assert_eq!(sep.name, "sep1");
    assert!(sep.separator);

    let exit = &opt_menu.sub_items[9];
    assert_eq!(exit.name, "mnuExit");
    assert_eq!(exit.caption, "E&xit");

    // Check controls
    assert!(form.controls.iter().any(|c| c.name == "cmdStartStop" && c.caption == "Start Demo"));
    assert!(form.controls.iter().any(|c| c.name == "Timer1"));
}

#[test]
fn test_atm_form_stream_and_controls_parsing() {
    let path = if std::path::Path::new("artifacts/bin/Samples/Atm/ATM32-native.EXE").exists() {
        "artifacts/bin/Samples/Atm/ATM32-native.EXE"
    } else {
        "../../artifacts/bin/Samples/Atm/ATM32-native.EXE"
    };
    if !std::path::Path::new(path).exists() {
        return;
    }
    let data = std::fs::read(path).expect("Failed to read ATM32-native.EXE");
    let magic_pos = data.windows(4).position(|w| w == b"\xff\xcc1\x00").expect("Magic not found");
    let form_slice = &data[magic_pos..];
    let form = FormDescriptor::parse_binary_stream(form_slice).expect("Failed to parse ATM binary stream");

    assert_eq!(form.name, "frmOpen");
    assert_eq!(form.caption, "Automated Teller Machine (A.T.M.)");
    assert_eq!(form.controls.len(), 17, "ATM frmOpen must contain 17 child controls");

    // Verify presence of buttons
    let cmd1 = form.find_control("Command1").expect("Command1 must exist");
    assert_eq!(cmd1.caption, "English");
    let cmd2 = form.find_control("Command2").expect("Command2 must exist");
    assert_eq!(cmd2.caption, "Francais");
    let cmd3 = form.find_control("Command3").expect("Command3 must exist");
    assert_eq!(cmd3.caption, "Deutsch");
    let cmd4 = form.find_control("Command4").expect("Command4 must exist");
    assert_eq!(cmd4.caption, "Italiano");
    let cmd5 = form.find_control("Command5").expect("Command5 must exist");
    assert_eq!(cmd5.caption, "Espanol");
    let cmd6 = form.find_control("Command6").expect("Command6 must exist");
    assert_eq!(cmd6.caption, "Japan");

    // Verify Frame container
    let frame = form.find_control("Frame").expect("Frame must exist");
    assert_eq!(frame.control_type, ControlType::Frame);
    assert_eq!(frame.caption, "Please select a language:");

    // Verify Images with embedded ICO pictures
    let mut image_count = 0;
    for c in &form.controls {
        if c.control_type == ControlType::Image {
            image_count += 1;
            assert!(c.picture_data.is_some(), "Image control must contain embedded picture bytes");
            let decoded = c.decode_picture().expect("Image control picture must decode to BGRA");
            assert_eq!(decoded.0, 32, "Decoded flag width must be 32");
            assert_eq!(decoded.1, 32, "Decoded flag height must be 32");
            assert_eq!(decoded.2.len(), 32 * 32 * 4, "Decoded BGRA byte count must match 32x32");
        }
    }
    assert_eq!(image_count, 6, "ATM frmOpen must have 6 language flag Image controls");

    // Also verify p-code binary
    let pcode_path = if std::path::Path::new("artifacts/bin/Samples/Atm/ATM32-pcode.EXE").exists() {
        "artifacts/bin/Samples/Atm/ATM32-pcode.EXE"
    } else {
        "../../artifacts/bin/Samples/Atm/ATM32-pcode.EXE"
    };
    if std::path::Path::new(pcode_path).exists() {
        let pdata = std::fs::read(pcode_path).expect("Failed to read ATM32-pcode.EXE");
        let pmagic = pdata.windows(4).position(|w| w == b"\xff\xcc1\x00").expect("PCode Magic not found");
        let pform = FormDescriptor::parse_binary_stream(&pdata[pmagic..]).expect("Failed to parse ATM pcode stream");
        assert_eq!(pform.name, "frmOpen");
        assert_eq!(pform.controls.len(), 17);
    }
}






