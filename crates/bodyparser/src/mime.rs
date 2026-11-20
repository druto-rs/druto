//! Magic-byte MIME type sniffing and content signature verification.

/// Octet stream fallback MIME type when content matches no recognized signature.
pub const OCTET_STREAM_MIME: &str = "application/octet-stream";

/// Inspects byte signatures to determine media types independently of client declarations.
pub struct MimeSniffer;

impl MimeSniffer {
    /// Inspects the initial slice of bytes (typically 512 bytes) and returns the detected MIME type if recognized.
    #[must_use]
    pub fn sniff(bytes: &[u8]) -> Option<&'static str> {
        if bytes.is_empty() {
            return None;
        }

        // JPEG: FF D8 FF
        if bytes.len() >= 3 && bytes[0] == 0xFF && bytes[1] == 0xD8 && bytes[2] == 0xFF {
            return Some("image/jpeg");
        }

        // PNG: 89 50 4E 47 0D 0A 1A 0A
        if bytes.len() >= 8 && bytes.starts_with(&[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A])
        {
            return Some("image/png");
        }

        // GIF: GIF87a or GIF89a
        if bytes.len() >= 6 && (bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a")) {
            return Some("image/gif");
        }

        // WebP: RIFF....WEBP
        if bytes.len() >= 12 && bytes.starts_with(b"RIFF") && &bytes[8..12] == b"WEBP" {
            return Some("image/webp");
        }

        // PDF: %PDF-
        if bytes.len() >= 5 && bytes.starts_with(b"%PDF-") {
            return Some("application/pdf");
        }

        // GZIP: 1F 8B
        if bytes.len() >= 2 && bytes[0] == 0x1F && bytes[1] == 0x8B {
            return Some("application/gzip");
        }

        // ZIP: PK\x03\x04 or PK\x05\x06 (empty archive) or PK\x07\x08
        if bytes.len() >= 4
            && bytes[0] == 0x50
            && bytes[1] == 0x4B
            && (bytes[2] == 0x03 && bytes[3] == 0x04
                || bytes[2] == 0x05 && bytes[3] == 0x06
                || bytes[2] == 0x07 && bytes[3] == 0x08)
        {
            return Some("application/zip");
        }

        // TAR: "ustar" at offset 257
        if bytes.len() >= 262 && &bytes[257..262] == b"ustar" {
            return Some("application/x-tar");
        }

        // MP3 with ID3v2 header: ID3
        if bytes.len() >= 3 && bytes.starts_with(b"ID3") {
            return Some("audio/mpeg");
        }

        // MP3 raw sync frame: FF FB or FF F3 or FF F2
        if bytes.len() >= 2
            && bytes[0] == 0xFF
            && (bytes[1] & 0xFE == 0xFA || bytes[1] & 0xFE == 0xF2)
        {
            return Some("audio/mpeg");
        }

        // MP4 / QuickTime: "ftyp" at offset 4
        if bytes.len() >= 8 && &bytes[4..8] == b"ftyp" {
            return Some("video/mp4");
        }

        // SVG: XML declaration or <svg opening tag within initial buffer
        if let Ok(text) = std::str::from_utf8(&bytes[..bytes.len().min(512)]) {
            let trimmed = text.trim_start();
            if trimmed.starts_with("<svg")
                || (trimmed.starts_with("<?xml") && trimmed.contains("<svg"))
            {
                return Some("image/svg+xml");
            }
        }

        None
    }

    /// Sniffs bytes and returns the detected MIME type, or falls back to `application/octet-stream`.
    #[must_use]
    pub fn sniff_or_fallback(bytes: &[u8]) -> &'static str {
        Self::sniff(bytes).unwrap_or(OCTET_STREAM_MIME)
    }

    /// Resolves canonical MIME type from a common file extension.
    #[must_use]
    pub fn mime_for_extension(extension: &str) -> Option<&'static str> {
        let clean = extension.trim_start_matches('.').to_ascii_lowercase();
        match clean.as_str() {
            "jpg" | "jpeg" => Some("image/jpeg"),
            "png" => Some("image/png"),
            "gif" => Some("image/gif"),
            "webp" => Some("image/webp"),
            "svg" => Some("image/svg+xml"),
            "pdf" => Some("application/pdf"),
            "zip" => Some("application/zip"),
            "gz" | "gzip" => Some("application/gzip"),
            "tar" => Some("application/x-tar"),
            "mp3" => Some("audio/mpeg"),
            "mp4" => Some("video/mp4"),
            "json" => Some("application/json"),
            "txt" => Some("text/plain"),
            "html" | "htm" => Some("text/html"),
            "csv" => Some("text/csv"),
            _ => None,
        }
    }

    /// Checks whether an extension is compatible with a given MIME type.
    #[must_use]
    pub fn is_compatible(extension: &str, mime: &str) -> bool {
        if let Some(expected_mime) = Self::mime_for_extension(extension) {
            expected_mime.eq_ignore_ascii_case(mime)
        } else {
            // Uncataloged extensions do not contradict MIME type
            true
        }
    }
}
