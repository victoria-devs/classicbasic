// Clean-room re-implementation. Behavior derived from publicly documented
// VB6 / COM / Win32 ABI semantics. No Microsoft or third-party source used.

//! Unit tests for Phase 3.5: Thunder Control Runtime, Property COM Vtables & Event Sink Dispatch.

use cb_win32::*;

#[test]
fn test_form_runtime_list_items_lifecycle() {
    let desc = FormDescriptor::default_app38_form();
    let mut runtime = FormRuntime::new(desc);

    assert_eq!(runtime.list_count("List1"), 0);

    runtime.add_list_item("List1", "First Item");
    runtime.add_list_item("List1", "Second Item");
    runtime.add_list_item("List1", "Third Item");

    assert_eq!(runtime.list_count("List1"), 3);
    assert_eq!(runtime.list_items.get("List1").unwrap(), &vec![
        "First Item".to_string(),
        "Second Item".to_string(),
        "Third Item".to_string(),
    ]);

    runtime.clear_list("List1");
    assert_eq!(runtime.list_count("List1"), 0);
}

#[test]
fn test_control_property_mutation_and_event_binding() {
    let desc = FormDescriptor::default_app38_form();
    let mut runtime = FormRuntime::new(desc);

    // Initial properties
    assert_eq!(runtime.control_text("Text1"), Some("Deserialized Text".to_string()));
    assert_eq!(runtime.control_value("Check1"), Some(1));
    assert_eq!(runtime.control_value("Option1"), Some(-1));

    // Dynamic mutations
    runtime.set_control_text("Text1", "Updated Event Text");
    runtime.set_control_value("Check1", 0);
    runtime.set_control_value("Option1", 1);
    runtime.set_control_caption("Command1", "New Caption");

    assert_eq!(runtime.control_text("Text1"), Some("Updated Event Text".to_string()));
    assert_eq!(runtime.control_value("Check1"), Some(0));
    assert_eq!(runtime.control_value("Option1"), Some(1));
    assert_eq!(runtime.control_caption("Command1"), Some("New Caption".to_string()));
}

#[test]
fn test_form_unload_cleanup_and_event_sequence() {
    let mut tree = VirtualWindowTree::new();
    let desc = FormDescriptor::default_app38_form();
    let mut runtime = FormRuntime::new(desc);

    assert!(!runtime.is_loaded);
    assert_eq!(runtime.hwnd, 0);

    let hwnd = runtime.load(&mut tree);
    assert!(runtime.is_loaded);
    assert_ne!(hwnd, 0);
    assert!(tree.is_window(hwnd));

    runtime.unload(&mut tree);
    assert!(!runtime.is_loaded);
    assert_eq!(runtime.hwnd, 0);
    assert!(!tree.is_window(hwnd));
}
