// Clean-room re-implementation. Behavior derived from publicly documented
// FIPS 180-4 and RFC 4648 specifications. No Microsoft or third-party source used.

//! Unit tests for SHA-1 and Base64 cryptographic primitives.

use cb_webserver::{sha1_digest, base64_encode, base64_decode, WS_GUID};

#[test]
fn test_sha1_fips_vectors() {
    // Vector 1: "abc"
    // Expected: a9993e36 4706816a ba3e2571 7850c26c 9cd0d89d
    let d1 = sha1_digest(b"abc");
    assert_eq!(
        hex_str(&d1),
        "a9993e364706816aba3e25717850c26c9cd0d89d"
    );

    // Vector 2: "" (empty string)
    // Expected: da39a3ee 5e6b4b0d 3255bfef 95601890 afd80709
    let d2 = sha1_digest(b"");
    assert_eq!(
        hex_str(&d2),
        "da39a3ee5e6b4b0d3255bfef95601890afd80709"
    );

    // Vector 3: "abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq"
    // Expected: 84983e44 1c3bd26e baae4aa1 f95129e5 e54670f1
    let d3 = sha1_digest(b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq");
    assert_eq!(
        hex_str(&d3),
        "84983e441c3bd26ebaae4aa1f95129e5e54670f1"
    );
}

#[test]
fn test_base64_rfc4648_test_vectors() {
    // RFC 4648 §10 Test Vectors
    assert_eq!(base64_encode(b""), "");
    assert_eq!(base64_encode(b"f"), "Zg==");
    assert_eq!(base64_encode(b"fo"), "Zm8=");
    assert_eq!(base64_encode(b"foo"), "Zm9v");
    assert_eq!(base64_encode(b"foob"), "Zm9vYg==");
    assert_eq!(base64_encode(b"fooba"), "Zm9vYmE=");
    assert_eq!(base64_encode(b"foobar"), "Zm9vYmFy");

    // Round-trip decoding
    assert_eq!(base64_decode("").unwrap(), b"");
    assert_eq!(base64_decode("Zg==").unwrap(), b"f");
    assert_eq!(base64_decode("Zm8=").unwrap(), b"fo");
    assert_eq!(base64_decode("Zm9v").unwrap(), b"foo");
    assert_eq!(base64_decode("Zm9vYg==").unwrap(), b"foob");
    assert_eq!(base64_decode("Zm9vYmE=").unwrap(), b"fooba");
    assert_eq!(base64_decode("Zm9vYmFy").unwrap(), b"foobar");

    // Invalid padding / characters
    assert!(base64_decode("Zg=").is_none());
    assert!(base64_decode("Zg==X").is_none());
    assert!(base64_decode("Z*==").is_none());
}

#[test]
fn test_rfc6455_handshake_accept_vector() {
    // RFC 6455 §1.3 example
    let client_key = "dGhlIHNhbXBsZSBub25jZQ==";
    let concat = format!("{}{}", client_key, WS_GUID);
    let digest = sha1_digest(concat.as_bytes());
    let accept_key = base64_encode(&digest);

    assert_eq!(accept_key, "s3pPLMBiTxaQ9kYGzzhZRbK+xOo=");
}

fn hex_str(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}
