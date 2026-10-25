// Clean-room re-implementation. Behavior derived from publicly documented
// VB6 / COM / Win32 ABI semantics. No Microsoft or third-party source used.

//! Unit & Integration Tests for Phase 3.1:
//! - 32-bit Win32 Handle Table (`HandleTable<T>`)
//! - Virtual Window Tree (`VirtualWindowTree`, `VirtualWindow`)
//! - Thread-safe `Condvar` Message Queue (`VirtualMessageQueue`)
//! - `USER32.DLL` High-Level and C ABI Exports

use cb_win32::handles::HandleTable;
use cb_win32::user32::*;
use std::ffi::CString;
use std::thread;
use std::time::Duration;

#[test]
fn test_handle_table_allocations() {
    let mut table: HandleTable<String> = HandleTable::new(0x0001_0000);
    assert!(table.is_empty());
    assert_eq!(table.len(), 0);

    let h1 = table.insert("Window1".to_string());
    let h2 = table.insert("Window2".to_string());
    let h3 = table.insert("Window3".to_string());

    assert_eq!(h1, 0x0001_0000);
    assert_eq!(h2, 0x0001_0001);
    assert_eq!(h3, 0x0001_0002);
    assert_eq!(table.len(), 3);

    assert_eq!(table.get(h1).map(|s| s.as_str()), Some("Window1"));
    assert_eq!(table.get(h2).map(|s| s.as_str()), Some("Window2"));
    assert_eq!(table.get(h3).map(|s| s.as_str()), Some("Window3"));
    assert_eq!(table.get(0x9999), None);

    assert!(table.contains(h2));
    assert_eq!(table.remove(h2), Some("Window2".to_string()));
    assert!(!table.contains(h2));
    assert_eq!(table.len(), 2);

    table.clear();
    assert!(table.is_empty());
}

#[test]
fn test_system_metrics() {
    assert_eq!(user32_get_system_metrics(SM_CXSCREEN), 1920);
    assert_eq!(user32_get_system_metrics(SM_CYSCREEN), 1080);
    assert_eq!(user32_get_system_metrics(SM_CXBORDER), 1);
    assert_eq!(user32_get_system_metrics(SM_CYBORDER), 1);
    assert_eq!(user32_get_system_metrics(SM_CXFRAME), 4);
    assert_eq!(user32_get_system_metrics(SM_CYFRAME), 4);
    assert_eq!(user32_get_system_metrics(SM_CYCAPTION), 19);
    assert_eq!(user32_get_system_metrics(SM_CMONITORS), 1);
    assert_eq!(user32_get_system_metrics(9999), 0);
}

#[test]
fn test_window_class_lifecycle() {
    reset_window_subsystem();

    let reg = RegisteredClass {
        name: "TestButtonClass".to_string(),
        style: 0,
        wndproc: 0x401000,
        cb_cls_extra: 0,
        cb_wnd_extra: 0,
        h_instance: 0x00400000,
        atom: 0,
    };

    let atom1 = window_tree().lock().unwrap().register_class(reg);
    assert!(atom1 >= 0xC000);

    // Registering again with same class name returns the existing atom
    let reg2 = RegisteredClass {
        name: "testbuttonclass".to_string(),
        style: 0,
        wndproc: 0x401000,
        cb_cls_extra: 0,
        cb_wnd_extra: 0,
        h_instance: 0x00400000,
        atom: 0,
    };
    let atom2 = window_tree().lock().unwrap().register_class(reg2);
    assert_eq!(atom1, atom2);

    let cls = window_tree().lock().unwrap().get_class("TestButtonClass").cloned();
    assert!(cls.is_some());
    assert_eq!(cls.unwrap().wndproc, 0x401000);

    let unregistered = window_tree().lock().unwrap().unregister_class("TestButtonClass");
    assert!(unregistered);
    assert!(window_tree().lock().unwrap().get_class("TestButtonClass").is_none());
}

#[test]
fn test_window_tree_hierarchy_and_lifecycle() {
    reset_window_subsystem();

    // 1. Create root window
    let h_root = window_tree().lock().unwrap().create_window(
        0,
        "MainWindowClass".to_string(),
        "ClassicBasic Main Form".to_string(),
        WS_OVERLAPPEDWINDOW | WS_VISIBLE,
        100,
        100,
        800,
        600,
        0,
        0,
        0x00400000,
    );

    assert_ne!(h_root, 0);
    assert!(window_tree().lock().unwrap().is_window(h_root));
    assert!(window_tree().lock().unwrap().is_window_visible(h_root));
    assert_eq!(window_tree().lock().unwrap().get_parent(h_root), 0);

    // 2. Create child control
    let h_child1 = window_tree().lock().unwrap().create_window(
        0,
        "ButtonClass".to_string(),
        "Click Me".to_string(),
        WS_CHILD | WS_VISIBLE,
        20,
        20,
        120,
        35,
        h_root,
        1001,
        0x00400000,
    );

    assert_ne!(h_child1, 0);
    assert!(window_tree().lock().unwrap().is_window(h_child1));
    assert_eq!(window_tree().lock().unwrap().get_parent(h_child1), h_root);

    // 3. Create sub-child control
    let h_subchild = window_tree().lock().unwrap().create_window(
        0,
        "IconClass".to_string(),
        "".to_string(),
        WS_CHILD | WS_VISIBLE,
        2,
        2,
        16,
        16,
        h_child1,
        1002,
        0x00400000,
    );

    assert_eq!(window_tree().lock().unwrap().get_parent(h_subchild), h_child1);

    // 4. Destroy parent window: should recursively destroy h_child1 and h_subchild
    let destroyed = window_tree().lock().unwrap().destroy_window(h_root);
    assert!(destroyed);

    assert!(!window_tree().lock().unwrap().is_window(h_root));
    assert!(!window_tree().lock().unwrap().is_window(h_child1));
    assert!(!window_tree().lock().unwrap().is_window(h_subchild));
}

#[test]
fn test_window_geometry_text_and_visibility() {
    reset_window_subsystem();

    let hwnd = window_tree().lock().unwrap().create_window(
        0,
        "FormClass".to_string(),
        "Initial Title".to_string(),
        WS_OVERLAPPEDWINDOW,
        50,
        50,
        640,
        480,
        0,
        0,
        0x00400000,
    );

    // Text queries
    assert_eq!(
        window_tree().lock().unwrap().get_window_text(hwnd),
        Some("Initial Title".to_string())
    );
    assert!(window_tree().lock().unwrap().set_window_text(hwnd, "Updated Title"));
    assert_eq!(
        window_tree().lock().unwrap().get_window_text(hwnd),
        Some("Updated Title".to_string())
    );

    // Rect queries
    let rc_win = window_tree().lock().unwrap().get_window_rect(hwnd).unwrap();
    assert_eq!(rc_win.left, 50);
    assert_eq!(rc_win.top, 50);
    assert_eq!(rc_win.width(), 640);
    assert_eq!(rc_win.height(), 480);

    let rc_client = window_tree().lock().unwrap().get_client_rect(hwnd).unwrap();
    assert_eq!(rc_client.left, 0);
    assert_eq!(rc_client.top, 0);
    assert!(rc_client.width() > 0 && rc_client.width() < 640);
    assert!(rc_client.height() > 0 && rc_client.height() < 480);

    // Move window
    assert!(window_tree().lock().unwrap().move_window(hwnd, 150, 200, 800, 600));
    let rc_moved = window_tree().lock().unwrap().get_window_rect(hwnd).unwrap();
    assert_eq!(rc_moved.left, 150);
    assert_eq!(rc_moved.top, 200);
    assert_eq!(rc_moved.width(), 800);
    assert_eq!(rc_moved.height(), 600);

    // Visibility
    assert!(!window_tree().lock().unwrap().is_window_visible(hwnd));
    assert!(!window_tree().lock().unwrap().show_window(hwnd, SW_SHOW)); // prev was hidden
    assert!(window_tree().lock().unwrap().is_window_visible(hwnd));
    assert!(window_tree().lock().unwrap().show_window(hwnd, SW_HIDE)); // prev was visible
    assert!(!window_tree().lock().unwrap().is_window_visible(hwnd));

    // Enablement: returns true if previously disabled, false if previously enabled
    assert!(!window_tree().lock().unwrap().enable_window(hwnd, false)); // was enabled, disable it -> returns false
    assert!(window_tree().lock().unwrap().enable_window(hwnd, true)); // was disabled, enable it -> returns true

    // Focus
    assert_eq!(window_tree().lock().unwrap().get_focus(), 0);
    window_tree().lock().unwrap().set_focus(hwnd);
    assert_eq!(window_tree().lock().unwrap().get_focus(), hwnd);

    window_tree().lock().unwrap().destroy_window(hwnd);
    assert_eq!(window_tree().lock().unwrap().get_focus(), 0);
}

#[test]
fn test_synchronous_message_dispatch() {
    reset_window_subsystem();

    let hwnd = window_tree().lock().unwrap().create_window(
        0,
        "FormClass".to_string(),
        "Dispatch Test".to_string(),
        WS_OVERLAPPEDWINDOW,
        0,
        0,
        400,
        300,
        0,
        0,
        0x00400000,
    );

    // WM_GETTEXTLENGTH
    let len = user32_send_message(hwnd, WM_GETTEXTLENGTH, 0, 0);
    assert_eq!(len, 13); // "Dispatch Test".len()

    // WM_SETTEXT via SendMessageA
    let new_title = CString::new("Synchronous Update").unwrap();
    let res = user32_send_message(hwnd, WM_SETTEXT, 0, new_title.as_ptr() as usize);
    assert_eq!(res, 1);
    assert_eq!(
        window_tree().lock().unwrap().get_window_text(hwnd),
        Some("Synchronous Update".to_string())
    );

    // WM_GETTEXT via SendMessageA
    let mut buf = [0u8; 64];
    let copied = user32_send_message(hwnd, WM_GETTEXT, 64, buf.as_mut_ptr() as usize);
    assert_eq!(copied, 18);
    let s = std::str::from_utf8(&buf[..copied as usize]).unwrap();
    assert_eq!(s, "Synchronous Update");

    // DefWindowProc with custom message
    let def_res = user32_def_window_proc(hwnd, WM_USER + 10, 1, 2);
    assert_eq!(def_res, 0);
}

#[test]
fn test_message_queue_and_condvar_pump() {
    reset_window_subsystem();

    let queue = message_queue();
    queue.clear();

    // 1. Post message and peek without removal
    queue.post_message(Win32Msg {
        hwnd: 0x00010001,
        message: WM_USER + 1,
        w_param: 42,
        l_param: 84,
        time: 100,
        pt_x: 10,
        pt_y: 20,
    });

    let peek1 = queue.peek_message(0x00010001, 0, 0, false);
    assert!(peek1.is_some());
    assert_eq!(peek1.unwrap().message, WM_USER + 1);

    // Peek again with removal
    let peek2 = queue.peek_message(0x00010001, 0, 0, true);
    assert!(peek2.is_some());
    assert_eq!(peek2.unwrap().message, WM_USER + 1);

    // Queue now empty
    assert!(queue.peek_message(0x00010001, 0, 0, false).is_none());

    // 2. Multithreaded Condvar blocking pump
    let handle = thread::spawn(|| {
        thread::sleep(Duration::from_millis(50));
        message_queue().post_message(Win32Msg {
            hwnd: 0x00010001,
            message: WM_USER + 99,
            w_param: 123,
            l_param: 456,
            time: 0,
            pt_x: 0,
            pt_y: 0,
        });
    });

    let got = queue.get_message(0x00010001, 0, 0);
    assert!(got.is_ok());
    let m = got.unwrap();
    assert_eq!(m.message, WM_USER + 99);
    assert_eq!(m.w_param, 123);
    assert_eq!(m.l_param, 456);
    handle.join().unwrap();

    // 3. PostQuitMessage terminates GetMessage loop with 0
    queue.post_quit_message(42);
    let quit_res = queue.get_message(0, 0, 0);
    assert_eq!(quit_res, Err(42));
}

#[test]
fn test_c_abi_shims_end_to_end() {
    reset_window_subsystem();

    unsafe {
        // RegisterClassA
        let class_name = CString::new("CAbiClass").unwrap();
        let wc = WndClassA {
            style: 0,
            lpfnWndProc: 0,
            cbClsExtra: 0,
            cbWndExtra: 0,
            hInstance: 0x00400000,
            hIcon: 0,
            hCursor: 0,
            hbrBackground: 0,
            lpszMenuName: core::ptr::null(),
            lpszClassName: class_name.as_ptr(),
        };

        let atom = win32_RegisterClassA(&wc);
        assert!(atom >= 0xC000);

        // CreateWindowExA
        let title = CString::new("C ABI Window").unwrap();
        let hwnd = win32_CreateWindowExA(
            0,
            class_name.as_ptr(),
            title.as_ptr(),
            WS_OVERLAPPEDWINDOW,
            100,
            100,
            640,
            480,
            0,
            0,
            0x00400000,
            core::ptr::null_mut(),
        );

        assert_ne!(hwnd, 0);
        assert_eq!(win32_IsWindow(hwnd), 1);

        // GetWindowTextA
        let mut buf = [0i8; 64];
        let len = win32_GetWindowTextA(hwnd, buf.as_mut_ptr(), 64);
        assert_eq!(len, 12);
        let read_str = std::ffi::CStr::from_ptr(buf.as_ptr()).to_str().unwrap();
        assert_eq!(read_str, "C ABI Window");

        // Rects
        let mut rc = Rect::default();
        assert_eq!(win32_GetWindowRect(hwnd, &mut rc), 1);
        assert_eq!(rc.width(), 640);
        assert_eq!(rc.height(), 480);

        // Visibility
        win32_ShowWindow(hwnd, SW_SHOW);
        assert_eq!(win32_IsWindowVisible(hwnd), 1);

        // Message pump via C ABI
        assert_eq!(win32_PostMessageA(hwnd, WM_USER + 5, 10, 20), 1);
        let mut msg = Win32Msg::default();
        assert_eq!(win32_PeekMessageA(&mut msg, hwnd, 0, 0, PM_REMOVE), 1);
        assert_eq!(msg.message, WM_USER + 5);
        assert_eq!(msg.w_param, 10);
        assert_eq!(msg.l_param, 20);

        // PostQuitMessage and GetMessageA
        win32_PostQuitMessage(0);
        assert_eq!(win32_GetMessageA(&mut msg, 0, 0, 0), 0);
        assert_eq!(msg.message, WM_QUIT);

        // DestroyWindow
        assert_eq!(win32_DestroyWindow(hwnd), 1);
        assert_eq!(win32_IsWindow(hwnd), 0);
    }
}
