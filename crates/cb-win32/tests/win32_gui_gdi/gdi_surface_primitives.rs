// Clean-room re-implementation. Behavior derived from publicly documented
// VB6 / COM / Win32 ABI semantics. No Microsoft or third-party source used.

//! Unit & Integration Tests for Phase 3.2: GDI32 32-Bit Surface, Vector Primitives & Dirty Rects.

use cb_win32::*;

#[test]
fn test_surface32_pixel_read_write_and_dirty_rects() {
    let mut surface = Surface32::new(100, 100);
    assert_eq!(surface.width, 100);
    assert_eq!(surface.height, 100);
    assert_eq!(surface.get_pixel(10, 10), 0);

    // Color: RGB(255, 128, 64) -> 0x004080FF
    let color = 0x0040_80FF;
    assert!(surface.set_pixel(10, 10, color));
    assert_eq!(surface.get_pixel(10, 10), color);
    assert_eq!(surface.dirty_regions.len(), 1);
    assert_eq!(
        surface.dirty_regions[0],
        DirtyRect {
            x: 10,
            y: 10,
            width: 1,
            height: 1,
        }
    );

    // Out of bounds returns CLR_INVALID (0xFFFFFFFF)
    assert!(!surface.set_pixel(150, 10, color));
    assert_eq!(surface.get_pixel(150, 10), 0xFFFF_FFFF);
}

#[test]
fn test_gdi_stock_objects_lifecycle() {
    unsafe {
        let white_brush = gdi32_GetStockObject(WHITE_BRUSH as i32);
        assert_ne!(white_brush, 0);

        let black_pen = gdi32_GetStockObject(BLACK_PEN as i32);
        assert_ne!(black_pen, 0);

        let null_brush = gdi32_GetStockObject(NULL_BRUSH as i32);
        assert_ne!(null_brush, 0);

        // DeleteObject on stock objects must return 1 without deleting
        assert_eq!(gdi32_DeleteObject(white_brush), 1);
        assert_eq!(gdi32_DeleteObject(black_pen), 1);
    }
}

#[test]
fn test_gdi_solid_pens_and_brushes() {
    unsafe {
        let red = 0x0000_00FF; // RGB(255, 0, 0) in COLORREF
        let green = 0x0000_FF00; // RGB(0, 255, 0)

        let h_pen = gdi32_CreatePen(PS_SOLID as i32, 1, red);
        assert_ne!(h_pen, 0);

        let h_brush = gdi32_CreateSolidBrush(green);
        assert_ne!(h_brush, 0);

        let mem_dc = gdi32_CreateCompatibleDC(0);
        assert_ne!(mem_dc, 0);

        let old_pen = gdi32_SelectObject(mem_dc, h_pen);
        assert_ne!(old_pen, 0);

        let old_brush = gdi32_SelectObject(mem_dc, h_brush);
        assert_ne!(old_brush, 0);

        // Restore & Delete
        gdi32_SelectObject(mem_dc, old_pen);
        gdi32_SelectObject(mem_dc, old_brush);
        assert_eq!(gdi32_DeleteObject(h_pen), 1);
        assert_eq!(gdi32_DeleteObject(h_brush), 1);
        assert_eq!(gdi32_DeleteDC(mem_dc), 1);
    }
}

#[test]
fn test_gdi_vector_drawing_primitives() {
    let _lock = crate::TEST_MUTEX.lock().unwrap();
    unsafe {
        let hwnd = {
            let mut tree = window_tree().lock().unwrap();
            tree.create_window(
                0,
                "GdiTestClassVector".to_string(),
                "Gdi Test Window".to_string(),
                WS_OVERLAPPEDWINDOW,
                0,
                0,
                200,
                200,
                0,
                0,
                0x400000,
            )
        };
        assert_ne!(hwnd, 0);

        let hdc = win32_GetDC(hwnd);
        assert_ne!(hdc, 0);

        let red_pen = gdi32_CreatePen(PS_SOLID as i32, 1, 0x0000_00FF);
        let green_brush = gdi32_CreateSolidBrush(0x0000_FF00);
        gdi32_SelectObject(hdc, red_pen);
        gdi32_SelectObject(hdc, green_brush);

        // 1. LineTo from (10, 10) to (30, 10)
        let mut pt = Point::default();
        assert_eq!(gdi32_MoveToEx(hdc, 10, 10, &mut pt), 1);
        assert_eq!(gdi32_LineTo(hdc, 30, 10), 1);
        let pixel_line = gdi32_GetPixel(hdc, 20, 10);
        assert_eq!(pixel_line, 0x0000_00FF);

        // 2. Rectangle [40, 40] to [80, 80]
        assert_eq!(gdi32_Rectangle(hdc, 40, 40, 80, 80), 1);
        assert_eq!(gdi32_GetPixel(hdc, 60, 60), 0x0000_FF00); // Interior
        assert_eq!(gdi32_GetPixel(hdc, 40, 40), 0x0000_00FF); // Border

        // 3. Ellipse [100, 100] to [140, 140]
        assert_eq!(gdi32_Ellipse(hdc, 100, 100, 140, 140), 1);
        assert_eq!(gdi32_GetPixel(hdc, 120, 120), 0x0000_FF00); // Center

        // Clean up
        gdi32_DeleteObject(red_pen);
        gdi32_DeleteObject(green_brush);
        win32_ReleaseDC(hwnd, hdc);
        window_tree().lock().unwrap().destroy_window(hwnd);
    }
}

#[test]
fn test_gdi_region_fills_and_framing() {
    unsafe {
        let mem_dc = gdi32_CreateCompatibleDC(0);
        let bmp = gdi32_CreateCompatibleBitmap(mem_dc, 100, 100);
        gdi32_SelectObject(mem_dc, bmp);

        let blue_brush = gdi32_CreateSolidBrush(0x00FF_0000); // RGB(0, 0, 255)
        let rc_fill = Rect {
            left: 10,
            top: 10,
            right: 50,
            bottom: 50,
        };
        assert_eq!(win32_FillRect(mem_dc, &rc_fill, blue_brush), 1);
        assert_eq!(gdi32_GetPixel(mem_dc, 25, 25), 0x00FF_0000);

        let yellow_brush = gdi32_CreateSolidBrush(0x0000_FFFF); // RGB(255, 255, 0)
        let rc_frame = Rect {
            left: 60,
            top: 60,
            right: 100,
            bottom: 100,
        };
        assert_eq!(win32_FrameRect(mem_dc, &rc_frame, yellow_brush), 1);
        assert_eq!(gdi32_GetPixel(mem_dc, 60, 60), 0x0000_FFFF);

        gdi32_DeleteObject(blue_brush);
        gdi32_DeleteObject(yellow_brush);
        gdi32_DeleteObject(bmp);
        gdi32_DeleteDC(mem_dc);
    }
}

#[test]
fn test_gdi_bitmaps_and_raster_ops() {
    unsafe {
        let mem_dc = gdi32_CreateCompatibleDC(0);
        let bmp = gdi32_CreateCompatibleBitmap(mem_dc, 32, 32);
        gdi32_SelectObject(mem_dc, bmp);

        // PatBlt with WHITENESS
        assert_eq!(gdi32_PatBlt(mem_dc, 0, 0, 32, 32, WHITENESS), 1);
        assert_eq!(gdi32_GetPixel(mem_dc, 0, 0), 0x00FF_FFFF);

        // Set pixel on mem_dc
        assert_eq!(gdi32_SetPixel(mem_dc, 5, 5, 0x0080_00FF), 0x0080_00FF);

        // Destination DC
        let dst_dc = gdi32_CreateCompatibleDC(0);
        let dst_bmp = gdi32_CreateCompatibleBitmap(dst_dc, 32, 32);
        gdi32_SelectObject(dst_dc, dst_bmp);

        // BitBlt SRCCOPY
        assert_eq!(gdi32_BitBlt(dst_dc, 0, 0, 32, 32, mem_dc, 0, 0, SRCCOPY), 1);
        assert_eq!(gdi32_GetPixel(dst_dc, 5, 5), 0x0080_00FF);

        gdi32_DeleteObject(bmp);
        gdi32_DeleteObject(dst_bmp);
        gdi32_DeleteDC(mem_dc);
        gdi32_DeleteDC(dst_dc);
    }
}

#[test]
fn test_window_paint_lifecycle_and_invalidation() {
    let _lock = crate::TEST_MUTEX.lock().unwrap();
    unsafe {
        let mut tree = window_tree().lock().unwrap();
        let hwnd = tree.create_window(
            0,
            "GdiPaintTest".to_string(),
            "Paint Window".to_string(),
            WS_OVERLAPPEDWINDOW,
            0,
            0,
            200,
            200,
            0,
            0,
            0x400000,
        );
        drop(tree);

        let rc_inv = Rect {
            left: 0,
            top: 0,
            right: 100,
            bottom: 100,
        };
        assert_eq!(win32_InvalidateRect(hwnd, &rc_inv, 1), 1);

        let mut ps = PaintStruct::default();
        let paint_dc = win32_BeginPaint(hwnd, &mut ps);
        assert_ne!(paint_dc, 0);
        assert!(ps.rc_paint.right >= 100);
        assert!(ps.rc_paint.bottom >= 100);

        assert_eq!(win32_EndPaint(hwnd, &ps), 1);
        window_tree().lock().unwrap().destroy_window(hwnd);
    }
}
