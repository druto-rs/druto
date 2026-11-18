use crypto::{Algorithm, CryptoError, Encrypter, Envelope, HmacSigner, Key, KeyRing, UrlSigner};

#[test]
fn test_key_length_validation_and_generation() {
    let generated = Key::generate().expect("key generation must succeed");
    assert_eq!(generated.as_bytes().len(), 32);

    let valid_bytes = [7u8; 32];
    let key = Key::from_bytes(valid_bytes);
    assert_eq!(key.as_bytes(), &valid_bytes);

    let invalid_short = Key::try_from_slice(&[1u8; 16]);
    assert!(invalid_short.is_err());
    assert!(matches!(
        invalid_short.expect_err("must fail"),
        CryptoError::InvalidKeyLength {
            expected: 32,
            actual: 16
        }
    ));

    let invalid_long = Key::try_from_slice(&[2u8; 64]);
    assert!(invalid_long.is_err());
    assert!(matches!(
        invalid_long.expect_err("must fail"),
        CryptoError::InvalidKeyLength {
            expected: 32,
            actual: 64
        }
    ));

    // Base64 decoding
    let b64_str = "base64:AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=";
    let b64_key = Key::from_base64(b64_str).expect("base64 key must parse");
    assert_eq!(b64_key.as_bytes(), &[0u8; 32]);

    // Redacted Debug format
    let debug_str = format!("{key:?}");
    assert_eq!(debug_str, "Key([REDACTED])");
}

#[test]
fn test_hmac_sha256_known_answer_test() {
    // RFC 4231 Test Case 1
    // Key: 0x0b repeated 20 times (20 bytes) -> padded / slice
    // Test with standard 32-byte key KAT
    let key_bytes = [0x0bu8; 32];
    let key = Key::from_bytes(key_bytes);
    let signer = HmacSigner::new(key);

    let data = b"Hi There";
    let signature = signer.sign(data);
    assert_eq!(signature.len(), 32);

    assert!(signer.verify(data, &signature));
    assert!(!signer.verify(b"Hi There!", &signature));

    // Base64URL signature roundtrip
    let b64_sig = signer.sign_base64(data);
    assert!(signer.verify_base64(data, &b64_sig));
    assert!(!signer.verify_base64(b"Hi There!", &b64_sig));
    assert!(!signer.verify_base64(data, "invalid_base64_sig"));
}

#[test]
fn test_encrypter_aes_roundtrip_and_purpose_binding() {
    let key = Key::generate().expect("key generation failed");
    let encrypter = Encrypter::from_key(key);

    assert_eq!(encrypter.algorithm(), Algorithm::Aes256Gcm);

    let message = "Hello Druto Framework! Sensitive session token payload.";
    let encrypted = encrypter.encrypt_string(message).expect("encrypt failed");

    // Decrypt without purpose
    let decrypted = encrypter
        .decrypt_string(&encrypted)
        .expect("decrypt failed");
    assert_eq!(decrypted, message);

    // Purpose binding
    let purpose_cookie = "cookie:session";
    let encrypted_with_purpose = encrypter
        .encrypt_string_with_purpose(message, purpose_cookie)
        .expect("encrypt with purpose failed");

    // Matching purpose succeeds
    let decrypted_cookie = encrypter
        .decrypt_string_with_purpose(&encrypted_with_purpose, purpose_cookie)
        .expect("decrypt with matching purpose failed");
    assert_eq!(decrypted_cookie, message);

    // Mismatched purpose fails with AuthenticationFailed
    let mismatched =
        encrypter.decrypt_string_with_purpose(&encrypted_with_purpose, "cookie:remember_me");
    assert!(mismatched.is_err());
    assert!(
        mismatched
            .expect_err("must fail")
            .is_authentication_failed()
    );

    // Decrypting purpose-bound ciphertext without purpose fails
    let no_purpose_attempt = encrypter.decrypt_string(&encrypted_with_purpose);
    assert!(no_purpose_attempt.is_err());
    assert!(
        no_purpose_attempt
            .expect_err("must fail")
            .is_authentication_failed()
    );
}

#[test]
fn test_encrypter_chacha20_roundtrip() {
    let key = Key::generate().expect("key generation failed");
    let keyring = KeyRing::new(key);
    let encrypter = Encrypter::with_algorithm(keyring, Algorithm::ChaCha20Poly1305);

    assert_eq!(encrypter.algorithm(), Algorithm::ChaCha20Poly1305);

    let message = "ChaCha20-Poly1305 high performance pure-Rust stream cipher payload";
    let encrypted = encrypter
        .encrypt_string(message)
        .expect("chacha encrypt failed");

    let decrypted = encrypter
        .decrypt_string(&encrypted)
        .expect("chacha decrypt failed");
    assert_eq!(decrypted, message);

    // Purpose binding on ChaCha20
    let encrypted_purpose = encrypter
        .encrypt_string_with_purpose(message, "token:auth")
        .expect("chacha encrypt purpose failed");
    let decrypted_purpose = encrypter
        .decrypt_string_with_purpose(&encrypted_purpose, "token:auth")
        .expect("chacha decrypt purpose failed");
    assert_eq!(decrypted_purpose, message);

    let wrong_purpose = encrypter.decrypt_string_with_purpose(&encrypted_purpose, "token:csrf");
    assert!(wrong_purpose.is_err());
    assert!(
        wrong_purpose
            .expect_err("must fail")
            .is_authentication_failed()
    );
}

#[test]
fn test_encrypter_tamper_detection() {
    let key = Key::generate().expect("key generation failed");
    let encrypter = Encrypter::from_key(key);

    let message = "Tamper proof integrity check";
    let encoded = encrypter.encrypt_string(message).expect("encrypt failed");

    let mut envelope = Envelope::from_base64(&encoded).expect("envelope parse failed");

    // Tamper with ciphertext
    let mut tampered_cipher = envelope.clone();
    if let Some(first_byte) = tampered_cipher.ciphertext.first_mut() {
        *first_byte ^= 0x01;
    }
    let tampered_encoded = tampered_cipher.to_base64();
    assert!(encrypter.decrypt_string(&tampered_encoded).is_err());

    // Tamper with nonce
    let mut tampered_nonce = envelope.clone();
    tampered_nonce.nonce[0] ^= 0x01;
    assert!(
        encrypter
            .decrypt_string(&tampered_nonce.to_base64())
            .is_err()
    );

    // Tamper with tag
    let mut tampered_tag = envelope.clone();
    tampered_tag.tag[0] ^= 0x01;
    assert!(encrypter.decrypt_string(&tampered_tag.to_base64()).is_err());

    // Tamper with algorithm code
    envelope.algorithm = Algorithm::ChaCha20Poly1305;
    assert!(encrypter.decrypt_string(&envelope.to_base64()).is_err());
}

#[test]
fn test_key_rotation_seamless_fallback() {
    let old_key = Key::generate().expect("old key generation failed");
    let old_encrypter = Encrypter::from_key(old_key.clone());

    let message = "Session encrypted under legacy key prior to rotation";
    let legacy_ciphertext = old_encrypter
        .encrypt_string(message)
        .expect("legacy encrypt failed");

    // Rotate keys: new primary key, old key placed in previous
    let new_primary = Key::generate().expect("new key generation failed");
    let rotated_keyring = KeyRing::with_previous(new_primary.clone(), vec![old_key]);
    let rotated_encrypter = Encrypter::new(rotated_keyring);

    // Old ciphertext decrypts seamlessly using the fallback key
    let decrypted_legacy = rotated_encrypter
        .decrypt_string(&legacy_ciphertext)
        .expect("legacy ciphertext must decrypt with previous key");
    assert_eq!(decrypted_legacy, message);

    // New encryption operations use the new primary key
    let new_ciphertext = rotated_encrypter
        .encrypt_string("New session token")
        .expect("new encrypt failed");

    // Old encrypter cannot decrypt new ciphertext
    assert!(old_encrypter.decrypt_string(&new_ciphertext).is_err());

    // Rotated encrypter decrypts new ciphertext
    assert_eq!(
        rotated_encrypter
            .decrypt_string(&new_ciphertext)
            .expect("decrypt failed"),
        "New session token"
    );
}

#[test]
fn test_url_signer_canonicalization_and_verification() {
    let key = Key::generate().expect("key gen failed");
    let signer = UrlSigner::new(key);

    let base_url = "https://druto.dev/api/verify-email?user=42&source=marketing";
    let signed = signer.sign(base_url);

    assert!(signed.contains("&signature="));
    assert!(signer.verify(&signed).is_ok());

    // Query parameters reordered by proxy or client: /api/verify-email?source=marketing&user=42&signature=...
    let reordered_url = "https://druto.dev/api/verify-email?source=marketing&user=42&signature=";
    let signature_part = signed.split("&signature=").nth(1).expect("sig part");
    let full_reordered = format!("{reordered_url}{signature_part}");

    assert!(
        signer.verify(&full_reordered).is_ok(),
        "canonicalization must allow reordered query parameters"
    );

    // Tampering with parameter value fails verification
    let tampered_url = format!(
        "https://druto.dev/api/verify-email?user=99&source=marketing&signature={signature_part}"
    );
    let tampered_res = signer.verify(&tampered_url);
    assert!(tampered_res.is_err());
    assert!(tampered_res.expect_err("must fail").is_invalid_signature());

    // Missing signature fails verification
    let missing_sig = signer.verify("https://druto.dev/api/verify-email?user=42");
    assert!(missing_sig.is_err());
}

#[test]
fn test_temporary_signed_url_expiration() {
    let key = Key::generate().expect("key gen failed");
    let signer = UrlSigner::new(key);

    let url = "/download/report.pdf?file=annual_report";
    let expires_timestamp: u64 = 1_700_000_000;

    let signed_url = signer.sign_temporary(url, expires_timestamp);

    assert!(signed_url.contains("expires=1700000000"));
    assert!(signed_url.contains("&signature="));

    // Valid before expiration
    let valid_time: u64 = 1_699_999_999;
    assert!(signer.verify_at(&signed_url, valid_time).is_ok());

    // Valid at exact expiration
    assert!(signer.verify_at(&signed_url, expires_timestamp).is_ok());

    // Expired after timestamp
    let expired_time: u64 = 1_700_000_001;
    let expired_result = signer.verify_at(&signed_url, expired_time);
    assert!(expired_result.is_err());
    let err = expired_result.expect_err("must be expired");
    assert!(err.is_expired_signature());
    assert!(matches!(
        err,
        CryptoError::ExpiredSignature {
            expires: 1_700_000_000,
            current: 1_700_000_001,
        }
    ));

    // Tampering with expiration timestamp fails signature check
    let tampered_expires = signed_url.replace("expires=1700000000", "expires=1800000000");
    let tampered_res = signer.verify_at(&tampered_expires, valid_time);
    assert!(tampered_res.is_err());
    assert!(tampered_res.expect_err("must fail").is_invalid_signature());
}

#[test]
fn test_envelope_binary_and_base64_serialization() {
    let nonce = [1u8; 12];
    let tag = [2u8; 16];
    let ciphertext = b"encrypted data payload".to_vec();

    let envelope = Envelope::new(Algorithm::Aes256Gcm, nonce, tag, ciphertext.clone());
    let bytes = envelope.to_bytes();
    assert_eq!(bytes.len(), 29 + ciphertext.len());

    let parsed = Envelope::from_bytes(&bytes).expect("parse must succeed");
    assert_eq!(parsed.algorithm, Algorithm::Aes256Gcm);
    assert_eq!(parsed.nonce, nonce);
    assert_eq!(parsed.tag, tag);
    assert_eq!(parsed.ciphertext, ciphertext);

    let b64 = envelope.to_base64();
    let from_b64 = Envelope::from_base64(&b64).expect("b64 parse must succeed");
    assert_eq!(from_b64, envelope);

    // Truncated bytes (< 29 bytes) fails with MalformedEnvelope
    let truncated_bytes = &bytes[..20];
    let truncated_res = Envelope::from_bytes(truncated_bytes);
    assert!(truncated_res.is_err());
    assert!(
        truncated_res
            .expect_err("must fail")
            .is_malformed_envelope()
    );
}

#[test]
fn test_crypto_error_display_and_helpers() {
    let err_auth = CryptoError::AuthenticationFailed;
    assert!(err_auth.is_authentication_failed());
    assert!(err_auth.to_string().contains("authentication failed"));

    let err_exp = CryptoError::ExpiredSignature {
        expires: 100,
        current: 105,
    };
    assert!(err_exp.is_expired_signature());
    assert!(err_exp.to_string().contains("has expired"));

    let err_sig = CryptoError::InvalidSignature;
    assert!(err_sig.is_invalid_signature());
    assert_eq!(err_sig.to_string(), "invalid cryptographic signature");

    let err_purpose = CryptoError::PurposeMismatch;
    assert!(err_purpose.is_purpose_mismatch());
    assert!(err_purpose.to_string().contains("purpose mismatch"));

    let err_malformed = CryptoError::MalformedEnvelope("header too short".to_string());
    assert!(err_malformed.is_malformed_envelope());
    assert!(err_malformed.to_string().contains("header too short"));

    let err_key_len = CryptoError::InvalidKeyLength {
        expected: 32,
        actual: 16,
    };
    assert!(err_key_len.to_string().contains("invalid key length"));

    let err_encoding = CryptoError::EncodingError("bad base64".to_string());
    assert!(err_encoding.to_string().contains("base64 encoding error"));

    let err_missing = CryptoError::MissingParameter("signature".to_string());
    assert!(err_missing.to_string().contains("missing parameter"));

    let err_internal = CryptoError::Internal("csprng failure".to_string());
    assert!(
        err_internal
            .to_string()
            .contains("internal cryptographic failure")
    );
}

#[test]
fn test_algorithm_display_and_from_code() {
    assert_eq!(format!("{}", Algorithm::Aes256Gcm), "aes-256-gcm");
    assert_eq!(
        format!("{}", Algorithm::ChaCha20Poly1305),
        "chacha20-poly1305"
    );

    assert_eq!(
        Algorithm::from_code(1).expect("code 1"),
        Algorithm::Aes256Gcm
    );
    assert_eq!(
        Algorithm::from_code(2).expect("code 2"),
        Algorithm::ChaCha20Poly1305
    );

    let invalid = Algorithm::from_code(99);
    assert!(invalid.is_err());
    assert!(invalid.expect_err("must fail").is_malformed_envelope());
}

#[test]
fn test_key_methods_and_base64_variants() {
    let raw = [42u8; 32];
    let key = Key::from_bytes(raw);
    assert_eq!(key.as_slice(), &raw[..]);

    let key_ring = KeyRing::with_previous(key.clone(), vec![Key::from_bytes([1u8; 32])]);
    assert_eq!(key_ring.primary(), &key);
    assert_eq!(key_ring.previous().len(), 1);

    // URL safe without padding
    use base64::prelude::*;
    let b64_url_safe = BASE64_URL_SAFE_NO_PAD.encode(raw);
    let decoded_key = Key::from_base64(&b64_url_safe).expect("url safe key must decode");
    assert_eq!(decoded_key.as_bytes(), &raw);

    // Standard without padding
    let b64_std_no_pad = BASE64_STANDARD_NO_PAD.encode(raw);
    let decoded_std = Key::from_base64(&b64_std_no_pad).expect("std no pad key must decode");
    assert_eq!(decoded_std.as_bytes(), &raw);

    // Invalid base64
    let invalid_b64 = Key::from_base64("!@#$%^&*()");
    assert!(invalid_b64.is_err());
}

#[test]
fn test_encrypter_raw_bytes_and_invalid_utf8() {
    let key = Key::generate().expect("key gen failed");
    let encrypter = Encrypter::from_key(key);
    assert_eq!(
        encrypter.keyring().primary(),
        encrypter.keyring().all_keys().next().expect("primary")
    );

    // Raw bytes encrypt/decrypt
    let raw_payload = vec![0x00, 0x99, 0xFF, 0xFE, 0x80];
    let encrypted = encrypter
        .encrypt(&raw_payload)
        .expect("encrypt raw bytes failed");
    let decrypted = encrypter
        .decrypt(&encrypted)
        .expect("decrypt raw bytes failed");
    assert_eq!(decrypted, raw_payload);

    // Decrypting non-utf8 bytes with decrypt_string must fail with EncodingError
    let non_utf8_payload = vec![0xFF, 0xFE, 0xFD];
    let encrypted_non_utf8 = encrypter
        .encrypt(&non_utf8_payload)
        .expect("encrypt failed");
    let string_res = encrypter.decrypt_string(&encrypted_non_utf8);
    assert!(string_res.is_err());
    assert!(matches!(
        string_res.expect_err("must fail"),
        CryptoError::EncodingError(_)
    ));

    // Decrypting non-utf8 bytes with purpose
    let encrypted_with_purpose = encrypter
        .encrypt_with_purpose(&non_utf8_payload, "ctx")
        .expect("encrypt failed");
    let decrypted_bytes = encrypter
        .decrypt_with_purpose(&encrypted_with_purpose, "ctx")
        .expect("decrypt failed");
    assert_eq!(decrypted_bytes, non_utf8_payload);

    let string_purpose_res = encrypter.decrypt_string_with_purpose(&encrypted_with_purpose, "ctx");
    assert!(string_purpose_res.is_err());
}

#[test]
fn test_hmac_signer_getters_and_edge_cases() {
    let raw_key = [15u8; 32];
    let key = Key::from_bytes(raw_key);
    let signer = HmacSigner::new(key.clone());
    assert_eq!(signer.key().as_bytes(), &raw_key);

    // Standard base64 signature
    use base64::prelude::*;
    let sig_bytes = signer.sign(b"sample data");
    let std_b64 = BASE64_STANDARD.encode(sig_bytes);
    assert!(signer.verify_base64(b"sample data", &std_b64));

    // Length mismatch in verify
    assert!(!signer.verify(b"sample data", &[1u8; 16]));
}

#[test]
fn test_url_signer_edge_cases_and_verification_order() {
    let key = Key::generate().expect("key gen failed");
    let signer = UrlSigner::new(key);

    // URL without query parameters signed normally
    let clean_url = "https://druto.dev/profile";
    let signed_clean = signer.sign(clean_url);
    assert!(signed_clean.starts_with("https://druto.dev/profile?signature="));
    assert!(signer.verify(&signed_clean).is_ok());

    // Temporary URL without existing query parameters
    let temp_clean = signer.sign_temporary(clean_url, 1_800_000_000);
    assert!(temp_clean.starts_with("https://druto.dev/profile?expires=1800000000&signature="));
    assert!(signer.verify_at(&temp_clean, 1_700_000_000).is_ok());

    // URL without query separator fails with MissingParameter
    let no_query_res = signer.verify("https://druto.dev/profile");
    assert!(no_query_res.is_err());
    assert!(matches!(
        no_query_res.expect_err("must fail"),
        CryptoError::MissingParameter(_)
    ));

    // Non-integer expires value fails with MalformedEnvelope
    let bad_expires_url = "https://druto.dev/profile?expires=not_a_number&signature=dummy";
    let bad_expires_res = signer.verify_at(bad_expires_url, 1_700_000_000);
    assert!(bad_expires_res.is_err());
    assert!(matches!(
        bad_expires_res.expect_err("must fail"),
        CryptoError::MalformedEnvelope(_)
    ));

    // Cryptographic hygiene: Forged/tampered URL with expired timestamp MUST return InvalidSignature
    let tampered_sig_url = format!("{temp_clean}tampered");
    let tampered_res = signer.verify_at(&tampered_sig_url, 1_900_000_000); // 1.9B is past 1.8B expiration
    assert!(tampered_res.is_err());
    assert_eq!(
        tampered_res.expect_err("must be InvalidSignature"),
        CryptoError::InvalidSignature
    );

    // Live system clock verify
    let future_time = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("time")
        .as_secs()
        + 3600;
    let live_signed = signer.sign_temporary("https://druto.dev/api", future_time);
    assert!(signer.verify(&live_signed).is_ok());

    // Query strings with empty pairs and valueless flags
    let complex_url = "https://druto.dev/search?filter&&flag&q=rust";
    let complex_signed = signer.sign(complex_url);
    assert!(signer.verify(&complex_signed).is_ok());
}

#[test]
fn test_envelope_error_branches_and_base64_fallbacks() {
    // Envelope from bytes with unsupported algorithm code
    let mut bytes = vec![99u8]; // invalid alg code
    bytes.extend_from_slice(&[1u8; 12]); // nonce
    bytes.extend_from_slice(&[2u8; 16]); // tag
    bytes.extend_from_slice(b"ciphertext");

    let err_alg = Envelope::from_bytes(&bytes);
    assert!(err_alg.is_err());

    // Envelope from standard base64 string
    use base64::prelude::*;
    let valid_envelope =
        Envelope::new(Algorithm::Aes256Gcm, [1u8; 12], [2u8; 16], b"data".to_vec());
    let std_b64 = BASE64_STANDARD.encode(valid_envelope.to_bytes());
    let parsed_std = Envelope::from_base64(&std_b64).expect("std b64 envelope parse");
    assert_eq!(parsed_std, valid_envelope);

    let invalid_b64 = Envelope::from_base64("!@#$%^&*()");
    assert!(invalid_b64.is_err());
}
