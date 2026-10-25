// Clean-room re-implementation. Behavior derived from publicly documented
// RFC 6455 §5 Data Framing specifications. No Microsoft or third-party source used.

//! Unit tests for RFC 6455 WebSocket frame encoding, masking, parsing, and fragmentation.

use cb_webserver::{WebSocketFrame, WebSocketOpcode, MessageReassembler};

#[test]
fn test_unmasked_server_text_frame_encode_decode() {
    let frame = WebSocketFrame::text("Hello, WebSocket!");
    let encoded = frame.encode();

    // byte 0: FIN (0x80) | Text (0x01) = 0x81
    assert_eq!(encoded[0], 0x81);
    // byte 1: unmasked, len = 17
    assert_eq!(encoded[1], 17);

    let (parsed, consumed) = WebSocketFrame::parse(&encoded, false).unwrap().unwrap();
    assert_eq!(consumed, encoded.len());
    assert_eq!(parsed.opcode, WebSocketOpcode::Text);
    assert_eq!(String::from_utf8(parsed.payload).unwrap(), "Hello, WebSocket!");
}

#[test]
fn test_masked_client_binary_frame_roundtrip() {
    let payload = vec![1, 2, 3, 4, 5, 6, 7, 8];
    let mut frame = WebSocketFrame::binary(payload.clone());
    frame.mask = Some([0x12, 0x34, 0x56, 0x78]);

    let encoded = frame.encode();
    // byte 0: 0x82 (FIN | Binary)
    assert_eq!(encoded[0], 0x82);
    // byte 1: 0x88 (MASK | 8)
    assert_eq!(encoded[1], 0x88);

    // Parse enforcing client mask
    let (parsed, consumed) = WebSocketFrame::parse(&encoded, true).unwrap().unwrap();
    assert_eq!(consumed, encoded.len());
    assert_eq!(parsed.payload, payload);
    assert!(parsed.mask.is_some());
}

#[test]
fn test_extended_16bit_and_64bit_payload_lengths() {
    // 16-bit extended length (300 bytes)
    let p300 = vec![0xAA; 300];
    let frame16 = WebSocketFrame::binary(p300.clone());
    let enc16 = frame16.encode();
    assert_eq!(enc16[1], 126); // length indicator 126

    let (parsed16, _) = WebSocketFrame::parse(&enc16, false).unwrap().unwrap();
    assert_eq!(parsed16.payload.len(), 300);

    // 64-bit extended length (70,000 bytes)
    let p70k = vec![0xBB; 70000];
    let frame64 = WebSocketFrame::binary(p70k.clone());
    let enc64 = frame64.encode();
    assert_eq!(enc64[1], 127); // length indicator 127

    let (parsed64, _) = WebSocketFrame::parse(&enc64, false).unwrap().unwrap();
    assert_eq!(parsed64.payload.len(), 70000);
}

#[test]
fn test_unmasked_client_frame_rejected_on_server() {
    let frame = WebSocketFrame::text("illegal unmasked from client");
    let enc = frame.encode();

    // Server must reject unmasked frames with protocol error
    let err = WebSocketFrame::parse(&enc, true);
    assert!(err.is_err());
}

#[test]
fn test_message_fragmentation_reassembly() {
    let mut reassembler = MessageReassembler::new();

    // Fragment 1: FIN = false, Opcode = Text, payload = "Hello "
    let frag1 = WebSocketFrame {
        fin: false,
        opcode: WebSocketOpcode::Text,
        mask: None,
        payload: b"Hello ".to_vec(),
    };
    assert!(reassembler.process_frame(frag1).unwrap().is_none());

    // Fragment 2: FIN = true, Opcode = Continuation, payload = "World!"
    let frag2 = WebSocketFrame {
        fin: true,
        opcode: WebSocketOpcode::Continuation,
        mask: None,
        payload: b"World!".to_vec(),
    };
    let (op, full) = reassembler.process_frame(frag2).unwrap().unwrap();
    assert_eq!(op, WebSocketOpcode::Text);
    assert_eq!(String::from_utf8(full).unwrap(), "Hello World!");
}

#[test]
fn test_control_frames_ping_pong_close() {
    let ping = WebSocketFrame::ping(b"heartbeat".to_vec());
    let ping_enc = ping.encode();
    let (parsed_ping, _) = WebSocketFrame::parse(&ping_enc, false).unwrap().unwrap();
    assert_eq!(parsed_ping.opcode, WebSocketOpcode::Ping);
    assert_eq!(parsed_ping.payload, b"heartbeat");

    let close = WebSocketFrame::close(Some(1000), "Normal");
    let close_enc = close.encode();
    let (parsed_close, _) = WebSocketFrame::parse(&close_enc, false).unwrap().unwrap();
    assert_eq!(parsed_close.opcode, WebSocketOpcode::Close);
    let code = u16::from_be_bytes([parsed_close.payload[0], parsed_close.payload[1]]);
    assert_eq!(code, 1000);
}
