// Clean-room re-implementation. Behavior derived from publicly documented
// VB6 / COM / Win32 ABI semantics. No Microsoft or third-party source used.

//! Unit tests for Phase 3.3: GDI32 Text Output, Font Management & Metrics.

use cb_win32::*;

#[test]
fn test_stock_and_custom_fonts() {
    unsafe {
        let dc = gdi32_CreateCompatibleDC(0);
        assert_ne!(dc, 0);

        // Stock fonts
        let h_sys_font = gdi32_GetStockObject(SYSTEM_FONT as i32);
        assert_eq!(h_sys_font, STOCK_OBJECT_BASE + SYSTEM_FONT);

        let h_gui_font = gdi32_GetStockObject(DEFAULT_GUI_FONT as i32);
        assert_eq!(h_gui_font, STOCK_OBJECT_BASE + DEFAULT_GUI_FONT);

        // Create font via CreateFontA
        let face = b"Arial\0";
        let h_font1 = gdi32_CreateFontA(
            18, 0, 0, 0, 700, 0, 0, 0, 0, 0, 0, 0, 0, face.as_ptr() as *const i8,
        );
        assert_ne!(h_font1, 0);

        // Select font
        let old_font = gdi32_SelectObject(dc, h_font1);
        assert_ne!(old_font, 0);

        // Create font via CreateFontIndirectA
        let mut lf = LogFontA {
            lf_height: 14,
            ..Default::default()
        };
        let face2 = b"MS Sans Serif";
        lf.lf_face_name[..face2.len()].copy_from_slice(face2);
        let h_font2 = gdi32_CreateFontIndirectA(&lf);
        assert_ne!(h_font2, 0);

        // Restore and delete
        gdi32_SelectObject(dc, old_font);
        assert_eq!(gdi32_DeleteObject(h_font1), 1);
        assert_eq!(gdi32_DeleteObject(h_font2), 1);
        gdi32_DeleteDC(dc);
    }
}

#[test]
fn test_text_metrics_and_extent() {
    unsafe {
        let dc = gdi32_CreateCompatibleDC(0);
        assert_ne!(dc, 0);

        let mut tm = TextMetricA::default();
        let res = gdi32_GetTextMetricsA(dc, &mut tm);
        assert_eq!(res, 1);
        assert_eq!(tm.tm_height, 16);
        assert_eq!(tm.tm_ave_char_width, 8);

        let text = b"Hello Win32";
        let mut sz = Size::default();
        let res_sz = gdi32_GetTextExtentPoint32A(dc, text.as_ptr(), text.len() as i32, &mut sz);
        assert_eq!(res_sz, 1);
        assert_eq!(sz.cx, 11 * 8);
        assert_eq!(sz.cy, 16);

        // Empty string
        let mut sz_empty = Size::default();
        gdi32_GetTextExtentPoint32A(dc, std::ptr::null(), 0, &mut sz_empty);
        assert_eq!(sz_empty.cx, 0);
        assert_eq!(sz_empty.cy, 0);

        gdi32_DeleteDC(dc);
    }
}

#[test]
fn test_text_color_and_bkmode() {
    unsafe {
        let dc = gdi32_CreateCompatibleDC(0);
        assert_ne!(dc, 0);

        // Text Color
        gdi32_SetTextColor(dc, 0x00FF_8000);
        assert_eq!(gdi32_GetTextColor(dc), 0x00FF_8000);

        // Background Color
        gdi32_SetBkColor(dc, 0x0000_64C8);
        assert_eq!(gdi32_GetBkColor(dc), 0x0000_64C8);

        // Background Mode
        gdi32_SetBkMode(dc, TRANSPARENT);
        assert_eq!(gdi32_GetBkMode(dc), TRANSPARENT);

        gdi32_SetBkMode(dc, OPAQUE);
        assert_eq!(gdi32_GetBkMode(dc), OPAQUE);

        gdi32_DeleteDC(dc);
    }
}

#[test]
fn test_textout_and_exttextout_rendering() {
    unsafe {
        let dc = gdi32_CreateCompatibleDC(0);
        let bmp = gdi32_CreateCompatibleBitmap(dc, 64, 32);
        let old_bmp = gdi32_SelectObject(dc, bmp);

        // Black fill
        gdi32_PatBlt(dc, 0, 0, 64, 32, BLACKNESS);

        // Set white text on black background
        gdi32_SetTextColor(dc, 0x00FF_FFFF);
        gdi32_SetBkColor(dc, 0x0000_0000);
        gdi32_SetBkMode(dc, OPAQUE);

        let text = b"TEST";
        let res = gdi32_TextOutA(dc, 0, 0, text.as_ptr(), 4);
        assert_eq!(res, 1);

        // Check that white pixels exist in glyph area
        let mut found_white = false;
        for y in 0..16 {
            for x in 0..32 {
                if gdi32_GetPixel(dc, x, y) == 0x00FF_FFFF {
                    found_white = true;
                    break;
                }
            }
            if found_white {
                break;
            }
        }
        assert!(found_white, "Expected white text glyph pixels");

        // ExtTextOut with ETO_OPAQUE | ETO_CLIPPED
        gdi32_PatBlt(dc, 0, 0, 64, 32, BLACKNESS);
        gdi32_SetBkColor(dc, 0x00FF_0000); // Blue in 0x00BBGGRR (crColor = 0x00FF0000 -> blue)
        gdi32_SetTextColor(dc, 0x0000_FFFF); // Yellow in 0x00BBGGRR
        let clip_rc = Rect { left: 0, top: 0, right: 20, bottom: 16 };
        let hi_text = b"HI";
        let res_eto = gdi32_ExtTextOutA(
            dc, 0, 0, ETO_OPAQUE | ETO_CLIPPED, &clip_rc, hi_text.as_ptr(), 2, std::ptr::null(),
        );
        assert_eq!(res_eto, 1);

        // Outside rect must be unchanged (black)
        assert_eq!(gdi32_GetPixel(dc, 30, 8), 0x0000_0000);

        // Clean up
        gdi32_SelectObject(dc, old_bmp);
        gdi32_DeleteObject(bmp);
        gdi32_DeleteDC(dc);
    }
}

#[test]
fn test_drawtext_formatting_and_calcrect() {
    unsafe {
        let dc = gdi32_CreateCompatibleDC(0);
        let bmp = gdi32_CreateCompatibleBitmap(dc, 100, 50);
        let old_bmp = gdi32_SelectObject(dc, bmp);

        // DT_CALCRECT
        let mut rc_calc = Rect { left: 0, top: 0, right: 0, bottom: 0 };
        let text = b"DrawText Test";
        let h = win32_DrawTextA(dc, text.as_ptr(), text.len() as i32, &mut rc_calc, DT_CALCRECT | DT_SINGLELINE);
        assert_eq!(h, 16);
        assert_eq!(rc_calc.right, 13 * 8);
        assert_eq!(rc_calc.bottom, 16);

        // Centered draw
        let mut rc_draw = Rect { left: 0, top: 0, right: 100, bottom: 50 };
        let text2 = b"Centered";
        let h2 = win32_DrawTextA(dc, text2.as_ptr(), text2.len() as i32, &mut rc_draw, DT_CENTER | DT_VCENTER | DT_SINGLELINE);
        assert_eq!(h2, 16);

        gdi32_SelectObject(dc, old_bmp);
        gdi32_DeleteObject(bmp);
        gdi32_DeleteDC(dc);
    }
}
