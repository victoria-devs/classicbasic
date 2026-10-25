// Clean-room re-implementation. Behavior derived from publicly documented
// VB6 / COM / Win32 ABI semantics. No Microsoft or third-party source used.

//! Win32 `USER32.DLL` Window Manager, `HWND` Hierarchy & Message Queue Foundation (Milestone 3).

use crate::handles::HandleTable;
use std::collections::{HashMap, VecDeque};
use std::ffi::CStr;
use std::sync::atomic::{AtomicBool, AtomicI32, Ordering};
use std::sync::{Condvar, Mutex, OnceLock};

/// 32-bit Window Handle (`HWND`).
pub type Hwnd = u32;

/// 32-bit Menu Handle (`HMENU`).
pub type Hmenu = u32;

/// 32-bit Cursor Handle (`HCURSOR`).
pub type Hcursor = u32;

/// 32-bit Icon Handle (`HICON`).
pub type Hicon = u32;

/// 16-bit Window Class Atom (`ATOM`).
pub type Atom = u16;

/// 2D Point structure (`POINT`).
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Point {
    pub x: i32,
    pub y: i32,
}

/// 2D Rectangle structure (`RECT`).
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Rect {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

impl Rect {
    /// Returns the rectangle's width in pixels.
    pub fn width(&self) -> i32 {
        self.right - self.left
    }

    /// Returns the rectangle's height in pixels.
    pub fn height(&self) -> i32 {
        self.bottom - self.top
    }
}

/// Standard Win32 `MSG` structure for the thread message queue.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Win32Msg {
    pub hwnd: Hwnd,
    pub message: u32,
    pub w_param: usize,
    pub l_param: usize,
    pub time: u32,
    pub pt_x: i32,
    pub pt_y: i32,
}

/// ANSI Window Class Registration Structure (`WNDCLASSA`).
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct WndClassA {
    pub style: u32,
    pub lpfnWndProc: u32,
    pub cbClsExtra: i32,
    pub cbWndExtra: i32,
    pub hInstance: u32,
    pub hIcon: u32,
    pub hCursor: u32,
    pub hbrBackground: u32,
    pub lpszMenuName: *const i8,
    pub lpszClassName: *const i8,
}

/// Wide Window Class Registration Structure (`WNDCLASSW`).
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct WndClassW {
    pub style: u32,
    pub lpfnWndProc: u32,
    pub cbClsExtra: i32,
    pub cbWndExtra: i32,
    pub hInstance: u32,
    pub hIcon: u32,
    pub hCursor: u32,
    pub hbrBackground: u32,
    pub lpszMenuName: *const u16,
    pub lpszClassName: *const u16,
}

// ---------------------------------------------------------------------------
// Win32 Window Message Constants
// ---------------------------------------------------------------------------
pub const WM_NULL: u32 = 0x0000;
pub const WM_CREATE: u32 = 0x0001;
pub const WM_DESTROY: u32 = 0x0002;
pub const WM_MOVE: u32 = 0x0003;
pub const WM_SIZE: u32 = 0x0005;
pub const WM_ACTIVATE: u32 = 0x0006;
pub const WM_SETFOCUS: u32 = 0x0007;
pub const WM_KILLFOCUS: u32 = 0x0008;
pub const WM_ENABLE: u32 = 0x000A;
pub const WM_SETREDRAW: u32 = 0x000B;
pub const WM_SETTEXT: u32 = 0x000C;
pub const WM_GETTEXT: u32 = 0x000D;
pub const WM_GETTEXTLENGTH: u32 = 0x000E;
pub const WM_PAINT: u32 = 0x000F;
pub const WM_CLOSE: u32 = 0x0010;
pub const WM_QUIT: u32 = 0x0012;
pub const WM_ERASEBKGND: u32 = 0x0014;
pub const WM_SHOWWINDOW: u32 = 0x0018;
pub const WM_KEYDOWN: u32 = 0x0100;
pub const WM_KEYUP: u32 = 0x0101;
pub const WM_CHAR: u32 = 0x0102;
pub const WM_SETCURSOR: u32 = 0x0020;
pub const WM_COMMAND: u32 = 0x0111;
pub const WM_TIMER: u32 = 0x0113;
pub const WM_MOUSEMOVE: u32 = 0x0200;
pub const WM_LBUTTONDOWN: u32 = 0x0201;
pub const WM_LBUTTONUP: u32 = 0x0202;
pub const WM_RBUTTONDOWN: u32 = 0x0204;
pub const WM_RBUTTONUP: u32 = 0x0205;
pub const WM_MBUTTONDOWN: u32 = 0x0207;
pub const WM_MBUTTONUP: u32 = 0x0208;
pub const WM_USER: u32 = 0x0400;

// ---------------------------------------------------------------------------
// ShowWindow Commands
// ---------------------------------------------------------------------------
pub const SW_HIDE: i32 = 0;
pub const SW_SHOWNORMAL: i32 = 1;
pub const SW_SHOWMINIMIZED: i32 = 2;
pub const SW_SHOWMAXIMIZED: i32 = 3;
pub const SW_SHOWNOACTIVATE: i32 = 4;
pub const SW_SHOW: i32 = 5;
pub const SW_MINIMIZE: i32 = 6;
pub const SW_SHOWMINNOACTIVE: i32 = 7;
pub const SW_SHOWNA: i32 = 8;
pub const SW_RESTORE: i32 = 9;

// ---------------------------------------------------------------------------
// PeekMessage Flags
// ---------------------------------------------------------------------------
pub const PM_NOREMOVE: u32 = 0x0000;
pub const PM_REMOVE: u32 = 0x0001;
pub const PM_NOYIELD: u32 = 0x0002;

// ---------------------------------------------------------------------------
// Window Styles
// ---------------------------------------------------------------------------
pub const WS_OVERLAPPED: u32 = 0x0000_0000;
pub const WS_POPUP: u32 = 0x8000_0000;
pub const WS_CHILD: u32 = 0x4000_0000;
pub const WS_MINIMIZE: u32 = 0x2000_0000;
pub const WS_VISIBLE: u32 = 0x1000_0000;
pub const WS_DISABLED: u32 = 0x0800_0000;
pub const WS_CLIPSIBLINGS: u32 = 0x0400_0000;
pub const WS_CLIPCHILDREN: u32 = 0x0200_0000;
pub const WS_MAXIMIZE: u32 = 0x0100_0000;
pub const WS_CAPTION: u32 = 0x00C0_0000;
pub const WS_BORDER: u32 = 0x0080_0000;
pub const WS_DLGFRAME: u32 = 0x0040_0000;
pub const WS_VSCROLL: u32 = 0x0020_0000;
pub const WS_HSCROLL: u32 = 0x0010_0000;
pub const WS_SYSMENU: u32 = 0x0008_0000;
pub const WS_THICKFRAME: u32 = 0x0004_0000;
pub const WS_GROUP: u32 = 0x0002_0000;
pub const WS_TABSTOP: u32 = 0x0001_0000;
pub const WS_MINIMIZEBOX: u32 = 0x0002_0000;
pub const WS_MAXIMIZEBOX: u32 = 0x0001_0000;
pub const WS_OVERLAPPEDWINDOW: u32 =
    WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_THICKFRAME | WS_MINIMIZEBOX | WS_MAXIMIZEBOX;

// ---------------------------------------------------------------------------
// System Metrics Constants
// ---------------------------------------------------------------------------
pub const SM_CXSCREEN: i32 = 0;
pub const SM_CYSCREEN: i32 = 1;
pub const SM_CXVSCROLL: i32 = 2;
pub const SM_CYHSCROLL: i32 = 3;
pub const SM_CYCAPTION: i32 = 4;
pub const SM_CXBORDER: i32 = 5;
pub const SM_CYBORDER: i32 = 6;
pub const SM_CXDLGFRAME: i32 = 7;
pub const SM_CYDLGFRAME: i32 = 8;
pub const SM_CYVTHUMB: i32 = 9;
pub const SM_CXHTHUMB: i32 = 10;
pub const SM_CXICON: i32 = 11;
pub const SM_CYICON: i32 = 12;
pub const SM_CXCURSOR: i32 = 13;
pub const SM_CYCURSOR: i32 = 14;
pub const SM_CYMENU: i32 = 15;
pub const SM_CXFULLSCREEN: i32 = 16;
pub const SM_CYFULLSCREEN: i32 = 17;
pub const SM_MOUSEPRESENT: i32 = 19;
pub const SM_CYVSCROLL: i32 = 20;
pub const SM_CXHSCROLL: i32 = 21;
pub const SM_CXFRAME: i32 = 32;
pub const SM_CYFRAME: i32 = 33;
pub const SM_CXEDGE: i32 = 45;
pub const SM_CYEDGE: i32 = 46;
pub const SM_CMONITORS: i32 = 80;

/// Registered Window Class descriptor.
#[derive(Debug, Clone)]
pub struct RegisteredClass {
    pub name: String,
    pub style: u32,
    pub wndproc: u32,
    pub cb_cls_extra: i32,
    pub cb_wnd_extra: i32,
    pub h_instance: u32,
    pub atom: Atom,
}

/// Virtual Window node in the window hierarchy tree.
#[derive(Debug, Clone)]
pub struct VirtualWindow {
    pub hwnd: Hwnd,
    pub class_name: String,
    pub caption: String,
    pub style: u32,
    pub ex_style: u32,
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
    pub parent: Hwnd,
    pub children: Vec<Hwnd>,
    pub visible: bool,
    pub enabled: bool,
    pub wndproc: u32,
    pub user_data: usize,
    pub h_instance: u32,
    pub h_menu: u32,
    pub surface: crate::gdi32::Surface32,
    pub invalid_rect: Option<Rect>,
}

impl VirtualWindow {
    /// Returns the bounding rectangle of the window in screen coordinates.
    pub fn window_rect(&self) -> Rect {
        Rect {
            left: self.x,
            top: self.y,
            right: self.x + self.width,
            bottom: self.y + self.height,
        }
    }

    /// Returns the client rectangle in window-relative coordinates (origin at 0, 0).
    pub fn client_rect(&self) -> Rect {
        // Standard overlapped windows reserve border and title bar space
        let (border_x, border_y, title_y) = if (self.style & WS_OVERLAPPEDWINDOW) == WS_OVERLAPPEDWINDOW {
            (4, 4, 19)
        } else if (self.style & WS_BORDER) != 0 {
            (1, 1, 0)
        } else {
            (0, 0, 0)
        };
        let client_w = (self.width - 2 * border_x).max(0);
        let client_h = (self.height - 2 * border_y - title_y).max(0);
        Rect {
            left: 0,
            top: 0,
            right: client_w,
            bottom: client_h,
        }
    }
}

/// Global Virtual Window Tree tracking all created windows, registered classes, and focus.
pub struct VirtualWindowTree {
    windows: HandleTable<VirtualWindow>,
    classes: HashMap<String, RegisteredClass>,
    next_atom: Atom,
    focus_hwnd: Option<Hwnd>,
}

impl VirtualWindowTree {
    /// Creates a new virtual window tree.
    pub fn new() -> Self {
        Self {
            windows: HandleTable::new(0x0001_0001),
            classes: HashMap::new(),
            next_atom: 0xC001,
            focus_hwnd: None,
        }
    }

    /// Registers a new window class and returns a unique class atom.
    pub fn register_class(&mut self, class: RegisteredClass) -> Atom {
        let name_key = class.name.to_lowercase();
        if let Some(existing) = self.classes.get(&name_key) {
            return existing.atom;
        }
        let atom = self.next_atom;
        self.next_atom = self.next_atom.wrapping_add(1);
        let mut entry = class;
        entry.atom = atom;
        self.classes.insert(name_key, entry);
        atom
    }

    /// Unregisters an existing window class by name.
    pub fn unregister_class(&mut self, class_name: &str) -> bool {
        self.classes.remove(&class_name.to_lowercase()).is_some()
    }

    /// Retrieves a registered class by name.
    pub fn get_class(&self, class_name: &str) -> Option<&RegisteredClass> {
        self.classes.get(&class_name.to_lowercase())
    }

    /// Creates and inserts a new window into the hierarchy.
    pub fn create_window(
        &mut self,
        ex_style: u32,
        class_name: String,
        caption: String,
        style: u32,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
        parent: Hwnd,
        h_menu: u32,
        h_instance: u32,
    ) -> Hwnd {
        let wndproc = self
            .classes
            .get(&class_name.to_lowercase())
            .map(|c| c.wndproc)
            .unwrap_or(0);
        let visible = (style & WS_VISIBLE) != 0;
        let enabled = (style & WS_DISABLED) == 0;

        let dummy = VirtualWindow {
            hwnd: 0,
            class_name: class_name.clone(),
            caption,
            style,
            ex_style,
            x,
            y,
            width,
            height,
            parent,
            children: Vec::new(),
            visible,
            enabled,
            wndproc,
            user_data: 0,
            h_instance,
            h_menu,
            surface: crate::gdi32::Surface32::new(width.max(1) as u16, height.max(1) as u16),
            invalid_rect: None,
        };

        let hwnd = self.windows.insert(dummy);
        if let Some(w) = self.windows.get_mut(hwnd) {
            w.hwnd = hwnd;
        }

        if parent != 0 {
            if let Some(p) = self.windows.get_mut(parent) {
                p.children.push(hwnd);
            }
        }

        hwnd
    }

    /// Recursively destroys a window and all of its descendants.
    pub fn destroy_window(&mut self, hwnd: Hwnd) -> bool {
        if !self.windows.contains(hwnd) {
            return false;
        }

        // Collect all descendant handles
        let mut to_destroy = Vec::new();
        let mut stack = vec![hwnd];
        while let Some(curr) = stack.pop() {
            to_destroy.push(curr);
            if let Some(node) = self.windows.get(curr) {
                for &child in &node.children {
                    stack.push(child);
                }
            }
        }

        // Unlink from parent if parent is not in to_destroy
        if let Some(node) = self.windows.get(hwnd) {
            let parent_hwnd = node.parent;
            if parent_hwnd != 0 && !to_destroy.contains(&parent_hwnd) {
                if let Some(parent) = self.windows.get_mut(parent_hwnd) {
                    parent.children.retain(|&c| c != hwnd);
                }
            }
        }

        // Clear focus if destroyed
        if let Some(f_hwnd) = self.focus_hwnd {
            if to_destroy.contains(&f_hwnd) {
                self.focus_hwnd = None;
            }
        }

        for h in to_destroy {
            self.windows.remove(h);
        }

        true
    }

    /// Checks if a window handle is valid.
    pub fn is_window(&self, hwnd: Hwnd) -> bool {
        self.windows.contains(hwnd)
    }

    /// Checks if a window is visible.
    pub fn is_window_visible(&self, hwnd: Hwnd) -> bool {
        self.windows.get(hwnd).map(|w| w.visible).unwrap_or(false)
    }

    /// Sets window visibility and returns previous visibility state.
    pub fn show_window(&mut self, hwnd: Hwnd, cmd_show: i32) -> bool {
        if let Some(w) = self.windows.get_mut(hwnd) {
            let prev = w.visible;
            w.visible = cmd_show != SW_HIDE;
            prev
        } else {
            false
        }
    }

    /// Enables or disables window mouse and keyboard input.
    pub fn enable_window(&mut self, hwnd: Hwnd, enable: bool) -> bool {
        if let Some(w) = self.windows.get_mut(hwnd) {
            let prev = !w.enabled;
            w.enabled = enable;
            prev
        } else {
            false
        }
    }

    /// Returns the window's parent handle.
    pub fn get_parent(&self, hwnd: Hwnd) -> Hwnd {
        self.windows.get(hwnd).map(|w| w.parent).unwrap_or(0)
    }

    /// Sets the text/caption of a window.
    pub fn set_window_text(&mut self, hwnd: Hwnd, text: &str) -> bool {
        if let Some(w) = self.windows.get_mut(hwnd) {
            w.caption = text.to_string();
            true
        } else {
            false
        }
    }

    /// Retrieves the text/caption of a window.
    pub fn get_window_text(&self, hwnd: Hwnd) -> Option<String> {
        self.windows.get(hwnd).map(|w| w.caption.clone())
    }

    /// Retrieves window bounding rectangle in screen coordinates.
    pub fn get_window_rect(&self, hwnd: Hwnd) -> Option<Rect> {
        self.windows.get(hwnd).map(|w| w.window_rect())
    }

    /// Retrieves window client rectangle.
    pub fn get_client_rect(&self, hwnd: Hwnd) -> Option<Rect> {
        self.windows.get(hwnd).map(|w| w.client_rect())
    }

    /// Moves and resizes a window.
    pub fn move_window(&mut self, hwnd: Hwnd, x: i32, y: i32, width: i32, height: i32) -> bool {
        if let Some(w) = self.windows.get_mut(hwnd) {
            w.x = x;
            w.y = y;
            if width > 0 && height > 0 && (width != w.width || height != w.height) {
                w.surface = crate::gdi32::Surface32::new(width as u16, height as u16);
            }
            w.width = width;
            w.height = height;
            true
        } else {
            false
        }
    }

    /// Retrieves an immutable reference to a window node.
    pub fn get_window(&self, hwnd: Hwnd) -> Option<&VirtualWindow> {
        self.windows.get(hwnd)
    }

    /// Retrieves a mutable reference to a window node.
    pub fn get_window_mut(&mut self, hwnd: Hwnd) -> Option<&mut VirtualWindow> {
        self.windows.get_mut(hwnd)
    }

    /// Sets keyboard focus to the specified window.
    pub fn set_focus(&mut self, hwnd: Hwnd) -> Option<Hwnd> {
        if self.windows.contains(hwnd) || hwnd == 0 {
            let prev = self.focus_hwnd;
            self.focus_hwnd = if hwnd == 0 { None } else { Some(hwnd) };
            prev
        } else {
            None
        }
    }

    /// Retrieves the currently focused window handle.
    pub fn get_focus(&self) -> Hwnd {
        self.focus_hwnd.unwrap_or(0)
    }

    /// Resets all windows and classes.
    pub fn clear(&mut self) {
        self.windows.clear();
        self.classes.clear();
        self.next_atom = 0xC001;
        self.focus_hwnd = None;
    }
}

impl Default for VirtualWindowTree {
    fn default() -> Self {
        Self::new()
    }
}

/// Thread-safe Virtual Message Queue backed by `Mutex<VecDeque<Win32Msg>>` and `Condvar`.
pub struct VirtualMessageQueue {
    queue: Mutex<VecDeque<Win32Msg>>,
    condvar: Condvar,
    quit_requested: AtomicBool,
    quit_exit_code: AtomicI32,
}

impl VirtualMessageQueue {
    /// Creates a new virtual message queue.
    pub fn new() -> Self {
        Self {
            queue: Mutex::new(VecDeque::new()),
            condvar: Condvar::new(),
            quit_requested: AtomicBool::new(false),
            quit_exit_code: AtomicI32::new(0),
        }
    }

    /// Posts a message asynchronously and wakes any thread waiting on `GetMessageA`.
    pub fn post_message(&self, msg: Win32Msg) {
        let mut q = self.queue.lock().unwrap();
        q.push_back(msg);
        self.condvar.notify_one();
    }

    /// Signals application quit (`WM_QUIT`) with an exit code.
    pub fn post_quit_message(&self, exit_code: i32) {
        self.quit_exit_code.store(exit_code, Ordering::SeqCst);
        self.quit_requested.store(true, Ordering::SeqCst);
        let mut q = self.queue.lock().unwrap();
        q.push_back(Win32Msg {
            hwnd: 0,
            message: WM_QUIT,
            w_param: exit_code as usize,
            l_param: 0,
            time: 0,
            pt_x: 0,
            pt_y: 0,
        });
        self.condvar.notify_all();
    }

    /// Inspects or retrieves a message non-blockingly (`PeekMessageA`).
    pub fn peek_message(
        &self,
        target_hwnd: Hwnd,
        msg_min: u32,
        msg_max: u32,
        remove: bool,
    ) -> Option<Win32Msg> {
        let mut q = self.queue.lock().unwrap();
        let idx = q.iter().position(|m| {
            let hwnd_match = target_hwnd == 0 || m.hwnd == target_hwnd;
            let range_match = (msg_min == 0 && msg_max == 0)
                || (m.message >= msg_min && m.message <= msg_max);
            hwnd_match && range_match
        })?;

        if remove {
            q.remove(idx)
        } else {
            q.get(idx).copied()
        }
    }

    /// Retrieves a message, blocking on the `Condvar` until a message arrives (`GetMessageA`).
    /// Returns `Some(msg)` if a message was retrieved, or `None` if `WM_QUIT` was received.
    pub fn get_message(
        &self,
        target_hwnd: Hwnd,
        msg_min: u32,
        msg_max: u32,
    ) -> Result<Win32Msg, i32> {
        let mut q = self.queue.lock().unwrap();
        loop {
            let idx = q.iter().position(|m| {
                let hwnd_match = target_hwnd == 0 || m.hwnd == target_hwnd;
                let range_match = (msg_min == 0 && msg_max == 0)
                    || (m.message >= msg_min && m.message <= msg_max);
                hwnd_match && range_match
            });

            if let Some(i) = idx {
                let msg = q.remove(i).unwrap();
                if msg.message == WM_QUIT {
                    return Err(msg.w_param as i32);
                }
                return Ok(msg);
            }

            // Block waiting for new messages
            q = self.condvar.wait(q).unwrap();
        }
    }

    /// Clears the message queue and resets quit flags.
    pub fn clear(&self) {
        let mut q = self.queue.lock().unwrap();
        q.clear();
        self.quit_requested.store(false, Ordering::SeqCst);
        self.quit_exit_code.store(0, Ordering::SeqCst);
    }
}

impl Default for VirtualMessageQueue {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Global Subsystem Instances
// ---------------------------------------------------------------------------
static GLOBAL_WINDOW_TREE: OnceLock<Mutex<VirtualWindowTree>> = OnceLock::new();
static GLOBAL_MESSAGE_QUEUE: OnceLock<VirtualMessageQueue> = OnceLock::new();

/// Returns a reference to the global `VirtualWindowTree`.
pub fn window_tree() -> &'static Mutex<VirtualWindowTree> {
    GLOBAL_WINDOW_TREE.get_or_init(|| Mutex::new(VirtualWindowTree::new()))
}

/// Returns a reference to the global `VirtualMessageQueue`.
pub fn message_queue() -> &'static VirtualMessageQueue {
    GLOBAL_MESSAGE_QUEUE.get_or_init(VirtualMessageQueue::new)
}

/// Resets both the window tree and message queue for testing or session termination.
pub fn reset_window_subsystem() {
    window_tree().lock().unwrap().clear();
    message_queue().clear();
}

// ---------------------------------------------------------------------------
// High-Level Win32 API Implementation Functions
// ---------------------------------------------------------------------------

/// Queries system metrics (`GetSystemMetrics`).
pub fn user32_get_system_metrics(n_index: i32) -> i32 {
    match n_index {
        SM_CXSCREEN => 1920,
        SM_CYSCREEN => 1080,
        SM_CXVSCROLL => 16,
        SM_CYHSCROLL => 16,
        SM_CYCAPTION => 19,
        SM_CXBORDER => 1,
        SM_CYBORDER => 1,
        SM_CXDLGFRAME => 3,
        SM_CYDLGFRAME => 3,
        SM_CYVTHUMB => 16,
        SM_CXHTHUMB => 16,
        SM_CXICON => 32,
        SM_CYICON => 32,
        SM_CXCURSOR => 32,
        SM_CYCURSOR => 32,
        SM_CYMENU => 19,
        SM_CXFULLSCREEN => 1920,
        SM_CYFULLSCREEN => 1061,
        SM_MOUSEPRESENT => 1,
        SM_CYVSCROLL => 16,
        SM_CXHSCROLL => 16,
        SM_CXFRAME => 4,
        SM_CYFRAME => 4,
        SM_CXEDGE => 2,
        SM_CYEDGE => 2,
        SM_CMONITORS => 1,
        _ => 0,
    }
}

/// Default window procedure (`DefWindowProcA`).
pub fn user32_def_window_proc(hwnd: Hwnd, msg: u32, w_param: usize, l_param: usize) -> i32 {
    match msg {
        WM_CLOSE => {
            window_tree().lock().unwrap().destroy_window(hwnd);
            0
        }
        WM_DESTROY => 0,
        _ => {
            let _ = (w_param, l_param);
            0
        }
    }
}

/// Synchronous message dispatch (`SendMessageA`).
pub fn user32_send_message(hwnd: Hwnd, msg: u32, w_param: usize, l_param: usize) -> i32 {
    match msg {
        WM_SETTEXT => {
            // If l_param is a pointer to ANSI string
            if l_param != 0 {
                let cstr = unsafe { CStr::from_ptr(l_param as *const i8) };
                let s = cstr.to_string_lossy();
                window_tree().lock().unwrap().set_window_text(hwnd, &s);
            }
            1
        }
        WM_GETTEXT => {
            if l_param != 0 && w_param > 0 {
                if let Some(text) = window_tree().lock().unwrap().get_window_text(hwnd) {
                    let bytes = text.as_bytes();
                    let copy_len = (w_param - 1).min(bytes.len());
                    unsafe {
                        std::ptr::copy_nonoverlapping(bytes.as_ptr(), l_param as *mut u8, copy_len);
                        *((l_param as *mut u8).add(copy_len)) = 0;
                    }
                    copy_len as i32
                } else {
                    0
                }
            } else {
                0
            }
        }
        WM_GETTEXTLENGTH => {
            window_tree()
                .lock()
                .unwrap()
                .get_window_text(hwnd)
                .map(|s| s.len() as i32)
                .unwrap_or(0)
        }
        _ => user32_def_window_proc(hwnd, msg, w_param, l_param),
    }
}

// ---------------------------------------------------------------------------
// C ABI Exported Functions (`extern "system"`)
// ---------------------------------------------------------------------------

#[no_mangle]
pub unsafe extern "system" fn win32_GetSystemMetrics(n_index: i32) -> i32 {
    user32_get_system_metrics(n_index)
}

#[no_mangle]
pub unsafe extern "system" fn win32_RegisterClassA(lp_wnd_class: *const WndClassA) -> u16 {
    if lp_wnd_class.is_null() {
        return 0;
    }
    let wc = &*lp_wnd_class;
    let class_name = if !wc.lpszClassName.is_null() {
        CStr::from_ptr(wc.lpszClassName).to_string_lossy().to_string()
    } else {
        return 0;
    };

    let reg = RegisteredClass {
        name: class_name,
        style: wc.style,
        wndproc: wc.lpfnWndProc,
        cb_cls_extra: wc.cbClsExtra,
        cb_wnd_extra: wc.cbWndExtra,
        h_instance: wc.hInstance,
        atom: 0,
    };

    window_tree().lock().unwrap().register_class(reg)
}

#[no_mangle]
pub unsafe extern "system" fn win32_RegisterClassW(lp_wnd_class: *const WndClassW) -> u16 {
    if lp_wnd_class.is_null() {
        return 0;
    }
    let wc = &*lp_wnd_class;
    let mut class_name = String::new();
    if !wc.lpszClassName.is_null() {
        let mut ptr = wc.lpszClassName;
        while *ptr != 0 {
            class_name.push(*ptr as u8 as char);
            ptr = ptr.add(1);
        }
    } else {
        return 0;
    }

    let reg = RegisteredClass {
        name: class_name,
        style: wc.style,
        wndproc: wc.lpfnWndProc,
        cb_cls_extra: wc.cbClsExtra,
        cb_wnd_extra: wc.cbWndExtra,
        h_instance: wc.hInstance,
        atom: 0,
    };

    window_tree().lock().unwrap().register_class(reg)
}

#[no_mangle]
pub unsafe extern "system" fn win32_UnregisterClassA(
    lp_class_name: *const i8,
    _h_instance: u32,
) -> i32 {
    if lp_class_name.is_null() {
        return 0;
    }
    let name = CStr::from_ptr(lp_class_name).to_string_lossy();
    if window_tree().lock().unwrap().unregister_class(&name) {
        1
    } else {
        0
    }
}

#[no_mangle]
pub unsafe extern "system" fn win32_CreateWindowExA(
    dw_ex_style: u32,
    lp_class_name: *const i8,
    lp_window_name: *const i8,
    dw_style: u32,
    x: i32,
    y: i32,
    n_width: i32,
    n_height: i32,
    h_wnd_parent: u32,
    h_menu: u32,
    h_instance: u32,
    _lp_param: *mut core::ffi::c_void,
) -> u32 {
    let class_name = if !lp_class_name.is_null() {
        if (lp_class_name as usize) <= 0xFFFF {
            format!("#{}", lp_class_name as usize)
        } else {
            CStr::from_ptr(lp_class_name).to_string_lossy().to_string()
        }
    } else {
        "DefaultClass".to_string()
    };

    let title = if !lp_window_name.is_null() {
        CStr::from_ptr(lp_window_name).to_string_lossy().to_string()
    } else {
        String::new()
    };

    window_tree().lock().unwrap().create_window(
        dw_ex_style,
        class_name,
        title,
        dw_style,
        x,
        y,
        n_width,
        n_height,
        h_wnd_parent,
        h_menu,
        h_instance,
    )
}

#[no_mangle]
pub unsafe extern "system" fn win32_CreateWindowExW(
    dw_ex_style: u32,
    lp_class_name: *const u16,
    lp_window_name: *const u16,
    dw_style: u32,
    x: i32,
    y: i32,
    n_width: i32,
    n_height: i32,
    h_wnd_parent: u32,
    h_menu: u32,
    h_instance: u32,
    _lp_param: *mut core::ffi::c_void,
) -> u32 {
    let mut class_name = String::new();
    if !lp_class_name.is_null() {
        let mut ptr = lp_class_name;
        while *ptr != 0 {
            class_name.push(*ptr as u8 as char);
            ptr = ptr.add(1);
        }
    }
    let mut title = String::new();
    if !lp_window_name.is_null() {
        let mut ptr = lp_window_name;
        while *ptr != 0 {
            title.push(*ptr as u8 as char);
            ptr = ptr.add(1);
        }
    }

    window_tree().lock().unwrap().create_window(
        dw_ex_style,
        class_name,
        title,
        dw_style,
        x,
        y,
        n_width,
        n_height,
        h_wnd_parent,
        h_menu,
        h_instance,
    )
}

#[no_mangle]
pub unsafe extern "system" fn win32_DestroyWindow(hwnd: u32) -> i32 {
    if window_tree().lock().unwrap().destroy_window(hwnd) {
        1
    } else {
        0
    }
}

#[no_mangle]
pub unsafe extern "system" fn win32_ShowWindow(hwnd: u32, n_cmd_show: i32) -> i32 {
    if window_tree().lock().unwrap().show_window(hwnd, n_cmd_show) {
        1
    } else {
        0
    }
}

#[no_mangle]
pub unsafe extern "system" fn win32_UpdateWindow(_hwnd: u32) -> i32 {
    1
}

#[no_mangle]
pub unsafe extern "system" fn win32_MoveWindow(
    hwnd: u32,
    x: i32,
    y: i32,
    n_width: i32,
    n_height: i32,
    _b_repaint: i32,
) -> i32 {
    if window_tree().lock().unwrap().move_window(hwnd, x, y, n_width, n_height) {
        1
    } else {
        0
    }
}

#[no_mangle]
pub unsafe extern "system" fn win32_SetWindowPos(
    hwnd: u32,
    _h_wnd_insert_after: u32,
    x: i32,
    y: i32,
    cx: i32,
    cy: i32,
    _u_flags: u32,
) -> i32 {
    if window_tree().lock().unwrap().move_window(hwnd, x, y, cx, cy) {
        1
    } else {
        0
    }
}

#[no_mangle]
pub unsafe extern "system" fn win32_SetWindowTextA(hwnd: u32, lp_string: *const i8) -> i32 {
    if lp_string.is_null() {
        return 0;
    }
    let text = CStr::from_ptr(lp_string).to_string_lossy();
    if window_tree().lock().unwrap().set_window_text(hwnd, &text) {
        1
    } else {
        0
    }
}

#[no_mangle]
pub unsafe extern "system" fn win32_GetWindowTextA(
    hwnd: u32,
    lp_string: *mut i8,
    n_max_count: i32,
) -> i32 {
    if lp_string.is_null() || n_max_count <= 0 {
        return 0;
    }
    if let Some(text) = window_tree().lock().unwrap().get_window_text(hwnd) {
        let bytes = text.as_bytes();
        let copy_len = (n_max_count as usize - 1).min(bytes.len());
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), lp_string as *mut u8, copy_len);
        *((lp_string as *mut u8).add(copy_len)) = 0;
        copy_len as i32
    } else {
        0
    }
}

#[no_mangle]
pub unsafe extern "system" fn win32_GetWindowTextLengthA(hwnd: u32) -> i32 {
    window_tree()
        .lock()
        .unwrap()
        .get_window_text(hwnd)
        .map(|s| s.len() as i32)
        .unwrap_or(0)
}

#[no_mangle]
pub unsafe extern "system" fn win32_GetClientRect(hwnd: u32, lp_rect: *mut Rect) -> i32 {
    if lp_rect.is_null() {
        return 0;
    }
    if let Some(rect) = window_tree().lock().unwrap().get_client_rect(hwnd) {
        *lp_rect = rect;
        1
    } else {
        0
    }
}

#[no_mangle]
pub unsafe extern "system" fn win32_GetWindowRect(hwnd: u32, lp_rect: *mut Rect) -> i32 {
    if lp_rect.is_null() {
        return 0;
    }
    if let Some(rect) = window_tree().lock().unwrap().get_window_rect(hwnd) {
        *lp_rect = rect;
        1
    } else {
        0
    }
}

#[no_mangle]
pub unsafe extern "system" fn win32_IsWindow(hwnd: u32) -> i32 {
    if window_tree().lock().unwrap().is_window(hwnd) {
        1
    } else {
        0
    }
}

#[no_mangle]
pub unsafe extern "system" fn win32_IsWindowVisible(hwnd: u32) -> i32 {
    if window_tree().lock().unwrap().is_window_visible(hwnd) {
        1
    } else {
        0
    }
}

#[no_mangle]
pub unsafe extern "system" fn win32_EnableWindow(hwnd: u32, b_enable: i32) -> i32 {
    if window_tree().lock().unwrap().enable_window(hwnd, b_enable != 0) {
        1
    } else {
        0
    }
}

#[no_mangle]
pub unsafe extern "system" fn win32_GetParent(hwnd: u32) -> u32 {
    window_tree().lock().unwrap().get_parent(hwnd)
}

#[no_mangle]
pub unsafe extern "system" fn win32_SendMessageA(
    hwnd: u32,
    msg: u32,
    w_param: usize,
    l_param: usize,
) -> i32 {
    user32_send_message(hwnd, msg, w_param, l_param)
}

#[no_mangle]
pub unsafe extern "system" fn win32_PostMessageA(
    hwnd: u32,
    msg: u32,
    w_param: usize,
    l_param: usize,
) -> i32 {
    let m = Win32Msg {
        hwnd,
        message: msg,
        w_param,
        l_param,
        time: 0,
        pt_x: 0,
        pt_y: 0,
    };
    message_queue().post_message(m);
    1
}

#[no_mangle]
pub unsafe extern "system" fn win32_PeekMessageA(
    lp_msg: *mut Win32Msg,
    hwnd: u32,
    w_msg_filter_min: u32,
    w_msg_filter_max: u32,
    w_remove_msg: u32,
) -> i32 {
    let remove = (w_remove_msg & PM_REMOVE) != 0;
    if let Some(msg) = message_queue().peek_message(hwnd, w_msg_filter_min, w_msg_filter_max, remove) {
        if !lp_msg.is_null() {
            *lp_msg = msg;
        }
        1
    } else {
        0
    }
}

#[no_mangle]
pub unsafe extern "system" fn win32_GetMessageA(
    lp_msg: *mut Win32Msg,
    hwnd: u32,
    w_msg_filter_min: u32,
    w_msg_filter_max: u32,
) -> i32 {
    match message_queue().get_message(hwnd, w_msg_filter_min, w_msg_filter_max) {
        Ok(msg) => {
            if !lp_msg.is_null() {
                *lp_msg = msg;
            }
            1
        }
        Err(exit_code) => {
            if !lp_msg.is_null() {
                *lp_msg = Win32Msg {
                    hwnd: 0,
                    message: WM_QUIT,
                    w_param: exit_code as usize,
                    l_param: 0,
                    time: 0,
                    pt_x: 0,
                    pt_y: 0,
                };
            }
            0
        }
    }
}

#[no_mangle]
pub unsafe extern "system" fn win32_PostQuitMessage(n_exit_code: i32) {
    message_queue().post_quit_message(n_exit_code);
}

#[no_mangle]
pub unsafe extern "system" fn win32_DefWindowProcA(
    hwnd: u32,
    msg: u32,
    w_param: usize,
    l_param: usize,
) -> i32 {
    user32_def_window_proc(hwnd, msg, w_param, l_param)
}

#[no_mangle]
pub unsafe extern "system" fn win32_SetFocus(hwnd: u32) -> u32 {
    window_tree().lock().unwrap().set_focus(hwnd).unwrap_or(0)
}

#[no_mangle]
pub unsafe extern "system" fn win32_GetFocus() -> u32 {
    window_tree().lock().unwrap().get_focus()
}

#[no_mangle]
pub unsafe extern "system" fn win32_TranslateMessage(_lp_msg: *const Win32Msg) -> i32 {
    0
}

#[no_mangle]
pub unsafe extern "system" fn win32_DispatchMessageA(lp_msg: *const Win32Msg) -> i32 {
    if lp_msg.is_null() {
        return 0;
    }
    let msg = &*lp_msg;
    user32_send_message(msg.hwnd, msg.message, msg.w_param, msg.l_param)
}

/// Standard Win32 `PAINTSTRUCT` structure.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct PaintStruct {
    pub hdc: u32,
    pub f_erase: i32,
    pub rc_paint: Rect,
    pub f_restore: i32,
    pub f_inc_update: i32,
    pub rgb_reserved: [u8; 32],
}

impl Default for PaintStruct {
    fn default() -> Self {
        Self {
            hdc: 0,
            f_erase: 0,
            rc_paint: Rect::default(),
            f_restore: 0,
            f_inc_update: 0,
            rgb_reserved: [0u8; 32],
        }
    }
}

#[no_mangle]
pub unsafe extern "system" fn win32_GetDC(hwnd: u32) -> u32 {
    let mut state = crate::gdi32::gdi_state().lock().unwrap();
    let dc = crate::gdi32::VirtualDC {
        hdc: 0,
        hwnd: Some(hwnd),
        selected_pen: crate::gdi32::STOCK_OBJECT_BASE + crate::gdi32::BLACK_PEN,
        selected_brush: crate::gdi32::STOCK_OBJECT_BASE + crate::gdi32::WHITE_BRUSH,
        selected_bitmap: 0,
        selected_font: crate::gdi32::STOCK_OBJECT_BASE + crate::font::SYSTEM_FONT,
        text_color: 0x0000_0000,
        bk_color: 0x00FF_FFFF,
        bk_mode: crate::font::OPAQUE,
        pen_pos: (0, 0),
        mem_surface: None,
    };
    let new_hdc = state.dcs.insert(dc);
    if let Some(d) = state.dcs.get_mut(new_hdc) {
        d.hdc = new_hdc;
    }
    new_hdc
}

#[no_mangle]
pub unsafe extern "system" fn win32_ReleaseDC(_hwnd: u32, hdc: u32) -> i32 {
    let mut state = crate::gdi32::gdi_state().lock().unwrap();
    if state.dcs.remove(hdc).is_some() { 1 } else { 0 }
}

#[no_mangle]
pub unsafe extern "system" fn win32_InvalidateRect(
    hwnd: u32,
    lp_rect: *const Rect,
    _b_erase: i32,
) -> i32 {
    let mut tree = window_tree().lock().unwrap();
    if let Some(w) = tree.get_window_mut(hwnd) {
        let inv = if !lp_rect.is_null() {
            *lp_rect
        } else {
            w.client_rect()
        };
        w.invalid_rect = Some(inv);
        1
    } else {
        0
    }
}

#[no_mangle]
pub unsafe extern "system" fn win32_BeginPaint(hwnd: u32, lp_paint: *mut PaintStruct) -> u32 {
    let mut tree = window_tree().lock().unwrap();
    if let Some(w) = tree.get_window_mut(hwnd) {
        let rc = w.invalid_rect.unwrap_or_else(|| w.client_rect());
        w.invalid_rect = None;
        drop(tree);

        let hdc = win32_GetDC(hwnd);
        if !lp_paint.is_null() {
            (*lp_paint).hdc = hdc;
            (*lp_paint).f_erase = 1;
            (*lp_paint).rc_paint = rc;
        }
        hdc
    } else {
        0
    }
}

#[no_mangle]
pub unsafe extern "system" fn win32_EndPaint(_hwnd: u32, lp_paint: *const PaintStruct) -> i32 {
    if !lp_paint.is_null() {
        win32_ReleaseDC(_hwnd, (*lp_paint).hdc);
    }
    1
}

#[no_mangle]
pub unsafe extern "system" fn win32_FillRect(hdc: u32, lp_rect: *const Rect, h_brush: u32) -> i32 {
    if lp_rect.is_null() {
        return 0;
    }
    let rect = *lp_rect;
    let brush_color = {
        let state = crate::gdi32::gdi_state().lock().unwrap();
        match state.get_object(h_brush) {
            Some(crate::gdi32::GdiObject::Brush { color, .. }) => color,
            _ => 0x00FF_FFFF,
        }
    };

    let (hwnd, selected_bitmap) = {
        let state = crate::gdi32::gdi_state().lock().unwrap();
        if let Some(dc) = state.dcs.get(hdc) {
            (dc.hwnd, dc.selected_bitmap)
        } else {
            return 0;
        }
    };

    if let Some(h) = hwnd {
        if h != 0 {
            let mut tree = window_tree().lock().unwrap();
            if let Some(win) = tree.get_window_mut(h) {
                win.surface.fill_rect(rect.left, rect.top, rect.right, rect.bottom, brush_color);
                return 1;
            }
        }
    }

    let mut state = crate::gdi32::gdi_state().lock().unwrap();
    if selected_bitmap != 0 {
        if let Some(crate::gdi32::GdiObject::Bitmap { surface, .. }) = state.objects.get_mut(selected_bitmap) {
            surface.fill_rect(rect.left, rect.top, rect.right, rect.bottom, brush_color);
            return 1;
        }
    }
    if let Some(dc) = state.dcs.get_mut(hdc) {
        if let Some(s) = &mut dc.mem_surface {
            s.fill_rect(rect.left, rect.top, rect.right, rect.bottom, brush_color);
            return 1;
        }
    }
    1
}

#[no_mangle]
pub unsafe extern "system" fn win32_FrameRect(hdc: u32, lp_rect: *const Rect, h_brush: u32) -> i32 {
    if lp_rect.is_null() {
        return 0;
    }
    let rect = *lp_rect;
    let brush_color = {
        let state = crate::gdi32::gdi_state().lock().unwrap();
        match state.get_object(h_brush) {
            Some(crate::gdi32::GdiObject::Brush { color, .. }) => color,
            _ => 0x0000_0000,
        }
    };

    let (hwnd, selected_bitmap) = {
        let state = crate::gdi32::gdi_state().lock().unwrap();
        if let Some(dc) = state.dcs.get(hdc) {
            (dc.hwnd, dc.selected_bitmap)
        } else {
            return 0;
        }
    };

    if let Some(h) = hwnd {
        if h != 0 {
            let mut tree = window_tree().lock().unwrap();
            if let Some(win) = tree.get_window_mut(h) {
                win.surface.frame_rect(rect.left, rect.top, rect.right, rect.bottom, brush_color);
                return 1;
            }
        }
    }

    let mut state = crate::gdi32::gdi_state().lock().unwrap();
    if selected_bitmap != 0 {
        if let Some(crate::gdi32::GdiObject::Bitmap { surface, .. }) = state.objects.get_mut(selected_bitmap) {
            surface.frame_rect(rect.left, rect.top, rect.right, rect.bottom, brush_color);
            return 1;
        }
    }
    if let Some(dc) = state.dcs.get_mut(hdc) {
        if let Some(s) = &mut dc.mem_surface {
            s.frame_rect(rect.left, rect.top, rect.right, rect.bottom, brush_color);
            return 1;
        }
    }
    1
}

#[no_mangle]
pub unsafe extern "system" fn win32_DrawTextA(
    hdc: u32,
    lpch_text: *const u8,
    n_count: i32,
    lp_rect: *mut Rect,
    u_format: u32,
) -> i32 {
    if lp_rect.is_null() {
        return 0;
    }
    let rect = &mut *lp_rect;
    let text = if n_count == -1 {
        if lpch_text.is_null() {
            String::new()
        } else {
            CStr::from_ptr(lpch_text as *const i8).to_string_lossy().to_string()
        }
    } else if n_count > 0 && !lpch_text.is_null() {
        let slice = std::slice::from_raw_parts(lpch_text, n_count as usize);
        String::from_utf8_lossy(slice).to_string()
    } else {
        String::new()
    };

    let text_len = text.len() as i32;
    let text_width = text_len * 8;
    let text_height = 16;

    let is_calc_rect = (u_format & crate::font::DT_CALCRECT) != 0;
    if is_calc_rect {
        rect.right = rect.left + text_width;
        rect.bottom = rect.top + text_height;
        return text_height;
    }

    // Alignment
    let x = if (u_format & crate::font::DT_CENTER) != 0 {
        rect.left + (rect.width() - text_width) / 2
    } else if (u_format & crate::font::DT_RIGHT) != 0 {
        rect.right - text_width
    } else {
        rect.left
    };

    let y = if (u_format & crate::font::DT_VCENTER) != 0 {
        rect.top + (rect.height() - text_height) / 2
    } else if (u_format & crate::font::DT_BOTTOM) != 0 {
        rect.bottom - text_height
    } else {
        rect.top
    };

    let clip_rect = if (u_format & crate::font::DT_NOCLIP) != 0 {
        None
    } else {
        Some(*rect)
    };

    let (hwnd, selected_bitmap, text_color, bk_color, bk_mode) = {
        let state = crate::gdi32::gdi_state().lock().unwrap();
        if let Some(dc) = state.dcs.get(hdc) {
            (dc.hwnd, dc.selected_bitmap, dc.text_color, dc.bk_color, dc.bk_mode)
        } else {
            return 0;
        }
    };

    let render_fn = |surf: &mut crate::gdi32::Surface32| {
        surf.draw_text(x, y, &text, text_color, bk_color, bk_mode, clip_rect.as_ref());
    };

    if let Some(h) = hwnd {
        if h != 0 {
            let mut tree = window_tree().lock().unwrap();
            if let Some(win) = tree.get_window_mut(h) {
                render_fn(&mut win.surface);
                return text_height;
            }
        }
    }

    let mut state = crate::gdi32::gdi_state().lock().unwrap();
    if selected_bitmap != 0 {
        if let Some(crate::gdi32::GdiObject::Bitmap { surface, .. }) = state.objects.get_mut(selected_bitmap) {
            render_fn(surface);
            return text_height;
        }
    }
    if let Some(dc) = state.dcs.get_mut(hdc) {
        if let Some(s) = &mut dc.mem_surface {
            render_fn(s);
            return text_height;
        }
    }
    text_height
}

#[no_mangle]
pub unsafe extern "system" fn win32_ValidateRect(
    hwnd: u32,
    _lp_rect: *const Rect,
) -> i32 {
    let mut tree = window_tree().lock().unwrap();
    if let Some(w) = tree.get_window_mut(hwnd) {
        w.invalid_rect = None;
        1
    } else {
        0
    }
}

#[no_mangle]
pub unsafe extern "system" fn win32_GetUpdateRect(
    hwnd: u32,
    lp_rect: *mut Rect,
    _b_erase: i32,
) -> i32 {
    let tree = window_tree().lock().unwrap();
    if let Some(w) = tree.get_window(hwnd) {
        if let Some(rc) = w.invalid_rect {
            if !lp_rect.is_null() {
                *lp_rect = rc;
            }
            1
        } else {
            if !lp_rect.is_null() {
                *lp_rect = Rect { left: 0, top: 0, right: 0, bottom: 0 };
            }
            0
        }
    } else {
        0
    }
}

static ACTIVE_CURSOR: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(32512); // IDC_ARROW
static CURSOR_POS_X: std::sync::atomic::AtomicI32 = std::sync::atomic::AtomicI32::new(0);
static CURSOR_POS_Y: std::sync::atomic::AtomicI32 = std::sync::atomic::AtomicI32::new(0);

#[no_mangle]
pub unsafe extern "system" fn win32_LoadCursorA(_h_instance: u32, lp_cursor_name: usize) -> u32 {
    lp_cursor_name as u32
}

#[no_mangle]
pub unsafe extern "system" fn win32_SetCursor(h_cursor: u32) -> u32 {
    ACTIVE_CURSOR.swap(h_cursor, std::sync::atomic::Ordering::SeqCst)
}

#[no_mangle]
pub unsafe extern "system" fn win32_GetCursor() -> u32 {
    ACTIVE_CURSOR.load(std::sync::atomic::Ordering::SeqCst)
}

#[no_mangle]
pub unsafe extern "system" fn win32_SetCursorPos(x: i32, y: i32) -> i32 {
    CURSOR_POS_X.store(x, std::sync::atomic::Ordering::SeqCst);
    CURSOR_POS_Y.store(y, std::sync::atomic::Ordering::SeqCst);
    1
}

#[no_mangle]
pub unsafe extern "system" fn win32_GetCursorPos(lp_point: *mut Point) -> i32 {
    if !lp_point.is_null() {
        (*lp_point).x = CURSOR_POS_X.load(std::sync::atomic::Ordering::SeqCst);
        (*lp_point).y = CURSOR_POS_Y.load(std::sync::atomic::Ordering::SeqCst);
        1
    } else {
        0
    }
}

static CLIPBOARD_TEXT: Mutex<Option<String>> = Mutex::new(None);

pub fn clipboard_clear() {
    let mut cb = CLIPBOARD_TEXT.lock().unwrap();
    *cb = None;
}

pub fn clipboard_set_text(text: &str) {
    let mut cb = CLIPBOARD_TEXT.lock().unwrap();
    *cb = Some(text.to_string());
}

pub fn clipboard_get_text() -> String {
    let cb = CLIPBOARD_TEXT.lock().unwrap();
    cb.clone().unwrap_or_default()
}

pub fn clipboard_get_format(format: u16) -> bool {
    let cb = CLIPBOARD_TEXT.lock().unwrap();
    format == 1 && cb.is_some()
}

