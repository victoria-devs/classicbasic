// Clean-room re-implementation. Behavior derived from publicly documented
// RFC 4648 The Base16, Base32, and Base64 Data Encodings. No Microsoft or third-party source used.

//! RFC 4648 Standard Base64 encoder and decoder with `=` padding.

const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

/// Encodes binary data into standard RFC 4648 Base64 with padding.
pub fn base64_encode(data: &[u8]) -> String {
    let mut out = String::with_capacity(data.len().div_ceil(3) * 4);
    for chunk in data.chunks(3) {
        let b0 = chunk[0];
        let b1 = if chunk.len() > 1 { chunk[1] } else { 0 };
        let b2 = if chunk.len() > 2 { chunk[2] } else { 0 };

        let idx0 = (b0 >> 2) as usize;
        let idx1 = (((b0 & 0x03) << 4) | (b1 >> 4)) as usize;
        let idx2 = (((b1 & 0x0F) << 2) | (b2 >> 6)) as usize;
        let idx3 = (b2 & 0x3F) as usize;

        out.push(ALPHABET[idx0] as char);
        out.push(ALPHABET[idx1] as char);
        if chunk.len() > 1 {
            out.push(ALPHABET[idx2] as char);
        } else {
            out.push('=');
        }
        if chunk.len() > 2 {
            out.push(ALPHABET[idx3] as char);
        } else {
            out.push('=');
        }
    }
    out
}

/// Decodes standard RFC 4648 Base64 string into binary data. Returns `None` if invalid characters or invalid padding.
pub fn base64_decode(input: &str) -> Option<Vec<u8>> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Some(Vec::new());
    }
    if trimmed.len() % 4 != 0 {
        return None;
    }

    let decode_char = |c: u8| -> Option<u8> {
        match c {
            b'A'..=b'Z' => Some(c - b'A'),
            b'a'..=b'z' => Some(c - b'a' + 26),
            b'0'..=b'9' => Some(c - b'0' + 52),
            b'+' => Some(62),
            b'/' => Some(63),
            _ => None,
        }
    };

    let bytes = trimmed.as_bytes();
    let mut out = Vec::with_capacity((bytes.len() / 4) * 3);

    for chunk in bytes.as_chunks::<4>().0 {
        let c0 = chunk[0];
        let c1 = chunk[1];
        let c2 = chunk[2];
        let c3 = chunk[3];

        let d0 = decode_char(c0)?;
        let d1 = decode_char(c1)?;

        let b0 = (d0 << 2) | (d1 >> 4);
        out.push(b0);

        if c2 == b'=' {
            if c3 != b'=' {
                return None; // Invalid padding pattern: '=X'
            }
            // Check trailing bits in d1
            if (d1 & 0x0F) != 0 {
                return None;
            }
            break;
        }

        let d2 = decode_char(c2)?;
        let b1 = ((d1 & 0x0F) << 4) | (d2 >> 2);
        out.push(b1);

        if c3 == b'=' {
            // Check trailing bits in d2
            if (d2 & 0x03) != 0 {
                return None;
            }
            break;
        }

        let d3 = decode_char(c3)?;
        let b2 = ((d2 & 0x03) << 6) | d3;
        out.push(b2);
    }

    Some(out)
}
