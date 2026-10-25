// Clean-room re-implementation. Behavior derived from publicly documented
// RFC 6455 The WebSocket Protocol (§5 Data Framing). No Microsoft or third-party source used.

//! RFC 6455 WebSocket frame codec and streaming parser.

/// Maximum payload length allowed per frame to protect against memory exhaustion (16 MiB).
pub const MAX_PAYLOAD_LIMIT: usize = 16 * 1024 * 1024;

/// RFC 6455 WebSocket opcodes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WebSocketOpcode {
    Continuation = 0x00,
    Text = 0x01,
    Binary = 0x02,
    Close = 0x08,
    Ping = 0x09,
    Pong = 0x0A,
}

impl WebSocketOpcode {
    /// Attempts to parse an opcode from a 4-bit nibble.
    pub fn from_u8(b: u8) -> Option<Self> {
        match b & 0x0F {
            0x00 => Some(Self::Continuation),
            0x01 => Some(Self::Text),
            0x02 => Some(Self::Binary),
            0x08 => Some(Self::Close),
            0x09 => Some(Self::Ping),
            0x0A => Some(Self::Pong),
            _ => None,
        }
    }

    /// Returns `true` if this is a control frame (Close, Ping, Pong).
    pub fn is_control(&self) -> bool {
        matches!(self, Self::Close | Self::Ping | Self::Pong)
    }
}

/// A parsed RFC 6455 WebSocket frame.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WebSocketFrame {
    /// Indicates if this is the final fragment in a message.
    pub fin: bool,
    /// Frame opcode.
    pub opcode: WebSocketOpcode,
    /// 4-byte masking key if masked (mandatory for client -> server).
    pub mask: Option<[u8; 4]>,
    /// Unmasked payload data.
    pub payload: Vec<u8>,
}

impl WebSocketFrame {
    /// Creates a new unmasked binary frame (typically for server -> client).
    pub fn binary(payload: Vec<u8>) -> Self {
        Self {
            fin: true,
            opcode: WebSocketOpcode::Binary,
            mask: None,
            payload,
        }
    }

    /// Creates a new unmasked text frame (typically for server -> client).
    pub fn text(text: &str) -> Self {
        Self {
            fin: true,
            opcode: WebSocketOpcode::Text,
            mask: None,
            payload: text.as_bytes().to_vec(),
        }
    }

    /// Creates a ping frame.
    pub fn ping(payload: Vec<u8>) -> Self {
        Self {
            fin: true,
            opcode: WebSocketOpcode::Ping,
            mask: None,
            payload,
        }
    }

    /// Creates a pong frame.
    pub fn pong(payload: Vec<u8>) -> Self {
        Self {
            fin: true,
            opcode: WebSocketOpcode::Pong,
            mask: None,
            payload,
        }
    }

    /// Creates a close frame with an optional status code and reason string.
    pub fn close(status_code: Option<u16>, reason: &str) -> Self {
        let mut payload = Vec::new();
        if let Some(code) = status_code {
            payload.extend_from_slice(&code.to_be_bytes());
            payload.extend_from_slice(reason.as_bytes());
        }
        Self {
            fin: true,
            opcode: WebSocketOpcode::Close,
            mask: None,
            payload,
        }
    }

    /// Encodes this frame into wire bytes according to RFC 6455.
    pub fn encode(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(self.payload.len() + 14);

        let byte0 = (if self.fin { 0x80 } else { 0x00 }) | (self.opcode as u8);
        out.push(byte0);

        let mask_bit = if self.mask.is_some() { 0x80 } else { 0x00 };
        let len = self.payload.len();

        if len <= 125 {
            out.push(mask_bit | (len as u8));
        } else if len <= 65535 {
            out.push(mask_bit | 126);
            out.extend_from_slice(&(len as u16).to_be_bytes());
        } else {
            out.push(mask_bit | 127);
            out.extend_from_slice(&(len as u64).to_be_bytes());
        }

        if let Some(mask) = self.mask {
            out.extend_from_slice(&mask);
            for (i, &b) in self.payload.iter().enumerate() {
                out.push(b ^ mask[i % 4]);
            }
        } else {
            out.extend_from_slice(&self.payload);
        }

        out
    }

    /// Attempts to parse a single WebSocket frame from a byte slice.
    ///
    /// - `require_masked`: when `true` (server receiving from client), unmasked frames are rejected.
    /// - Returns `Ok(Some((frame, consumed_bytes)))` on complete frame.
    /// - Returns `Ok(None)` if the slice is incomplete.
    /// - Returns `Err(String)` if protocol violation or corruption occurs.
    pub fn parse(buf: &[u8], require_masked: bool) -> Result<Option<(Self, usize)>, String> {
        if buf.len() < 2 {
            return Ok(None);
        }

        let byte0 = buf[0];
        let byte1 = buf[1];

        let fin = (byte0 & 0x80) != 0;
        let rsv = byte0 & 0x70;
        if rsv != 0 {
            return Err("RSV bits must be 0 without extensions".to_string());
        }

        let opcode = WebSocketOpcode::from_u8(byte0)
            .ok_or_else(|| format!("Unknown opcode: {:#x}", byte0 & 0x0F))?;

        if opcode.is_control() && !fin {
            return Err("Control frames cannot be fragmented (FIN must be 1)".to_string());
        }

        let is_masked = (byte1 & 0x80) != 0;
        if require_masked && !is_masked {
            return Err("Client to server frames must be masked (RFC 6455 §5.1)".to_string());
        }

        let raw_len = (byte1 & 0x7F) as usize;
        let mut offset = 2;

        let payload_len: usize = if raw_len <= 125 {
            raw_len
        } else if raw_len == 126 {
            if buf.len() < offset + 2 {
                return Ok(None);
            }
            let len = u16::from_be_bytes([buf[offset], buf[offset + 1]]) as usize;
            offset += 2;
            if len < 126 {
                return Err("Non-minimal 16-bit payload length encoding".to_string());
            }
            len
        } else {
            if buf.len() < offset + 8 {
                return Ok(None);
            }
            let len64 = u64::from_be_bytes([
                buf[offset],
                buf[offset + 1],
                buf[offset + 2],
                buf[offset + 3],
                buf[offset + 4],
                buf[offset + 5],
                buf[offset + 6],
                buf[offset + 7],
            ]);
            offset += 8;
            if len64 <= 65535 {
                return Err("Non-minimal 64-bit payload length encoding".to_string());
            }
            if len64 > MAX_PAYLOAD_LIMIT as u64 {
                return Err(format!("Payload length {} exceeds limit", len64));
            }
            len64 as usize
        };

        if opcode.is_control() && payload_len > 125 {
            return Err("Control frame payload cannot exceed 125 bytes".to_string());
        }

        let mask = if is_masked {
            if buf.len() < offset + 4 {
                return Ok(None);
            }
            let m = [buf[offset], buf[offset + 1], buf[offset + 2], buf[offset + 3]];
            offset += 4;
            Some(m)
        } else {
            None
        };

        if buf.len() < offset + payload_len {
            return Ok(None);
        }

        let mut payload = vec![0u8; payload_len];
        if let Some(m) = mask {
            for (i, b) in payload.iter_mut().enumerate() {
                *b = buf[offset + i] ^ m[i % 4];
            }
        } else {
            payload.copy_from_slice(&buf[offset..offset + payload_len]);
        }

        let total_consumed = offset + payload_len;
        Ok(Some((
            Self {
                fin,
                opcode,
                mask,
                payload,
            },
            total_consumed,
        )))
    }
}

/// Helper that reassembles fragmented WebSocket frames into complete messages.
#[derive(Debug, Default)]
pub struct MessageReassembler {
    active_opcode: Option<WebSocketOpcode>,
    buffer: Vec<u8>,
}

impl MessageReassembler {
    /// Creates a new empty reassembler.
    pub fn new() -> Self {
        Self::default()
    }

    /// Feeds a parsed frame. If the frame completes a full message, returns `Ok(Some((opcode, payload)))`.
    pub fn process_frame(&mut self, frame: WebSocketFrame) -> Result<Option<(WebSocketOpcode, Vec<u8>)>, String> {
        if frame.opcode.is_control() {
            // Control frames can be interleaved between fragments
            return Ok(Some((frame.opcode, frame.payload)));
        }

        if frame.opcode == WebSocketOpcode::Continuation {
            let op = self
                .active_opcode
                .ok_or_else(|| "Unexpected continuation frame without initial fragment".to_string())?;

            if self.buffer.len() + frame.payload.len() > MAX_PAYLOAD_LIMIT {
                return Err("Reassembled message exceeds payload limit".to_string());
            }
            self.buffer.extend_from_slice(&frame.payload);

            if frame.fin {
                self.active_opcode = None;
                let payload = std::mem::take(&mut self.buffer);
                return Ok(Some((op, payload)));
            }
            return Ok(None);
        }

        // New initial data fragment
        if self.active_opcode.is_some() {
            return Err("Received new data frame before previous fragmented message completed".to_string());
        }

        if frame.fin {
            return Ok(Some((frame.opcode, frame.payload)));
        }

        self.active_opcode = Some(frame.opcode);
        self.buffer = frame.payload;
        Ok(None)
    }
}
