// Clean-room re-implementation. Behavior derived from publicly documented
// HTML5 Canvas 2D ImageData & GDI 32-bit BGRA surface blitting semantics. No Microsoft or third-party source used.

//! Unit & Integration Tests for the Client RGBA Surface and Pixel Blitter.

use cb_client::surface::{ClientSurface, DirtyRect};

#[test]
fn test_surface_creation_and_pixel_access() {
    let mut surface = ClientSurface::new(640, 480);
    assert_eq!(surface.width(), 640);
    assert_eq!(surface.height(), 480);
    assert_eq!(surface.blitted_rects(), 0);

    // Initial state: opaque black
    let px = surface.get_pixel(0, 0).expect("Pixel (0,0)");
    assert_eq!(px, [0, 0, 0, 255]);

    // Out of bounds queries return None
    assert!(surface.get_pixel(640, 0).is_none());
    assert!(surface.get_pixel(0, 480).is_none());

    // Set pixel
    surface.set_pixel(10, 20, 255, 128, 64, 255);
    let px = surface.get_pixel(10, 20).expect("Pixel (10,20)");
    assert_eq!(px, [255, 128, 64, 255]);
}

#[test]
fn test_bgra_to_rgba_channel_swizzle() {
    let mut surface = ClientSurface::new(100, 100);
    surface.take_dirty_rect(); // Clear initial full dirty rect

    // Test 3 specific pixels:
    // Pixel 0: Pure Blue  (BGRA: 255, 0, 0, 255)   -> RGBA: (0, 0, 255, 255)
    // Pixel 1: Pure Green (BGRA: 0, 255, 0, 255)   -> RGBA: (0, 255, 0, 255)
    // Pixel 2: Pure Red   (BGRA: 0, 0, 255, 255)   -> RGBA: (255, 0, 0, 255)
    // Pixel 3: Translucent (BGRA: 10, 20, 30, 128) -> RGBA: (30, 20, 10, 128)
    let bgra_pixels = vec![
        255, 0, 0, 255,
        0, 255, 0, 255,
        0, 0, 255, 255,
        10, 20, 30, 128,
    ];

    surface.blit_bgra_rect(10, 10, 4, 1, &bgra_pixels);

    assert_eq!(surface.get_pixel(10, 10), Some([0, 0, 255, 255]));
    assert_eq!(surface.get_pixel(11, 10), Some([0, 255, 0, 255]));
    assert_eq!(surface.get_pixel(12, 10), Some([255, 0, 0, 255]));
    assert_eq!(surface.get_pixel(13, 10), Some([30, 20, 10, 128]));

    let dirty = surface.take_dirty_rect().expect("Dirty rect expected");
    assert_eq!(dirty, DirtyRect::new(10, 10, 4, 1));
}

#[test]
fn test_blitting_with_boundary_clipping() {
    let mut surface = ClientSurface::new(20, 20);
    surface.take_dirty_rect();

    // Blit a 10x10 block at (15, 15) -> Overhangs right and bottom by 5 pixels!
    // Visible region must be 5x5 at (15, 15)
    let mut bgra = vec![0u8; 10 * 10 * 4];
    for chunk in bgra.as_chunks_mut::<4>().0 {
        chunk[0] = 0;   // B
        chunk[1] = 255; // G
        chunk[2] = 0;   // R
        chunk[3] = 255; // A
    }

    surface.blit_bgra_rect(15, 15, 10, 10, &bgra);

    // Pixels inside bounds [15..20, 15..20] should be green
    assert_eq!(surface.get_pixel(15, 15), Some([0, 255, 0, 255]));
    assert_eq!(surface.get_pixel(19, 19), Some([0, 255, 0, 255]));

    let dirty = surface.take_dirty_rect().expect("Clipped dirty rect");
    assert_eq!(dirty, DirtyRect::new(15, 15, 5, 5));

    // Blit completely outside bounds: should be ignored safely
    surface.blit_bgra_rect(50, 50, 10, 10, &bgra);
    assert!(surface.take_dirty_rect().is_none());
}

#[test]
fn test_dirty_rect_union_accumulation() {
    let mut surface = ClientSurface::new(100, 100);
    surface.take_dirty_rect();

    let bgra = vec![0xFF; 4 * 4 * 4]; // 4x4 block

    // Blit 1 at (10, 10)
    surface.blit_bgra_rect(10, 10, 4, 4, &bgra);
    // Blit 2 at (40, 50)
    surface.blit_bgra_rect(40, 50, 4, 4, &bgra);

    // Expected bounding union:
    // left = min(10, 40) = 10
    // top = min(10, 50) = 10
    // right = max(14, 44) = 44
    // bottom = max(14, 54) = 54
    // width = 44 - 10 = 34, height = 54 - 10 = 44
    let dirty = surface.take_dirty_rect().expect("Accumulated dirty rect");
    assert_eq!(dirty, DirtyRect::new(10, 10, 34, 44));

    // After take_dirty_rect(), dirty tracking is reset
    assert!(surface.take_dirty_rect().is_none());
}

#[test]
fn test_surface_resize_and_content_preservation() {
    let mut surface = ClientSurface::new(10, 10);
    surface.set_pixel(2, 2, 200, 100, 50, 255);

    // Expand surface
    surface.resize(20, 20);
    assert_eq!(surface.width(), 20);
    assert_eq!(surface.height(), 20);

    // Pixel preserved
    assert_eq!(surface.get_pixel(2, 2), Some([200, 100, 50, 255]));
    // New area initialized to opaque black
    assert_eq!(surface.get_pixel(15, 15), Some([0, 0, 0, 255]));

    let dirty = surface.take_dirty_rect().expect("Resize dirty rect");
    assert_eq!(dirty, DirtyRect::new(0, 0, 20, 20));
}

#[test]
fn test_surface_clear() {
    let mut surface = ClientSurface::new(10, 10);
    surface.take_dirty_rect();

    surface.clear(128, 64, 32, 255);
    assert_eq!(surface.get_pixel(0, 0), Some([128, 64, 32, 255]));
    assert_eq!(surface.get_pixel(9, 9), Some([128, 64, 32, 255]));

    let dirty = surface.take_dirty_rect().expect("Clear dirty rect");
    assert_eq!(dirty, DirtyRect::new(0, 0, 10, 10));
}
