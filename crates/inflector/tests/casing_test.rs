use std::borrow::Cow;

use inflector::{StrExt, case::*};

#[test]
fn test_casing_conversions_roundtrip() {
    let cases = [
        (
            "user_profile_id",
            "userProfileId",
            "UserProfileId",
            "user-profile-id",
            "User-Profile-Id",
            "User Profile Id",
            "User profile id",
            "USER_PROFILE_ID",
        ),
        (
            "foo_bar_baz",
            "fooBarBaz",
            "FooBarBaz",
            "foo-bar-baz",
            "Foo-Bar-Baz",
            "Foo Bar Baz",
            "Foo bar baz",
            "FOO_BAR_BAZ",
        ),
    ];

    for (snake, camel, pascal, kebab, train, title, sentence, screaming) in cases {
        assert_eq!(to_snake_case(snake), snake);
        assert_eq!(to_snake_case(camel), snake);
        assert_eq!(to_snake_case(pascal), snake);
        assert_eq!(to_snake_case(kebab), snake);

        assert_eq!(to_camel_case(snake), camel);
        assert_eq!(to_camel_case(pascal), camel);
        assert_eq!(to_camel_case(kebab), camel);

        assert_eq!(to_pascal_case(snake), pascal);
        assert_eq!(to_pascal_case(camel), pascal);
        assert_eq!(to_pascal_case(kebab), pascal);

        assert_eq!(to_kebab_case(snake), kebab);
        assert_eq!(to_kebab_case(camel), kebab);
        assert_eq!(to_kebab_case(pascal), kebab);

        assert_eq!(to_train_case(snake), train);
        assert_eq!(to_title_case(snake), title);
        assert_eq!(to_sentence_case(snake), sentence);
        assert_eq!(to_screaming_snake_case(snake), screaming);
    }
}

#[test]
fn test_acronym_handling() {
    assert_eq!(to_snake_case("HTTPResponse"), "http_response");
    assert_eq!(to_camel_case("HTTPResponse"), "httpResponse");
    assert_eq!(to_pascal_case("HTTPResponse"), "HttpResponse");
    assert_eq!(to_kebab_case("HTTPResponse"), "http-response");
    assert_eq!(to_screaming_snake_case("HTTPResponse"), "HTTP_RESPONSE");

    assert_eq!(
        to_snake_case("getHTTPResponseCode"),
        "get_http_response_code"
    );
    assert_eq!(to_camel_case("getHTTPResponseCode"), "getHttpResponseCode");
    assert_eq!(to_pascal_case("getHTTPResponseCode"), "GetHttpResponseCode");
    assert_eq!(
        to_kebab_case("getHTTPResponseCode"),
        "get-http-response-code"
    );

    assert_eq!(to_snake_case("XmlHttpRequest"), "xml_http_request");
    assert_eq!(to_camel_case("XmlHttpRequest"), "xmlHttpRequest");
    assert_eq!(to_pascal_case("XmlHttpRequest"), "XmlHttpRequest");

    assert_eq!(to_snake_case("XMLHTTPRequest"), "xmlhttp_request");
    assert_eq!(to_camel_case("XMLHTTPRequest"), "xmlhttpRequest");
    assert_eq!(to_pascal_case("XMLHTTPRequest"), "XmlhttpRequest");
}

#[test]
fn test_delimiters_sanitization() {
    assert_eq!(to_snake_case("__hello__world__"), "hello_world");
    assert_eq!(to_kebab_case("--hello--world--"), "hello-world");
    assert_eq!(to_camel_case("__hello__world__"), "helloWorld");
    assert_eq!(to_pascal_case("__hello__world__"), "HelloWorld");
    assert_eq!(to_snake_case("   user   account   "), "user_account");
    assert_eq!(
        to_snake_case("path/to/resource:name"),
        "path_to_resource_name"
    );
}

#[test]
fn test_zero_allocation_cow_borrowed() {
    let snake = "user_account_id";
    let snake_res = to_snake_case(snake);
    assert!(matches!(snake_res, Cow::Borrowed(_)));

    let kebab = "user-account-id";
    let kebab_res = to_kebab_case(kebab);
    assert!(matches!(kebab_res, Cow::Borrowed(_)));

    let pascal = "UserAccountId";
    let pascal_res = to_pascal_case(pascal);
    assert!(matches!(pascal_res, Cow::Borrowed(_)));

    let camel = "userAccountId";
    let camel_res = to_camel_case(camel);
    assert!(matches!(camel_res, Cow::Borrowed(_)));

    let screaming = "USER_ACCOUNT_ID";
    let screaming_res = to_screaming_snake_case(screaming);
    assert!(matches!(screaming_res, Cow::Borrowed(_)));
}

#[test]
fn test_casing_predicates() {
    assert!("user_id".is_snake_case());
    assert!(!"userId".is_snake_case());
    assert!(!"User_Id".is_snake_case());

    assert!("userId".is_camel_case());
    assert!(!"UserId".is_camel_case());
    assert!(!"user_id".is_camel_case());

    assert!("UserId".is_pascal_case());
    assert!(!"userId".is_pascal_case());
    assert!(!"USER_ID".is_pascal_case());

    assert!("user-id".is_kebab_case());
    assert!(!"user_id".is_kebab_case());

    assert!("USER_ID".is_screaming_snake_case());
    assert!(!"user_id".is_screaming_snake_case());
}

#[test]
fn test_convention_helpers() {
    assert_eq!(tableize("UserProfile"), "user_profiles");
    assert_eq!(classify("user_profiles"), "UserProfile");
    assert_eq!(foreign_key("UserProfile"), "user_profile_id");

    assert_eq!(tableize("Person"), "people");
    assert_eq!(classify("people"), "Person");
    assert_eq!(foreign_key("Person"), "person_id");

    assert_eq!(tableize("Category"), "categories");
    assert_eq!(classify("categories"), "Category");
}

#[test]
fn test_all_str_ext_methods() {
    assert_eq!("user_id".to_screaming_snake_case(), "USER_ID");
    assert_eq!("user_id".to_kebab_case(), "user-id");
    assert_eq!("user_id".to_train_case(), "User-Id");
    assert_eq!("user_id".to_title_case(), "User Id");
    assert_eq!("user_id".to_sentence_case(), "User id");
    assert_eq!("user_id".to_pascal_case(), "UserId");
    assert_eq!("user_id".foreign_key(), "user_id_id");

    assert!("User-Id".is_train_case());
    assert!(!"user_id".is_train_case());

    assert!("User Id".is_title_case());
    assert!(!"user_id".is_title_case());

    assert!("User id".is_sentence_case());
    assert!(!"user_id".is_sentence_case());

    assert_eq!("box".pluralize_with_count(1), "box");
    assert_eq!("box".pluralize_with_count(2), "boxes");
    assert_eq!("box".pluralize_with_count_inclusive(3), "3 boxes");

    let trunc_opts = inflector::utils::TruncateOptions::new().with_suffix("..");
    assert_eq!(
        "long text here".truncate_with_options(6, &trunc_opts),
        "long.."
    );

    assert_eq!("one two three".words(2, "..."), "one two...");

    let excerpt_opts = inflector::utils::ExcerptOptions::new().with_radius(5);
    assert_eq!(
        "prefix match suffix".excerpt("match", &excerpt_opts),
        "...efix match suff..."
    );

    assert_eq!("value".wrap("(", ")"), "(value)");
    assert_eq!("(value)".unwrap("(", ")"), "value");

    let slug_opts = inflector::slug::SlugOptions::new().with_delimiter('_');
    assert_eq!(
        "Hello World".to_slug_with_options(&slug_opts),
        "hello_world"
    );
}
