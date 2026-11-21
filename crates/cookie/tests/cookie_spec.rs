use std::error::Error;

use cookie::{Cookie, CookieConfig, CookieError, CookieJar, CookieSecurity, SameSite};
use gopon::{Key, KeyRing};
use proptest::prelude::*;
use time::{Duration, OffsetDateTime};

fn test_key() -> Key {
    let key_bytes = [42u8; 32];
    Key::from_bytes(key_bytes)
}

fn test_keyring() -> KeyRing {
    KeyRing::new(test_key())
}

#[test]
fn test_valid_cookie_name_and_value_validation() {
    let valid_cookie = Cookie::new("session_token", "abc123XYZ-._~");
    assert!(valid_cookie.is_ok());
    let cookie = valid_cookie.expect("valid cookie");
    assert_eq!(cookie.name(), "session_token");
    assert_eq!(cookie.value(), "abc123XYZ-._~");

    // Invalid cookie names
    let empty_name = Cookie::new("", "value");
    assert!(empty_name.is_err());
    assert!(empty_name.expect_err("empty name").is_invalid_name());

    let space_name = Cookie::new("session token", "value");
    assert!(space_name.is_err());
    assert!(space_name.expect_err("space in name").is_invalid_name());

    let semicolon_name = Cookie::new("session;token", "value");
    assert!(semicolon_name.is_err());
    assert!(
        semicolon_name
            .expect_err("semicolon in name")
            .is_invalid_name()
    );

    let control_char_name = Cookie::new("session\x07token", "value");
    assert!(control_char_name.is_err());
    assert!(
        control_char_name
            .expect_err("control char")
            .is_invalid_name()
    );

    let non_ascii_name = Cookie::new("session©", "value");
    assert!(non_ascii_name.is_err());
    assert!(non_ascii_name.expect_err("non-ascii").is_invalid_name());

    // Invalid cookie values
    let space_val = Cookie::new("name", "val ue");
    assert!(space_val.is_err());
    assert!(space_val.expect_err("space in val").is_invalid_value());

    let semicolon_val = Cookie::new("name", "val;ue");
    assert!(semicolon_val.is_err());
    assert!(
        semicolon_val
            .expect_err("semicolon in val")
            .is_invalid_value()
    );

    let comma_val = Cookie::new("name", "val,ue");
    assert!(comma_val.is_err());
    assert!(comma_val.expect_err("comma in val").is_invalid_value());

    let backslash_val = Cookie::new("name", "val\\ue");
    assert!(backslash_val.is_err());
    assert!(
        backslash_val
            .expect_err("backslash in val")
            .is_invalid_value()
    );

    let control_val = Cookie::new("name", "val\nue");
    assert!(control_val.is_err());
    assert!(
        control_val
            .expect_err("control char in val")
            .is_invalid_value()
    );
}

#[test]
fn test_cookie_builder_and_to_set_cookie_serialization() {
    let expires_time = OffsetDateTime::from_unix_timestamp(1_700_000_000).expect("timestamp");

    let cookie = Cookie::build("sid", "secret123")
        .path("/api")
        .domain("example.com")
        .http_only(true)
        .secure(true)
        .same_site(SameSite::Strict)
        .partitioned(true)
        .max_age(Duration::hours(2))
        .expires(expires_time)
        .build()
        .expect("builder must succeed");

    assert_eq!(cookie.name(), "sid");
    assert_eq!(cookie.value(), "secret123");
    assert_eq!(cookie.path(), Some("/api"));
    assert_eq!(cookie.domain(), Some("example.com"));
    assert!(cookie.http_only());
    assert!(cookie.secure());
    assert_eq!(cookie.same_site(), Some(SameSite::Strict));
    assert!(cookie.partitioned());
    assert_eq!(cookie.max_age(), Some(Duration::hours(2)));
    assert_eq!(cookie.expires(), Some(expires_time));

    let header = cookie.to_set_cookie_header();
    assert!(header.starts_with("sid=secret123"));
    assert!(header.contains("Path=/api"));
    assert!(header.contains("Domain=example.com"));
    assert!(header.contains("Max-Age=7200"));
    assert!(header.contains("Secure"));
    assert!(header.contains("HttpOnly"));
    assert!(header.contains("SameSite=Strict"));
    assert!(header.contains("Partitioned"));
    assert!(header.contains("Expires="));

    // Display trait matches to_set_cookie_header
    assert_eq!(cookie.to_string(), header);
}

#[test]
fn test_parse_set_cookie_header() {
    let header_str = "auth_token=xyz789; Path=/admin; Domain=sub.example.com; Max-Age=3600; Secure; HttpOnly; SameSite=Lax; Partitioned";
    let parsed = Cookie::parse_set_cookie(header_str).expect("valid Set-Cookie string");

    assert_eq!(parsed.name(), "auth_token");
    assert_eq!(parsed.value(), "xyz789");
    assert_eq!(parsed.path(), Some("/admin"));
    assert_eq!(parsed.domain(), Some("sub.example.com"));
    assert_eq!(parsed.max_age(), Some(Duration::seconds(3600)));
    assert!(parsed.secure());
    assert!(parsed.http_only());
    assert_eq!(parsed.same_site(), Some(SameSite::Lax));
    assert!(parsed.partitioned());

    // Quoted value in Set-Cookie
    let quoted_header = "token=\"quoted_secret\"; Path=/";
    let parsed_quoted = Cookie::parse_set_cookie(quoted_header).expect("quoted parse");
    assert_eq!(parsed_quoted.name(), "token");
    assert_eq!(parsed_quoted.value(), "quoted_secret");

    // Malformed Set-Cookie headers
    let empty_header = Cookie::parse_set_cookie("");
    assert!(empty_header.is_err());

    let missing_equals = Cookie::parse_set_cookie("onlyname; Path=/");
    assert!(missing_equals.is_err());

    let invalid_max_age = Cookie::parse_set_cookie("name=val; Max-Age=notanumber");
    assert!(invalid_max_age.is_err());
}

#[test]
fn test_parse_cookie_request_header() {
    let cookie_header = "session=xyz; theme=dark; logged_in=true; quoted=\"double_quoted\"";
    let parsed = Cookie::parse_cookie_header(cookie_header);

    assert_eq!(parsed.len(), 4);
    assert_eq!(parsed[0], ("session".to_string(), "xyz".to_string()));
    assert_eq!(parsed[1], ("theme".to_string(), "dark".to_string()));
    assert_eq!(parsed[2], ("logged_in".to_string(), "true".to_string()));
    assert_eq!(
        parsed[3],
        ("quoted".to_string(), "double_quoted".to_string())
    );

    // Tolerant handling of malformed parts and extra semicolons
    let messy_header = ";; session=token;; invalid_no_equals; theme=light; ;";
    let parsed_messy = Cookie::parse_cookie_header(messy_header);
    assert_eq!(parsed_messy.len(), 2);
    assert_eq!(
        parsed_messy[0],
        ("session".to_string(), "token".to_string())
    );
    assert_eq!(parsed_messy[1], ("theme".to_string(), "light".to_string()));
}

#[test]
fn test_plain_signed_and_encrypted_cookie_roundtrips() {
    let keyring = test_keyring();
    let mut jar = CookieJar::with_keyring(keyring);

    // 1. Plain cookie
    jar.set_plain("theme", "dracula").expect("plain set");
    assert_eq!(
        jar.get("theme").expect("get plain"),
        Some("dracula".to_string())
    );
    assert_eq!(jar.get_plain("theme"), Some("dracula"));

    // 2. Signed cookie
    jar.set_signed("cart_total", "149.99").expect("signed set");
    let signed_wire = jar.get_plain("cart_total").expect("signed wire exists");
    assert!(signed_wire.starts_with("s:149.99."));
    assert_eq!(
        jar.get("cart_total").expect("get signed"),
        Some("149.99".to_string())
    );
    assert_eq!(
        jar.get_signed("cart_total").expect("get_signed"),
        Some("149.99".to_string())
    );

    // 3. Encrypted cookie
    jar.set_encrypted("auth_session", "user_id=12345")
        .expect("encrypted set");
    let encrypted_wire = jar
        .get_plain("auth_session")
        .expect("encrypted wire exists");
    assert!(encrypted_wire.starts_with("e:"));
    assert!(!encrypted_wire.contains("user_id=12345"));
    assert_eq!(
        jar.get("auth_session").expect("get encrypted"),
        Some("user_id=12345".to_string())
    );
    assert_eq!(
        jar.get_encrypted("auth_session").expect("get_encrypted"),
        Some("user_id=12345".to_string())
    );
}

#[test]
fn test_signed_cookie_tamper_detection() {
    let keyring = test_keyring();
    let mut jar = CookieJar::with_keyring(keyring.clone());

    jar.set_signed("account", "basic_user").expect("signed set");
    let valid_wire = jar.get_plain("account").expect("wire").to_string();

    // Attacker modifies the payload to "admin_user" keeping original signature
    let (prefix_and_val, sig) = valid_wire.rsplit_once('.').expect("wire has dot");
    assert!(prefix_and_val.starts_with("s:"));
    let tampered_wire = format!("s:admin_user.{sig}");

    let tampered_jar =
        CookieJar::from_headers_with_keyring(Some(&format!("account={tampered_wire}")), keyring);

    let verify_result = tampered_jar.get("account");
    assert!(verify_result.is_err());
    assert!(
        verify_result
            .expect_err("tamper")
            .is_authentication_failed()
    );
}

#[test]
fn test_purpose_binding_cross_cookie_substitution_attack_rejection() {
    let keyring = test_keyring();
    let mut jar = CookieJar::with_keyring(keyring.clone());

    // Server encrypts role=guest for "user_role"
    jar.set_encrypted("user_role", "guest")
        .expect("set encrypted");
    let role_wire = jar.get_plain("user_role").expect("wire").to_string();

    // Attacker attempts cross-cookie replay: copying "user_role" payload into "admin_role"
    let attacker_jar = CookieJar::from_headers_with_keyring(
        Some(&format!("admin_role={role_wire}")),
        keyring.clone(),
    );

    let substitution_result = attacker_jar.get("admin_role");
    assert!(substitution_result.is_err());
    let err = substitution_result.expect_err("cross-cookie replay must fail");
    assert!(err.is_purpose_mismatch() || err.is_authentication_failed());

    // Same check for signed cookies: copying signed "role" into "rank"
    let mut signer_jar = CookieJar::with_keyring(keyring.clone());
    signer_jar.set_signed("role", "member").expect("set signed");
    let signed_wire = signer_jar.get_plain("role").expect("wire").to_string();

    let attacker_signer_jar =
        CookieJar::from_headers_with_keyring(Some(&format!("rank={signed_wire}")), keyring);
    let signed_sub_result = attacker_signer_jar.get("rank");
    assert!(signed_sub_result.is_err());
    assert!(
        signed_sub_result
            .expect_err("signed cross-cookie replay")
            .is_authentication_failed()
    );
}

#[test]
fn test_zero_downtime_key_rotation_support() {
    let old_key = Key::from_bytes([1u8; 32]);
    let new_key = Key::from_bytes([2u8; 32]);

    // Client received cookies signed and encrypted under old_key
    let old_keyring = KeyRing::new(old_key.clone());
    let mut old_jar = CookieJar::with_keyring(old_keyring);
    old_jar.set_signed("cart", "item_42").expect("signed");
    old_jar
        .set_encrypted("token", "secret_token_val")
        .expect("encrypted");

    let cart_wire = old_jar.get_plain("cart").expect("cart wire").to_string();
    let token_wire = old_jar.get_plain("token").expect("token wire").to_string();

    // Server rotates to new_key, retaining old_key in fallback list
    let rotated_keyring = KeyRing::with_previous(new_key, vec![old_key]);
    let rotated_jar = CookieJar::from_headers_with_keyring(
        Some(&format!("cart={cart_wire}; token={token_wire}")),
        rotated_keyring,
    );

    // Both signed and encrypted cookies decrypt seamlessly using rotated keyring
    assert_eq!(
        rotated_jar
            .get("cart")
            .expect("cart verified with rotated key"),
        Some("item_42".to_string())
    );
    assert_eq!(
        rotated_jar
            .get("token")
            .expect("token decrypted with rotated key"),
        Some("secret_token_val".to_string())
    );
}

#[test]
fn test_overlay_read_your_own_writes_and_delta_tracking() {
    let incoming_header = "theme=light; lang=en; session=abc123";
    let mut jar = CookieJar::from_headers(Some(incoming_header));

    // Incoming inspection
    assert_eq!(jar.incoming_count(), 3);
    assert_eq!(jar.outgoing_count(), 0);
    assert_eq!(jar.get("theme").expect("theme"), Some("light".to_string()));
    assert_eq!(jar.get("lang").expect("lang"), Some("en".to_string()));
    assert_eq!(
        jar.get("session").expect("session"),
        Some("abc123".to_string())
    );
    assert!(jar.has("theme"));

    // Mutation 1: Read-your-own-writes (theme changed to dark)
    jar.set_plain("theme", "dark").expect("theme mutated");
    assert_eq!(jar.outgoing_count(), 1);
    assert_eq!(
        jar.get("theme").expect("read own write"),
        Some("dark".to_string())
    );
    assert_eq!(jar.get_plain("theme"), Some("dark"));

    // Mutation 2: New cookie added
    jar.set_plain("currency", "USD").expect("currency added");
    assert_eq!(jar.outgoing_count(), 2);
    assert_eq!(
        jar.get("currency").expect("currency"),
        Some("USD".to_string())
    );

    // Mutation 3: Deletion tombstone
    jar.remove("session");
    assert_eq!(jar.outgoing_count(), 3);
    assert_eq!(jar.get("session").expect("removed cookie"), None);
    assert!(!jar.has("session"));

    // Verify unstage
    jar.set_plain("temporary", "discard_me")
        .expect("temporary set");
    assert!(jar.is_staged("temporary"));
    let unstaged = jar.unstage("temporary");
    assert!(unstaged.is_some());
    assert!(!jar.is_staged("temporary"));

    // Delta verification: only modified, added, and removed cookies are in delta
    let delta = jar.delta();
    assert_eq!(delta.len(), 3);

    let headers = jar.render_set_cookie_headers();
    assert_eq!(headers.len(), 3);

    // Headers are deterministically sorted by name because CookieJar uses BTreeMap
    assert!(headers[0].starts_with("currency=USD; "));
    assert!(headers[1].starts_with("session=; "));
    assert!(headers[1].contains("Max-Age=0"));
    assert!(headers[1].contains("Expires=Thu, 01 Jan 1970 00:00:00 GMT"));
    assert!(headers[2].starts_with("theme=dark; "));
}

#[test]
fn test_unkeyed_jar_missing_key_error() {
    let mut unkeyed_jar = CookieJar::new();

    let signed_err = unkeyed_jar.set_signed("foo", "bar");
    assert!(signed_err.is_err());
    assert!(signed_err.expect_err("missing key").is_missing_key());

    let encrypted_err = unkeyed_jar.set_encrypted("foo", "bar");
    assert!(encrypted_err.is_err());
    assert!(encrypted_err.expect_err("missing key").is_missing_key());

    // Reading signed/encrypted without key returns MissingKey
    let mut incoming_signed_jar = CookieJar::from_headers(Some("foo=s:bar.invalidsig"));
    let get_signed_err = incoming_signed_jar.get("foo");
    assert!(get_signed_err.is_err());
    assert!(get_signed_err.expect_err("missing key").is_missing_key());

    incoming_signed_jar.set_keyring(test_keyring());
    assert!(incoming_signed_jar.keyring().is_some());
}

#[test]
fn test_cookie_config_defaults_and_customization() {
    let custom_config = CookieConfig::new()
        .path("/app")
        .domain("mycorp.internal")
        .http_only(true)
        .secure(true)
        .same_site(SameSite::Strict)
        .partitioned(true)
        .max_age(Duration::days(30));

    let mut jar = CookieJar::new().with_config(custom_config.clone());
    assert_eq!(jar.config(), &custom_config);

    jar.set_plain("session", "xyz").expect("plain set");
    let staged = jar.get_staged("session").expect("staged session");
    assert_eq!(staged.path(), Some("/app"));
    assert_eq!(staged.domain(), Some("mycorp.internal"));
    assert!(staged.http_only());
    assert!(staged.secure());
    assert_eq!(staged.same_site(), Some(SameSite::Strict));
    assert!(staged.partitioned());
    assert_eq!(staged.max_age(), Some(Duration::days(30)));
}

#[test]
fn test_samesite_and_security_enum_behaviors() {
    // SameSite variants and as_str
    assert_eq!(SameSite::Strict.as_str(), "Strict");
    assert_eq!(SameSite::Lax.as_str(), "Lax");
    assert_eq!(SameSite::None.as_str(), "None");
    assert_eq!(SameSite::default(), SameSite::Lax);
    assert_eq!(format!("{}", SameSite::Strict), "Strict");

    // SameSite parse
    assert_eq!(SameSite::parse("strict").expect("strict"), SameSite::Strict);
    assert_eq!(SameSite::parse("LAX").expect("lax"), SameSite::Lax);
    assert_eq!(SameSite::parse("None").expect("none"), SameSite::None);
    assert!(SameSite::parse("unsupported").is_err());

    // CookieSecurity detection and default
    assert_eq!(CookieSecurity::default(), CookieSecurity::Plain);
    assert_eq!(
        cookie::security::detect_security("s:val.sig"),
        CookieSecurity::Signed
    );
    assert_eq!(
        cookie::security::detect_security("e:envelope"),
        CookieSecurity::Encrypted
    );
    assert_eq!(
        cookie::security::detect_security("raw_val"),
        CookieSecurity::Plain
    );
}

#[test]
fn test_cookie_mutators_and_tombstone_detection() {
    let mut cookie = Cookie::new("session", "abc").expect("valid cookie");
    assert!(!cookie.is_tombstone());

    cookie.set_value("def").expect("valid set");
    assert_eq!(cookie.value(), "def");

    let invalid_set = cookie.set_value("invalid value with space");
    assert!(invalid_set.is_err());

    cookie.set_path("/test");
    assert_eq!(cookie.path(), Some("/test"));

    cookie.set_domain("sub.example.com");
    assert_eq!(cookie.domain(), Some("sub.example.com"));

    cookie.set_secure(true);
    assert!(cookie.secure());

    cookie.set_http_only(true);
    assert!(cookie.http_only());

    cookie.set_same_site(Some(SameSite::Strict));
    assert_eq!(cookie.same_site(), Some(SameSite::Strict));

    cookie.set_partitioned(true);
    assert!(cookie.partitioned());

    // Max-Age tombstone
    cookie.set_max_age(Duration::ZERO);
    assert!(cookie.is_tombstone());

    cookie.set_max_age(Duration::seconds(-10));
    assert!(cookie.is_tombstone());

    cookie.set_max_age(Duration::hours(1));
    assert!(!cookie.is_tombstone());

    // Unix epoch timestamp tombstone
    cookie.set_expires(OffsetDateTime::UNIX_EPOCH);
    assert!(cookie.is_tombstone());
}

#[test]
fn test_security_module_direct_pack_and_unpack() {
    let key = test_key();
    let signer = gopon::HmacSigner::new(key.clone());
    let encrypter = gopon::Encrypter::new(KeyRing::new(key));

    // Signed low-level
    let signed = cookie::security::pack_signed("token", "val_xyz", &signer);
    let unpacked_signed = cookie::security::unpack_signed("token", &signed, &signer);
    assert_eq!(unpacked_signed.expect("unpack signed"), "val_xyz");

    // Invalid prefix
    assert!(cookie::security::unpack_signed("token", "val_xyz", &signer).is_err());
    // Missing dot
    assert!(cookie::security::unpack_signed("token", "s:val_xyz", &signer).is_err());
    // Tampered
    assert!(cookie::security::unpack_signed("token", "s:val_xyz.badsig", &signer).is_err());

    // Encrypted low-level
    let encrypted =
        cookie::security::pack_encrypted("secret", "top_secret", &encrypter).expect("pack");
    let decrypted =
        cookie::security::unpack_encrypted("secret", &encrypted, &encrypter).expect("unpack");
    assert_eq!(decrypted, "top_secret");

    // Invalid prefix
    assert!(cookie::security::unpack_encrypted("secret", "top_secret", &encrypter).is_err());
    // Tampered
    assert!(
        cookie::security::unpack_encrypted("secret", "e:invalid_envelope", &encrypter).is_err()
    );
}

#[test]
fn test_cookie_error_display_and_traits() {
    let err_name = CookieError::InvalidName("empty".to_string());
    assert!(err_name.is_invalid_name());
    assert!(format!("{err_name}").contains("invalid cookie name"));

    let err_val = CookieError::InvalidValue("space".to_string());
    assert!(err_val.is_invalid_value());
    assert!(format!("{err_val}").contains("invalid cookie value"));

    let err_header = CookieError::MalformedHeader("bad".to_string());
    assert!(format!("{err_header}").contains("malformed cookie header string"));

    let err_auth = CookieError::AuthenticationFailed;
    assert!(err_auth.is_authentication_failed());
    assert!(format!("{err_auth}").contains("cookie authentication failed"));

    let err_purpose = CookieError::PurposeMismatch;
    assert!(err_purpose.is_purpose_mismatch());
    assert!(format!("{err_purpose}").contains("cookie purpose mismatch"));

    let err_key = CookieError::MissingKey;
    assert!(err_key.is_missing_key());
    assert!(format!("{err_key}").contains("cannot sign or encrypt cookie"));

    let err_enc = CookieError::EncodingError("utf8".to_string());
    assert!(format!("{err_enc}").contains("cookie encoding error"));

    let crypto_err = gopon::Error::MalformedEnvelope("corrupt".to_string());
    let err_crypto = CookieError::from(crypto_err.clone());
    assert!(format!("{err_crypto}").contains("cryptographic error"));
    assert!(err_crypto.source().is_some());

    assert_eq!(
        CookieError::from(gopon::Error::AuthenticationFailed),
        CookieError::AuthenticationFailed
    );
    assert_eq!(
        CookieError::from(gopon::Error::PurposeMismatch),
        CookieError::PurposeMismatch
    );
}

#[test]
fn test_cookie_jar_extended_api_coverage() {
    let mut jar = CookieJar::new();
    assert!(jar.is_empty());

    // Mutate config via config_mut
    jar.config_mut().default_path = "/custom".to_string();
    assert_eq!(jar.config().default_path, "/custom");

    // remove_with_options
    jar.remove_with_options("sess", Some("/api"), Some("example.com"));
    let staged = jar.get_staged("sess").expect("staged");
    assert_eq!(staged.path(), Some("/api"));
    assert_eq!(staged.domain(), Some("example.com"));
    assert!(staged.is_tombstone());

    // get_raw
    assert_eq!(jar.get_raw("non_existent"), None);
    assert_eq!(jar.get_plain("non_existent"), None);
    assert_eq!(jar.get_signed("non_existent").expect("none"), None);
    assert_eq!(jar.get_encrypted("non_existent").expect("none"), None);

    // Iterators
    let incoming_jar = CookieJar::from_headers(Some("k1=v1; k2=v2"));
    let incoming_map: std::collections::BTreeMap<_, _> = incoming_jar.incoming().collect();
    assert_eq!(incoming_map.len(), 2);
    assert_eq!(incoming_map.get("k1"), Some(&"v1"));
    assert_eq!(incoming_map.get("k2"), Some(&"v2"));

    let mut out_jar = CookieJar::new();
    out_jar.set_plain("out1", "val1").expect("set");
    let out_names: Vec<_> = out_jar.outgoing_cookies().map(Cookie::name).collect();
    assert_eq!(out_names, vec!["out1"]);
}

#[test]
fn test_typed_getters_reject_mismatched_modes() {
    let keyring = test_keyring();
    let mut jar = CookieJar::with_keyring(keyring);

    jar.set_plain("raw_cookie", "simple_value").expect("plain");
    // get_signed on plain cookie fails
    let signed_err = jar.get_signed("raw_cookie");
    assert!(signed_err.is_err());
    assert!(
        signed_err
            .expect_err("type mismatch")
            .is_authentication_failed()
    );

    // get_encrypted on plain cookie fails
    let encrypted_err = jar.get_encrypted("raw_cookie");
    assert!(encrypted_err.is_err());
    assert!(
        encrypted_err
            .expect_err("type mismatch")
            .is_authentication_failed()
    );
}

#[test]
fn test_cookie_and_jar_edge_cases_and_parsing() {
    // 1. Error source None for non-crypto errors
    assert!(CookieError::AuthenticationFailed.source().is_none());
    assert!(
        CookieError::InvalidName("bad".to_string())
            .source()
            .is_none()
    );
    assert!(CookieError::MissingKey.source().is_none());

    // 2. parse_cookie_header tolerant handling of invalid names and missing names
    let pairs = Cookie::parse_cookie_header("bad@name=value; valid=123; ; =no_name");
    assert_eq!(pairs.len(), 1);
    assert_eq!(pairs[0], ("valid".to_string(), "123".to_string()));

    // 3. parse_set_cookie empty or whitespace-only header error
    assert!(Cookie::parse_set_cookie("").is_err());
    assert!(Cookie::parse_set_cookie("   ").is_err());

    // 4. parse_set_cookie with unknown attributes and flags and empty segments
    let parsed_complex =
        Cookie::parse_set_cookie("foo=bar; ; UnknownAttr=hello; UnknownFlag; HttpOnly; Path=/path")
            .expect("parsed successfully");
    assert_eq!(parsed_complex.name(), "foo");
    assert_eq!(parsed_complex.value(), "bar");
    assert!(parsed_complex.http_only());
    assert_eq!(parsed_complex.path(), Some("/path"));

    // 5. parse_set_cookie with IMF-fixdate Expires
    let parsed_imf = Cookie::parse_set_cookie("foo=bar; Expires=Wed, 21 Oct 2026 07:28:00 GMT")
        .expect("parsed IMF date");
    assert!(parsed_imf.expires().is_some());

    // 6. parse_set_cookie with RFC 2822 Expires fallback
    let parsed_rfc2822 =
        Cookie::parse_set_cookie("foo=bar; Expires=Wed, 21 Oct 2026 07:28:00 +0000")
            .expect("parsed RFC 2822 date");
    assert!(parsed_rfc2822.expires().is_some());

    // 7. parse_set_cookie with malformed date
    let malformed_date = Cookie::parse_set_cookie("foo=bar; Expires=not-a-valid-date");
    assert!(malformed_date.is_err());

    // 8. CookieJar from_headers(None)
    let empty_jar = CookieJar::from_headers(None);
    assert!(empty_jar.is_empty());
    assert_eq!(empty_jar.get("none").expect("none"), None);

    // 9. jar.set_config
    let mut custom_jar = CookieJar::new();
    let config = CookieConfig::new().path("/custom_path").same_site(None);
    custom_jar.set_config(config);
    assert_eq!(custom_jar.config().default_path, "/custom_path");
    assert_eq!(custom_jar.config().default_same_site, None);

    // Test creating configured cookie with default_same_site None
    custom_jar.set_plain("test_name", "test_val").expect("set");
    let staged_cookie = custom_jar.get_staged("test_name").expect("staged");
    assert_eq!(staged_cookie.same_site(), None);

    // 10. jar.get on incoming plain cookie
    let incoming_jar = CookieJar::from_headers(Some("existing_key=existing_val"));
    assert_eq!(
        incoming_jar.get("existing_key").expect("existing key"),
        Some("existing_val".to_string())
    );
    assert_eq!(incoming_jar.get("missing_key").expect("missing key"), None);

    // 11. jar.set(cookie) directly staging a Cookie
    let direct_cookie = Cookie::new("direct", "direct_val").expect("valid cookie");
    custom_jar.set(direct_cookie);
    assert_eq!(
        custom_jar.get("direct").expect("get direct"),
        Some("direct_val".to_string())
    );

    // 12. jar.get_plain on tombstoned cookie returns None
    custom_jar.remove("direct");
    assert_eq!(custom_jar.get_plain("direct"), None);

    // 13. remove_with_options variations
    let mut opts_jar = CookieJar::new();
    opts_jar.remove_with_options("sess1", Some("/only_path"), None);
    assert_eq!(
        opts_jar.get_staged("sess1").and_then(Cookie::path),
        Some("/only_path")
    );
    assert_eq!(opts_jar.get_staged("sess1").and_then(Cookie::domain), None);

    opts_jar.remove_with_options("sess2", None, Some("only.domain.com"));
    assert_eq!(opts_jar.get_staged("sess2").and_then(Cookie::path), None);
    assert_eq!(
        opts_jar.get_staged("sess2").and_then(Cookie::domain),
        Some("only.domain.com")
    );

    // Invalid cookie name in remove_with_options is ignored safely
    opts_jar.remove_with_options("invalid;cookie;name", None, None);
    assert!(!opts_jar.is_staged("invalid;cookie;name"));
}

proptest! {
    #[test]
    fn proptest_cookie_name_and_value_roundtrip(
        name in "[a-zA-Z0-9_-]{1,30}",
        value in "[a-zA-Z0-9._~-]{0,64}"
    ) {
        let cookie = Cookie::new(&name, &value).expect("valid token");
        assert_eq!(cookie.name(), name);
        assert_eq!(cookie.value(), value);

        let header = cookie.to_set_cookie_header();
        let parsed = Cookie::parse_set_cookie(&header).expect("must parse Set-Cookie");
        assert_eq!(parsed.name(), name);
        assert_eq!(parsed.value(), value);
    }

    #[test]
    fn proptest_signed_cookie_roundtrip(
        value in "[a-zA-Z0-9._~-]{1,64}"
    ) {
        let keyring = test_keyring();
        let mut jar = CookieJar::with_keyring(keyring);

        jar.set_signed("prop_cookie", &value).expect("set signed");
        let retrieved = jar.get("prop_cookie").expect("get signed");
        assert_eq!(retrieved, Some(value));
    }

    #[test]
    fn proptest_encrypted_cookie_roundtrip(
        value in "[a-zA-Z0-9._~-]{1,64}"
    ) {
        let keyring = test_keyring();
        let mut jar = CookieJar::with_keyring(keyring);

        jar.set_encrypted("prop_enc", &value).expect("set encrypted");
        let retrieved = jar.get("prop_enc").expect("get encrypted");
        assert_eq!(retrieved, Some(value));
    }
}
