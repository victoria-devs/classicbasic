// Clean-room re-implementation. Behavior derived from publicly documented
// HTML5 Canvas 2D text rendering & Win32 GDI font metrics semantics. No Microsoft or third-party source used.

//! Unit & Integration Tests for the Delegated Text Pipeline in `cb-client`.

use cb_client::text::{TextDrawCommand, TextQueue};

#[test]
fn test_text_command_color_and_font_formatting() {
    // Win32 COLORREF 0x00FF8800 -> R=0x00, G=0x88, B=0xFF
    let cmd1 = TextDrawCommand::new(
        100,
        10,
        20,
        0x00FF8800,
        1, // TRANSPARENT
        -16, // 16px
        "Hello World".to_string(),
    );

    assert_eq!(cmd1.color_css, "rgb(0, 136, 255)");
    assert_eq!(cmd1.font_css, "16px sans-serif");
    assert_eq!(cmd1.text, "Hello World");

    // Pure Red COLORREF 0x000000FF -> R=255, G=0, B=0
    let cmd2 = TextDrawCommand::new(
        100,
        0,
        0,
        0x000000FF,
        2, // OPAQUE
        12, // 12px
        "ClassicBasic".to_string(),
    );
    assert_eq!(cmd2.color_css, "rgb(255, 0, 0)");
    assert_eq!(cmd2.font_css, "12px sans-serif");

    // Zero font height defaults to 13px
    let cmd3 = TextDrawCommand::new(100, 0, 0, 0, 1, 0, "Default".to_string());
    assert_eq!(cmd3.font_css, "13px sans-serif");
}

#[test]
fn test_text_queue_fifo_lifecycle() {
    let mut queue = TextQueue::new();
    assert!(queue.is_empty());
    assert_eq!(queue.len(), 0);

    let cmd1 = TextDrawCommand::new(1, 0, 0, 0xFF, 1, -14, "One".to_string());
    let cmd2 = TextDrawCommand::new(1, 10, 10, 0xFF00, 1, -14, "Two".to_string());
    let cmd3 = TextDrawCommand::new(1, 20, 20, 0xFF0000, 1, -14, "Three".to_string());

    queue.push(cmd1.clone());
    queue.push(cmd2.clone());
    queue.push(cmd3.clone());

    assert_eq!(queue.len(), 3);
    assert_eq!(queue.peek()[0].text, "One");
    assert_eq!(queue.peek()[1].text, "Two");
    assert_eq!(queue.peek()[2].text, "Three");

    // Drain consumes all commands in FIFO order
    let drained = queue.drain();
    assert_eq!(drained.len(), 3);
    assert_eq!(drained[0], cmd1);
    assert_eq!(drained[1], cmd2);
    assert_eq!(drained[2], cmd3);

    assert!(queue.is_empty());
}
