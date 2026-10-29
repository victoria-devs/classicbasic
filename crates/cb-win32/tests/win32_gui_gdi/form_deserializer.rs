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

