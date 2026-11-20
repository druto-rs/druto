//! Unit tests for magic-byte MIME type sniffing.

use bodyparser::{MimeSniffer, OCTET_STREAM_MIME};

#[test]
fn test_sniff_jpeg() {
    let data = [0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10, 0x4A, 0x46];
    assert_eq!(MimeSniffer::sniff(&data), Some("image/jpeg"));
}

#[test]
fn test_sniff_png() {
    let data = [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00];
    assert_eq!(MimeSniffer::sniff(&data), Some("image/png"));
}

#[test]
fn test_sniff_gif() {
    assert_eq!(MimeSniffer::sniff(b"GIF87a..."), Some("image/gif"));
    assert_eq!(MimeSniffer::sniff(b"GIF89a..."), Some("image/gif"));
}

#[test]
fn test_sniff_webp() {
    let mut data = b"RIFF....WEBPVP8 ".to_vec();
    assert_eq!(MimeSniffer::sniff(&data), Some("image/webp"));

    data[8] = b'X';
    assert_ne!(MimeSniffer::sniff(&data), Some("image/webp"));
}

#[test]
fn test_sniff_pdf() {
    assert_eq!(
        MimeSniffer::sniff(b"%PDF-1.7\r\n..."),
        Some("application/pdf")
    );
}

#[test]
fn test_sniff_gzip() {
    let data = [0x1F, 0x8B, 0x08, 0x00];
    assert_eq!(MimeSniffer::sniff(&data), Some("application/gzip"));
}

#[test]
fn test_sniff_zip() {
    let data = [0x50, 0x4B, 0x03, 0x04, 0x14, 0x00];
    assert_eq!(MimeSniffer::sniff(&data), Some("application/zip"));

    let empty_zip = [0x50, 0x4B, 0x05, 0x06, 0x00, 0x00];
    assert_eq!(MimeSniffer::sniff(&empty_zip), Some("application/zip"));
}

#[test]
fn test_sniff_tar() {
    let mut data = vec![0u8; 300];
    data[257..262].copy_from_slice(b"ustar");
    assert_eq!(MimeSniffer::sniff(&data), Some("application/x-tar"));
}

#[test]
fn test_sniff_mp3() {
    assert_eq!(
        MimeSniffer::sniff(b"ID3\x03\x00\x00..."),
        Some("audio/mpeg")
    );
    let raw_frame = [0xFF, 0xFB, 0x90, 0x64];
    assert_eq!(MimeSniffer::sniff(&raw_frame), Some("audio/mpeg"));
}

#[test]
fn test_sniff_mp4() {
    let data = b"....ftypisom....";
    assert_eq!(MimeSniffer::sniff(data), Some("video/mp4"));
}

#[test]
fn test_sniff_svg() {
    assert_eq!(
        MimeSniffer::sniff(b"<svg xmlns=\"http://www.w3.org/2000/svg\"></svg>"),
        Some("image/svg+xml")
    );
    assert_eq!(
        MimeSniffer::sniff(b"<?xml version=\"1.0\"?>\n<svg viewBox=\"0 0 100 100\"></svg>"),
        Some("image/svg+xml")
    );
}

#[test]
fn test_sniff_fallback_for_unknown() {
    let data = [0x01, 0x02, 0x03, 0x04, 0x05];
    assert_eq!(MimeSniffer::sniff(&data), None);
    assert_eq!(MimeSniffer::sniff_or_fallback(&data), OCTET_STREAM_MIME);
    assert_eq!(MimeSniffer::sniff_or_fallback(&[]), OCTET_STREAM_MIME);
}

#[test]
fn test_mime_for_extension() {
    assert_eq!(MimeSniffer::mime_for_extension("jpg"), Some("image/jpeg"));
    assert_eq!(MimeSniffer::mime_for_extension(".png"), Some("image/png"));
    assert_eq!(
        MimeSniffer::mime_for_extension("PDF"),
        Some("application/pdf")
    );
    assert_eq!(MimeSniffer::mime_for_extension("unknown"), None);
}

#[test]
fn test_is_compatible() {
    assert!(MimeSniffer::is_compatible("png", "image/png"));
    assert!(MimeSniffer::is_compatible("jpeg", "image/jpeg"));
    assert!(!MimeSniffer::is_compatible("png", "application/pdf"));
    // Uncataloged extensions do not contradict
    assert!(MimeSniffer::is_compatible(
        "xyz",
        "application/octet-stream"
    ));
}
