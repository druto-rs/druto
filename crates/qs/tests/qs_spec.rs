use std::collections::{BTreeMap, HashMap};

use proptest::prelude::*;
use qs::{
    ArrayFormat, Duplicates, ParseOptions, QsError, QsValue, StringifyOptions, from_str,
    from_str_with_options, parse, parse_with_options, stringify, stringify_with_options, to_string,
    to_string_with_options, to_value,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
enum UserRole {
    Admin,
    Member,
    Guest,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct Pagination {
    page: u64,
    per_page: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct Filter {
    role: Option<UserRole>,
    active: bool,
    tags: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct UserQuery {
    search: String,
    filter: Filter,
    pagination: Pagination,
}

#[test]
fn test_parse_simple_flat_query() {
    let query = "name=John&city=Paris&active=true";
    let value = parse(query).expect("parsing should succeed");

    assert_eq!(value.get("name").and_then(QsValue::as_scalar), Some("John"));
    assert_eq!(
        value.get("city").and_then(QsValue::as_scalar),
        Some("Paris")
    );
    assert_eq!(
        value.get("active").and_then(QsValue::as_scalar),
        Some("true")
    );
}

#[test]
fn test_parse_empty_and_prefix_handling() {
    assert_eq!(parse("").unwrap(), QsValue::Object(BTreeMap::new()));
    assert_eq!(parse("?").unwrap(), QsValue::Object(BTreeMap::new()));

    let with_prefix = parse("?a=1&b=2").unwrap();
    assert_eq!(with_prefix.get("a").and_then(QsValue::as_scalar), Some("1"));
    assert_eq!(with_prefix.get("b").and_then(QsValue::as_scalar), Some("2"));

    let opts = ParseOptions::new().ignore_query_prefix(false);
    let without_strip = parse_with_options("?a=1", &opts).unwrap();
    assert_eq!(
        without_strip.get("?a").and_then(QsValue::as_scalar),
        Some("1")
    );
}

#[test]
fn test_parse_custom_delimiter() {
    let opts = ParseOptions::new().delimiter(b';');
    let value = parse_with_options("a=1;b=2;c=3", &opts).unwrap();

    assert_eq!(value.get("a").and_then(QsValue::as_scalar), Some("1"));
    assert_eq!(value.get("b").and_then(QsValue::as_scalar), Some("2"));
    assert_eq!(value.get("c").and_then(QsValue::as_scalar), Some("3"));
}

#[test]
fn test_parse_nested_objects() {
    let query = "user[profile][name]=John&user[profile][age]=30&user[address][city]=Paris";
    let value = parse(query).unwrap();

    let user = value.get("user").expect("user should exist");
    let profile = user.get("profile").expect("profile should exist");
    assert_eq!(
        profile.get("name").and_then(QsValue::as_scalar),
        Some("John")
    );
    assert_eq!(profile.get("age").and_then(QsValue::as_scalar), Some("30"));

    let address = user.get("address").expect("address should exist");
    assert_eq!(
        address.get("city").and_then(QsValue::as_scalar),
        Some("Paris")
    );
}

#[test]
fn test_parse_sequential_arrays() {
    let query = "ids[]=10&ids[]=20&ids[]=30";
    let value = parse(query).unwrap();

    let ids = value.get("ids").and_then(QsValue::as_array).unwrap();
    assert_eq!(ids.len(), 3);
    assert_eq!(ids[0].as_scalar(), Some("10"));
    assert_eq!(ids[1].as_scalar(), Some("20"));
    assert_eq!(ids[2].as_scalar(), Some("30"));
}

#[test]
fn test_parse_indexed_arrays() {
    let query = "items[0]=apple&items[1]=banana&items[2]=cherry";
    let value = parse(query).unwrap();

    let items = value.get("items").and_then(QsValue::as_array).unwrap();
    assert_eq!(items.len(), 3);
    assert_eq!(items[0].as_scalar(), Some("apple"));
    assert_eq!(items[1].as_scalar(), Some("banana"));
    assert_eq!(items[2].as_scalar(), Some("cherry"));
}

#[test]
fn test_parse_out_of_order_indexed_arrays() {
    let query = "items[1]=second&items[0]=first";
    let value = parse(query).unwrap();

    let items = value.get("items").and_then(QsValue::as_array).unwrap();
    assert_eq!(items.len(), 2);
    assert_eq!(items[0].as_scalar(), Some("first"));
    assert_eq!(items[1].as_scalar(), Some("second"));
}

#[test]
fn test_parse_arrays_of_objects() {
    let query = "users[0][name]=Alice&users[0][age]=25&users[1][name]=Bob&users[1][age]=30";
    let value = parse(query).unwrap();

    let users = value.get("users").and_then(QsValue::as_array).unwrap();
    assert_eq!(users.len(), 2);

    let alice = &users[0];
    assert_eq!(
        alice.get("name").and_then(QsValue::as_scalar),
        Some("Alice")
    );
    assert_eq!(alice.get("age").and_then(QsValue::as_scalar), Some("25"));

    let bob = &users[1];
    assert_eq!(bob.get("name").and_then(QsValue::as_scalar), Some("Bob"));
    assert_eq!(bob.get("age").and_then(QsValue::as_scalar), Some("30"));
}

#[test]
fn test_parse_dot_notation() {
    let opts = ParseOptions::new().allow_dots(true);
    let query = "user.name=John&user.profile.age=30&items.0=first";
    let value = parse_with_options(query, &opts).unwrap();

    let user = value.get("user").unwrap();
    assert_eq!(user.get("name").and_then(QsValue::as_scalar), Some("John"));

    let profile = user.get("profile").unwrap();
    assert_eq!(profile.get("age").and_then(QsValue::as_scalar), Some("30"));

    let items = value.get("items").and_then(QsValue::as_array).unwrap();
    assert_eq!(items[0].as_scalar(), Some("first"));
}

#[test]
fn test_parse_decode_dot_in_keys() {
    let opts = ParseOptions::new()
        .decode_dot_in_keys(true)
        .allow_dots(false);
    let query = "my%2Ekey=val";
    let value = parse_with_options(query, &opts).unwrap();

    assert_eq!(
        value.get("my.key").and_then(QsValue::as_scalar),
        Some("val")
    );
}

#[test]
fn test_parse_comma_separated_values() {
    let opts = ParseOptions::new().comma(true);
    let query = "tags=rust,web,systems";
    let value = parse_with_options(query, &opts).unwrap();

    let tags = value.get("tags").and_then(QsValue::as_array).unwrap();
    assert_eq!(tags.len(), 3);
    assert_eq!(tags[0].as_scalar(), Some("rust"));
    assert_eq!(tags[1].as_scalar(), Some("web"));
    assert_eq!(tags[2].as_scalar(), Some("systems"));
}

#[test]
fn test_parse_strict_null_handling() {
    let strict_opts = ParseOptions::new().strict_null_handling(true);
    let value = parse_with_options("a&b=&c=val", &strict_opts).unwrap();

    assert_eq!(value.get("a"), Some(&QsValue::Null));
    assert_eq!(value.get("b").and_then(QsValue::as_scalar), Some(""));
    assert_eq!(value.get("c").and_then(QsValue::as_scalar), Some("val"));

    let non_strict_opts = ParseOptions::new().strict_null_handling(false);
    let non_strict_val = parse_with_options("a&b=&c=val", &non_strict_opts).unwrap();
    assert_eq!(
        non_strict_val.get("a").and_then(QsValue::as_scalar),
        Some("")
    );
}

#[test]
fn test_parse_depth_limit_lenient_default() {
    let query = "a[b][c][d][e][f][g]=overflow";
    let value = parse(query).unwrap();

    let a = value.get("a").unwrap();
    let b = a.get("b").unwrap();
    let c = b.get("c").unwrap();
    let d = c.get("d").unwrap();
    let e = d.get("e").unwrap();
    assert!(e.get("[g]").is_some() || e.get("f").is_some());
}

#[test]
fn test_parse_depth_limit_strict_error() {
    let opts = ParseOptions::new().depth(2).strict_depth(true);
    let query = "a[b][c][d]=val";
    let result = parse_with_options(query, &opts);

    assert_eq!(result, Err(QsError::DepthLimitExceeded(2)));
}

#[test]
fn test_parse_depth_zero_treats_as_flat() {
    let opts = ParseOptions::new().depth(0);
    let query = "a[b][c]=val";
    let value = parse_with_options(query, &opts).unwrap();

    assert_eq!(
        value.get("a[b][c]").and_then(QsValue::as_scalar),
        Some("val")
    );
}

#[test]
fn test_parse_parameter_limit_exceeded() {
    let opts = ParseOptions::new().parameter_limit(3);
    let query = "a=1&b=2&c=3&d=4";
    let result = parse_with_options(query, &opts);

    assert_eq!(result, Err(QsError::ParameterLimitExceeded(3)));
}

#[test]
fn test_parse_array_limit_defense_against_sparse_bombs() {
    let query = "items[100]=val&items[5]=small";
    let value = parse(query).unwrap();

    let items = value.get("items").unwrap();
    assert!(matches!(items, QsValue::Object(_)));
    assert_eq!(items.get("100").and_then(QsValue::as_scalar), Some("val"));
    assert_eq!(items.get("5").and_then(QsValue::as_scalar), Some("small"));
}

#[test]
fn test_parse_sparse_array_compaction() {
    let query = "items[2]=third&items[5]=sixth";
    let value = parse(query).unwrap();

    let items = value.get("items").and_then(QsValue::as_array).unwrap();
    assert_eq!(items.len(), 2);
    assert_eq!(items[0].as_scalar(), Some("third"));
    assert_eq!(items[1].as_scalar(), Some("sixth"));

    let sparse_opts = ParseOptions::new().allow_sparse(true);
    let sparse_val = parse_with_options(query, &sparse_opts).unwrap();
    let sparse_items = sparse_val.get("items").and_then(QsValue::as_array).unwrap();
    assert_eq!(sparse_items.len(), 6);
    assert_eq!(sparse_items[0], QsValue::Null);
    assert_eq!(sparse_items[2].as_scalar(), Some("third"));
    assert_eq!(sparse_items[5].as_scalar(), Some("sixth"));
}

#[test]
fn test_parse_duplicates_strategies() {
    let query = "tag=rust&tag=web&tag=api";

    let combined = parse(query).unwrap();
    let tags = combined.get("tag").and_then(QsValue::as_array).unwrap();
    assert_eq!(tags.len(), 3);
    assert_eq!(tags[0].as_scalar(), Some("rust"));
    assert_eq!(tags[1].as_scalar(), Some("web"));
    assert_eq!(tags[2].as_scalar(), Some("api"));

    let first_opts = ParseOptions::new().duplicates(Duplicates::First);
    let first_val = parse_with_options(query, &first_opts).unwrap();
    assert_eq!(
        first_val.get("tag").and_then(QsValue::as_scalar),
        Some("rust")
    );

    let last_opts = ParseOptions::new().duplicates(Duplicates::Last);
    let last_val = parse_with_options(query, &last_opts).unwrap();
    assert_eq!(
        last_val.get("tag").and_then(QsValue::as_scalar),
        Some("api")
    );
}

#[test]
fn test_rfc3986_query_space_and_plus_handling() {
    let query = "greeting=hello+world&math=1%2B1%3D2";
    let value = parse(query).unwrap();

    assert_eq!(
        value.get("greeting").and_then(QsValue::as_scalar),
        Some("hello world")
    );
    assert_eq!(
        value.get("math").and_then(QsValue::as_scalar),
        Some("1+1=2")
    );
}

#[test]
fn test_percent_encoded_brackets_in_keys() {
    let query = "user%5Bname%5D=John&user%5Bage%5D=30";
    let value = parse(query).unwrap();

    let user = value.get("user").unwrap();
    assert_eq!(user.get("name").and_then(QsValue::as_scalar), Some("John"));
    assert_eq!(user.get("age").and_then(QsValue::as_scalar), Some("30"));
}

#[test]
fn test_bracket_with_equals_sign_inside() {
    let query = "filter[=field]=val";
    let value = parse(query).unwrap();

    let filter = value.get("filter").unwrap();
    assert_eq!(
        filter.get("=field").and_then(QsValue::as_scalar),
        Some("val")
    );
}

#[test]
fn test_unclosed_bracket_key() {
    let query = "data[open=123";
    let value = parse(query).unwrap();

    assert!(value.get("data").is_some() || value.get("data[open").is_some());
}

#[test]
fn test_parse_empty_brackets_with_nested_tail() {
    let query = "users[][name]=Alice&users[][name]=Bob";
    let value = parse(query).unwrap();

    let users = value.get("users").and_then(QsValue::as_array).unwrap();
    assert_eq!(users.len(), 2);
    assert_eq!(
        users[0].get("name").and_then(QsValue::as_scalar),
        Some("Alice")
    );
    assert_eq!(
        users[1].get("name").and_then(QsValue::as_scalar),
        Some("Bob")
    );
}

#[test]
fn test_parse_array_to_object_conversion_when_field_added() {
    let query = "items[0]=first&items[custom]=second";
    let value = parse(query).unwrap();

    let items = value.get("items").unwrap();
    assert!(matches!(items, QsValue::Object(_)));
    assert_eq!(items.get("0").and_then(QsValue::as_scalar), Some("first"));
    assert_eq!(
        items.get("custom").and_then(QsValue::as_scalar),
        Some("second")
    );
}

#[test]
fn test_parse_duplicates_combine_with_arrays() {
    let query = "tags[]=a&tags=b&tags[]=c";
    let value = parse(query).unwrap();

    let tags = value.get("tags").and_then(QsValue::as_array).unwrap();
    assert_eq!(tags.len(), 3);
    assert_eq!(tags[0].as_scalar(), Some("a"));
    assert_eq!(tags[1].as_scalar(), Some("b"));
    assert_eq!(tags[2].as_scalar(), Some("c"));
}

#[test]
fn test_stringify_array_formats() {
    let mut map = BTreeMap::new();
    map.insert(
        "ids".to_string(),
        QsValue::Array(vec![QsValue::from("1"), QsValue::from("2")]),
    );
    let val = QsValue::Object(map);

    let indices = stringify_with_options(
        &val,
        &StringifyOptions::new()
            .array_format(ArrayFormat::Indices)
            .encode(false),
    );
    assert_eq!(indices, "ids[0]=1&ids[1]=2");

    let brackets = stringify_with_options(
        &val,
        &StringifyOptions::new()
            .array_format(ArrayFormat::Brackets)
            .encode(false),
    );
    assert_eq!(brackets, "ids[]=1&ids[]=2");

    let repeat = stringify_with_options(
        &val,
        &StringifyOptions::new()
            .array_format(ArrayFormat::Repeat)
            .encode(false),
    );
    assert_eq!(repeat, "ids=1&ids=2");

    let comma = stringify_with_options(
        &val,
        &StringifyOptions::new()
            .array_format(ArrayFormat::Comma)
            .encode(false),
    );
    assert_eq!(comma, "ids=1,2");
}

#[test]
fn test_stringify_deterministic_alphabetical_key_sorting() {
    let mut map = BTreeMap::new();
    map.insert("zebra".to_string(), QsValue::from("stripes"));
    map.insert("apple".to_string(), QsValue::from("fruit"));
    map.insert("monkey".to_string(), QsValue::from("banana"));
    let val = QsValue::Object(map);

    let result = stringify(&val);
    assert_eq!(result, "apple=fruit&monkey=banana&zebra=stripes");
}

#[test]
fn test_stringify_encode_values_only() {
    let mut inner = BTreeMap::new();
    inner.insert("full name".to_string(), QsValue::from("John Doe"));
    let mut outer = BTreeMap::new();
    outer.insert("user".to_string(), QsValue::Object(inner));
    let val = QsValue::Object(outer);

    let opts = StringifyOptions::new()
        .encode(true)
        .encode_values_only(true);
    let result = stringify_with_options(&val, &opts);
    assert_eq!(result, "user[full%20name]=John%20Doe");
}

#[test]
fn test_stringify_query_prefix_and_null_handling() {
    let mut map = BTreeMap::new();
    map.insert("empty".to_string(), QsValue::Null);
    map.insert("name".to_string(), QsValue::from("John"));
    let val = QsValue::Object(map);

    let default_str = stringify(&val);
    assert_eq!(default_str, "empty=&name=John");

    let skip_opts = StringifyOptions::new().skip_nulls(true);
    assert_eq!(stringify_with_options(&val, &skip_opts), "name=John");

    let strict_opts = StringifyOptions::new().strict_null_handling(true);
    assert_eq!(
        stringify_with_options(&val, &strict_opts),
        "empty&name=John"
    );

    let prefix_opts = StringifyOptions::new().add_query_prefix(true);
    assert_eq!(
        stringify_with_options(&val, &prefix_opts),
        "?empty=&name=John"
    );
}

#[test]
fn test_stringify_allow_dots() {
    let mut inner = BTreeMap::new();
    inner.insert("city".to_string(), QsValue::from("Paris"));
    let mut outer = BTreeMap::new();
    outer.insert("address".to_string(), QsValue::Object(inner));
    let val = QsValue::Object(outer);

    let opts = StringifyOptions::new().allow_dots(true).encode(false);
    assert_eq!(stringify_with_options(&val, &opts), "address.city=Paris");
}

#[test]
fn test_stringify_empty_value_and_empty_array() {
    assert_eq!(stringify(&QsValue::Null), "");
    assert_eq!(stringify(&QsValue::Scalar("lone".to_string())), "");

    let mut map = BTreeMap::new();
    map.insert("empty_arr".to_string(), QsValue::Array(Vec::new()));
    let val = QsValue::Object(map);
    assert_eq!(stringify(&val), "");
}

#[test]
fn test_serde_all_integers_and_floats() {
    #[derive(Debug, PartialEq, Eq, Deserialize, Serialize)]
    struct AllInts {
        i8_v: i8,
        i16_v: i16,
        i32_v: i32,
        i64_v: i64,
        i128_v: i128,
        u8_v: u8,
        u16_v: u16,
        u32_v: u32,
        u64_v: u64,
        u128_v: u128,
    }

    let expected = AllInts {
        i8_v: -128,
        i16_v: -32768,
        i32_v: -2147483648,
        i64_v: -9223372036854775808,
        i128_v: -170141183460469231731687303715884105728,
        u8_v: 255,
        u16_v: 65535,
        u32_v: 4294967295,
        u64_v: 18446744073709551615,
        u128_v: 340282366920938463463374607431768211455,
    };

    let serialized = to_string(&expected).expect("serialization succeeds");
    let deserialized: AllInts = from_str(&serialized).expect("deserialization succeeds");
    assert_eq!(deserialized, expected);
}

#[test]
fn test_serde_primitive_type_coercion() {
    #[derive(Debug, PartialEq, Deserialize, Serialize)]
    struct Primitives {
        u8_val: u8,
        i16_val: i16,
        u32_val: u32,
        i64_val: i64,
        f32_val: f32,
        f64_val: f64,
        bool_true: bool,
        bool_false: bool,
        char_val: char,
        opt_some: Option<u32>,
        opt_none: Option<u32>,
    }

    let query = "u8_val=255&i16_val=-32000&u32_val=4200000000&i64_val=-9000000000000000000&f32_val=12.5&f64_val=45.75&bool_true=yes&bool_false=0&char_val=Z&opt_some=99&opt_none=";

    let opts = ParseOptions::new().strict_null_handling(true);
    let parsed: Primitives =
        from_str_with_options(query, &opts).expect("deserialization should succeed");

    assert_eq!(parsed.u8_val, 255);
    assert_eq!(parsed.i16_val, -32000);
    assert_eq!(parsed.u32_val, 4_200_000_000);
    assert_eq!(parsed.i64_val, -9_000_000_000_000_000_000);
    assert!((parsed.f32_val - 12.5).abs() < 1e-4);
    assert!((parsed.f64_val - 45.75).abs() < 1e-8);
    assert!(parsed.bool_true);
    assert!(!parsed.bool_false);
    assert_eq!(parsed.char_val, 'Z');
    assert_eq!(parsed.opt_some, Some(99));
    assert_eq!(parsed.opt_none, None);
}

#[test]
fn test_serde_nested_struct_deserialization_roundtrip() {
    let expected = UserQuery {
        search: "rust ace".to_string(),
        filter: Filter {
            role: Some(UserRole::Admin),
            active: true,
            tags: vec!["druto".to_string(), "framework".to_string()],
        },
        pagination: Pagination {
            page: 1,
            per_page: 25,
        },
    };

    let serialized = to_string_with_options(&expected, &StringifyOptions::default())
        .expect("serialization should succeed");
    let deserialized: UserQuery = from_str(&serialized).expect("deserialization should succeed");

    assert_eq!(deserialized, expected);
}

#[test]
fn test_serde_unit_and_newtype_and_tuple_structs() {
    #[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
    struct Wrapper {
        id: MyId,
        pos: Point,
        unit: MyUnit,
    }

    #[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
    struct MyId(u64);

    #[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
    struct Point(i32, i32);

    #[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
    struct MyUnit;

    let expected = Wrapper {
        id: MyId(42),
        pos: Point(10, -20),
        unit: MyUnit,
    };

    let serialized = to_string(&expected).expect("serialization succeeds");
    let deserialized: Wrapper = from_str(&serialized).expect("deserialization succeeds");
    assert_eq!(deserialized, expected);
}

#[test]
fn test_serde_enum_all_variant_types() {
    #[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
    enum Message {
        Quit,
        Echo(String),
        Move(i32, i32),
        Write { text: String, count: u32 },
    }

    #[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
    struct Envelope {
        msg: Message,
    }

    // Unit variant
    let e_quit = Envelope { msg: Message::Quit };
    let s_quit = to_string(&e_quit).unwrap();
    assert_eq!(from_str::<Envelope>(&s_quit).unwrap(), e_quit);

    // Newtype variant
    let e_echo = Envelope {
        msg: Message::Echo("hello".to_string()),
    };
    let s_echo = to_string(&e_echo).unwrap();
    assert_eq!(from_str::<Envelope>(&s_echo).unwrap(), e_echo);

    // Tuple variant
    let e_move = Envelope {
        msg: Message::Move(15, 30),
    };
    let s_move = to_string(&e_move).unwrap();
    assert_eq!(from_str::<Envelope>(&s_move).unwrap(), e_move);

    // Struct variant
    let e_write = Envelope {
        msg: Message::Write {
            text: "doc".to_string(),
            count: 3,
        },
    };
    let s_write = to_string(&e_write).unwrap();
    assert_eq!(from_str::<Envelope>(&s_write).unwrap(), e_write);
}

#[test]
fn test_serde_maps() {
    let mut map = HashMap::new();
    map.insert("first".to_string(), 10u32);
    map.insert("second".to_string(), 20u32);

    let serialized = to_string(&map).unwrap();
    let deserialized: HashMap<String, u32> = from_str(&serialized).unwrap();
    assert_eq!(deserialized, map);
}

#[test]
fn test_serde_deserialization_error_cases() {
    #[derive(Debug, Deserialize, PartialEq, Eq)]
    struct NumberHolder {
        num: u32,
    }

    let bad_number: Result<NumberHolder, _> = from_str("num=not_a_number");
    assert!(bad_number.is_err());

    #[derive(Debug, Deserialize, PartialEq, Eq)]
    struct BoolHolder {
        flag: bool,
    }

    let bad_bool: Result<BoolHolder, _> = from_str("flag=maybe");
    assert!(bad_bool.is_err());

    #[derive(Debug, Deserialize, PartialEq, Eq)]
    struct CharHolder {
        ch: char,
    }

    let bad_char: Result<CharHolder, _> = from_str("ch=too_long");
    assert!(bad_char.is_err());
    let empty_char: Result<CharHolder, _> = from_str("ch=");
    assert!(empty_char.is_err());

    #[derive(Debug, Deserialize)]
    struct FloatHolder {
        num: f64,
    }
    let good_float: FloatHolder = from_str("num=12.5").unwrap();
    assert!((good_float.num - 12.5).abs() < f64::EPSILON);
    let bad_float: Result<FloatHolder, _> = from_str("num=xyz");
    assert!(bad_float.is_err());

    #[derive(Debug, Deserialize)]
    struct Int128Holder {
        signed_num: i128,
    }
    let good_i128: Int128Holder = from_str("signed_num=42").unwrap();
    assert_eq!(good_i128.signed_num, 42);
    let bad_i128: Result<Int128Holder, _> = from_str("signed_num=huge_nan");
    assert!(bad_i128.is_err());

    #[derive(Debug, Deserialize)]
    struct Uint128Holder {
        unsigned_num: u128,
    }
    let good_u128: Uint128Holder = from_str("unsigned_num=42").unwrap();
    assert_eq!(good_u128.unsigned_num, 42);
    let bad_u128: Result<Uint128Holder, _> = from_str("unsigned_num=-5");
    assert!(bad_u128.is_err());
}

#[test]
fn test_qs_value_accessor_methods_and_conversions() {
    let scalar = QsValue::from("hello");
    assert!(!scalar.is_null());
    assert_eq!(scalar.as_scalar(), Some("hello"));
    assert_eq!(scalar.as_array(), None);
    assert_eq!(scalar.as_object(), None);
    assert_eq!(scalar.get("foo"), None);
    assert_eq!(scalar.get_index(0), None);

    let from_string = QsValue::from("string_val".to_string());
    assert_eq!(from_string.as_scalar(), Some("string_val"));

    let mut array = QsValue::from(vec![QsValue::from("first"), QsValue::from("second")]);
    assert_eq!(array.as_array().map(<[QsValue]>::len), Some(2));
    assert_eq!(
        array.get_index(1).and_then(QsValue::as_scalar),
        Some("second")
    );
    assert_eq!(array.get_index(10), None);
    assert!(array.as_array_mut().is_some());
    assert_eq!(array.as_scalar(), None);

    let mut btree = BTreeMap::new();
    btree.insert("key".to_string(), QsValue::from("val"));
    let mut object = QsValue::from(btree);
    if let Some(map) = object.as_object_mut() {
        map.insert("another".to_string(), QsValue::from("second_val"));
    }
    assert_eq!(object.get("key").and_then(QsValue::as_scalar), Some("val"));
    assert_eq!(object.get("missing"), None);
    assert_eq!(object.as_scalar(), None);
    assert!(object.as_object().is_some());

    let null_val = QsValue::Null;
    assert!(null_val.is_null());
    assert_eq!(null_val.as_scalar(), None);
    assert_eq!(null_val.as_array(), None);
    assert_eq!(null_val.as_object(), None);
}

#[test]
fn test_options_builders_and_getters() {
    let parse_opts = ParseOptions::new()
        .depth(8)
        .strict_depth(true)
        .parameter_limit(500)
        .array_limit(50)
        .allow_sparse(true)
        .allow_dots(true)
        .comma(true)
        .duplicates(Duplicates::Last)
        .ignore_query_prefix(false)
        .strict_null_handling(true)
        .delimiter(b';')
        .decode_dot_in_keys(true);

    assert_eq!(parse_opts.get_depth(), 8);
    assert!(parse_opts.get_strict_depth());
    assert_eq!(parse_opts.get_parameter_limit(), 500);
    assert_eq!(parse_opts.get_array_limit(), 50);
    assert!(parse_opts.get_allow_sparse());
    assert!(parse_opts.get_allow_dots());
    assert!(parse_opts.get_comma());
    assert_eq!(parse_opts.get_duplicates(), Duplicates::Last);
    assert!(!parse_opts.get_ignore_query_prefix());
    assert!(parse_opts.get_strict_null_handling());
    assert_eq!(parse_opts.get_delimiter(), b';');
    assert!(parse_opts.get_decode_dot_in_keys());

    let str_opts = StringifyOptions::new()
        .array_format(ArrayFormat::Repeat)
        .encode(false)
        .encode_values_only(true)
        .add_query_prefix(true)
        .skip_nulls(true)
        .strict_null_handling(true)
        .sort(false)
        .delimiter(b';')
        .allow_dots(true);

    assert_eq!(str_opts.get_array_format(), ArrayFormat::Repeat);
    assert!(!str_opts.get_encode());
    assert!(str_opts.get_encode_values_only());
    assert!(str_opts.get_add_query_prefix());
    assert!(str_opts.get_skip_nulls());
    assert!(str_opts.get_strict_null_handling());
    assert!(!str_opts.get_sort());
    assert_eq!(str_opts.get_delimiter(), b';');
    assert!(str_opts.get_allow_dots());
}

#[test]
fn test_qs_error_display_and_serde_errors() {
    let err_depth = QsError::DepthLimitExceeded(5);
    assert!(err_depth.to_string().contains("depth limit exceeded"));

    let err_param = QsError::ParameterLimitExceeded(100);
    assert!(err_param.to_string().contains("parameter limit exceeded"));

    let err_array = QsError::ArrayLimitExceeded(20);
    assert!(err_array.to_string().contains("array limit exceeded"));

    let err_syntax = QsError::InvalidSyntax("malformed bracket".to_string());
    assert!(
        err_syntax
            .to_string()
            .contains("invalid query string syntax")
    );

    let err_custom = QsError::Custom("something went wrong".to_string());
    assert_eq!(err_custom.to_string(), "something went wrong");

    let de_err: QsError = <QsError as serde::de::Error>::custom("custom de error");
    assert_eq!(de_err, QsError::Custom("custom de error".to_string()));

    let ser_err: QsError = <QsError as serde::ser::Error>::custom("custom ser error");
    assert_eq!(ser_err, QsError::Custom("custom ser error".to_string()));
}

#[test]
fn test_serde_direct_qs_value_roundtrip() {
    let mut map = BTreeMap::new();
    map.insert("name".to_string(), QsValue::from("John"));
    map.insert(
        "scores".to_string(),
        QsValue::Array(vec![QsValue::from("10"), QsValue::from("20")]),
    );
    let original = QsValue::Object(map);

    let val = to_value(&original).unwrap();
    assert_eq!(val, original);

    // Roundtrip through serde_json to test serialize and deserialize impls for QsValue
    let json = serde_json::to_string(&original).unwrap();
    let from_json: QsValue = serde_json::from_str(&json).unwrap();
    assert_eq!(from_json, original);
}

proptest! {
    #[test]
    fn proptest_rfc3986_component_roundtrip(s in "[a-zA-Z0-9 _.~!@#$%^&*()-+=]{1,64}") {
        let encoded = qs::encoding::encode_component(&s);
        let decoded = qs::encoding::decode_component(&encoded);
        prop_assert_eq!(decoded, s);
    }

    #[test]
    fn proptest_serde_roundtrip_flat_struct(
        name in "[a-zA-Z0-9]{1,32}",
        age in 0u32..1000u32,
        active in any::<bool>()
    ) {
        #[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
        struct Person {
            name: String,
            age: u32,
            active: bool,
        }

        let p = Person { name, age, active };
        let serialized = to_string(&p).expect("serialization succeeds");
        let deserialized: Person = from_str(&serialized).expect("deserialization succeeds");
        prop_assert_eq!(deserialized, p);
    }
}

#[test]
fn test_serde_de_fallback_and_error_paths() {
    use serde::de::Deserializer;

    // deserialize_unit with non-unit
    let scalar = QsValue::from("not_unit");
    let err = scalar.deserialize_unit(serde::de::IgnoredAny);
    assert!(err.is_err());

    // deserialize_map with non-object
    let not_map = QsValue::from("not_map");
    let err_map = not_map.deserialize_map(serde::de::IgnoredAny);
    assert!(err_map.is_err());

    // deserialize_enum with invalid shape
    let mut multi_map = BTreeMap::new();
    multi_map.insert("one".to_string(), QsValue::from("1"));
    multi_map.insert("two".to_string(), QsValue::from("2"));
    let err_enum = QsValue::Object(multi_map).deserialize_enum("Test", &[], serde::de::IgnoredAny);
    assert!(err_enum.is_err());

    let array_enum = QsValue::Array(vec![]).deserialize_enum("Test", &[], serde::de::IgnoredAny);
    assert!(array_enum.is_err());

    // Non-scalar fallbacks for bool, char, str, bytes, i64, u64, f64
    let null_val = QsValue::Null;
    assert!(
        null_val
            .clone()
            .deserialize_bool(serde::de::IgnoredAny)
            .is_ok()
    );
    assert!(
        null_val
            .clone()
            .deserialize_char(serde::de::IgnoredAny)
            .is_ok()
    );
    assert!(
        null_val
            .clone()
            .deserialize_str(serde::de::IgnoredAny)
            .is_ok()
    );
    assert!(
        null_val
            .clone()
            .deserialize_bytes(serde::de::IgnoredAny)
            .is_ok()
    );
    assert!(
        null_val
            .clone()
            .deserialize_i64(serde::de::IgnoredAny)
            .is_ok()
    );
    assert!(
        null_val
            .clone()
            .deserialize_u64(serde::de::IgnoredAny)
            .is_ok()
    );
    assert!(
        null_val
            .clone()
            .deserialize_f64(serde::de::IgnoredAny)
            .is_ok()
    );
    assert!(
        null_val
            .clone()
            .deserialize_i128(serde::de::IgnoredAny)
            .is_ok()
    );
    assert!(
        null_val
            .clone()
            .deserialize_u128(serde::de::IgnoredAny)
            .is_ok()
    );

    // deserialize_seq with Null and scalar
    assert!(null_val.deserialize_seq(serde::de::IgnoredAny).is_ok());
    assert!(
        QsValue::from("single")
            .deserialize_seq(serde::de::IgnoredAny)
            .is_ok()
    );

    // deserialize_bytes with array of valid u8 bytes and invalid item
    let valid_bytes = QsValue::Array(vec![QsValue::from("65"), QsValue::from("66")]);
    assert!(valid_bytes.deserialize_bytes(serde::de::IgnoredAny).is_ok());

    let invalid_byte_num = QsValue::Array(vec![QsValue::from("999")]);
    assert!(
        invalid_byte_num
            .deserialize_bytes(serde::de::IgnoredAny)
            .is_err()
    );

    let invalid_byte_type = QsValue::Array(vec![QsValue::Null]);
    assert!(
        invalid_byte_type
            .deserialize_bytes(serde::de::IgnoredAny)
            .is_err()
    );
}

#[test]
fn test_serde_variant_access_error_paths() {
    #[derive(Debug, PartialEq, Eq, Deserialize)]
    enum BadEnum {
        Unit,
        Newtype(u32),
        Tuple(u32, u32),
        Struct { a: u32 },
    }

    // Attempting to deserialize a unit variant from a map enum
    let mut map = BTreeMap::new();
    map.insert("Unit".to_string(), QsValue::Null);
    let parsed: Result<BadEnum, _> = from_str("Unit=");
    assert_eq!(parsed.unwrap(), BadEnum::Unit);

    // Attempting invalid variant deserializations from scalar
    let newtype_from_scalar: Result<BadEnum, _> = from_str("Newtype");
    assert!(newtype_from_scalar.is_err());

    let tuple_from_scalar: Result<BadEnum, _> = from_str("Tuple");
    assert!(tuple_from_scalar.is_err());

    let struct_from_scalar: Result<BadEnum, _> = from_str("Struct");
    assert!(struct_from_scalar.is_err());
}

#[test]
fn test_serde_ser_map_with_non_string_key_error() {
    let mut map = HashMap::new();
    map.insert(vec![10u32, 20u32], "ten".to_string());

    let err = to_string(&map);
    assert!(err.is_err());
}

#[test]
fn test_serde_ser_bytes() {
    let val = to_value(&[10u8, 20u8, 30u8]).unwrap();
    assert_eq!(
        val,
        QsValue::Array(vec![
            QsValue::from("10"),
            QsValue::from("20"),
            QsValue::from("30"),
        ])
    );
}

#[test]
fn test_parser_combining_arrays_and_scalars_all_branches() {
    // Array + scalar
    let q1 = "items[]=1&items=2";
    let v1 = parse(q1).unwrap();
    let arr1 = v1.get("items").and_then(QsValue::as_array).unwrap();
    assert_eq!(arr1.len(), 2);
    assert_eq!(arr1[0].as_scalar(), Some("1"));
    assert_eq!(arr1[1].as_scalar(), Some("2"));

    // Scalar + array
    let q2 = "items=1&items[]=2";
    let v2 = parse(q2).unwrap();
    let arr2 = v2.get("items").and_then(QsValue::as_array).unwrap();
    assert_eq!(arr2.len(), 2);
    assert_eq!(arr2[0].as_scalar(), Some("1"));
    assert_eq!(arr2[1].as_scalar(), Some("2"));

    // Array + comma array
    let opts = ParseOptions::new().comma(true);
    let q3 = "items[]=1&items=2,3";
    let v3 = parse_with_options(q3, &opts).unwrap();
    let arr3 = v3.get("items").and_then(QsValue::as_array).unwrap();
    assert_eq!(arr3.len(), 3);
    assert_eq!(arr3[0].as_scalar(), Some("1"));
    assert_eq!(arr3[1].as_scalar(), Some("2"));
    assert_eq!(arr3[2].as_scalar(), Some("3"));

    // Scalar + comma array
    let q4 = "items=1&items=2,3";
    let v4 = parse_with_options(q4, &opts).unwrap();
    let arr4 = v4.get("items").and_then(QsValue::as_array).unwrap();
    assert_eq!(arr4.len(), 3);
}

#[test]
fn test_stringify_root_arrays() {
    let arr = QsValue::Array(vec![QsValue::from("a"), QsValue::from("b")]);

    let indices = stringify_with_options(
        &arr,
        &StringifyOptions::new()
            .array_format(ArrayFormat::Indices)
            .encode(false),
    );
    assert_eq!(indices, "[0]=a&[1]=b");

    let brackets = stringify_with_options(
        &arr,
        &StringifyOptions::new()
            .array_format(ArrayFormat::Brackets)
            .encode(false),
    );
    assert_eq!(brackets, "[]=a&[]=b");
}

#[test]
fn test_value_negative_mut_accessors() {
    let mut scalar = QsValue::from("hello");
    assert!(scalar.as_array_mut().is_none());
    assert!(scalar.as_object_mut().is_none());
    assert!(scalar.as_object().is_none());
}

#[test]
fn test_serde_ser_direct_calls() {
    let s = to_value(&'X').unwrap();
    assert_eq!(s, QsValue::Scalar("X".to_string()));

    #[derive(Serialize)]
    struct Unit;
    let u = to_value(&Unit).unwrap();
    assert_eq!(u, QsValue::Null);

    let opt_none: Option<u32> = None;
    let n = to_value(&opt_none).unwrap();
    assert_eq!(n, QsValue::Null);

    // QsValue::Null serialization
    let null_ser = to_value(&QsValue::Null).unwrap();
    assert_eq!(null_ser, QsValue::Null);

    let map_ser = to_value(&BTreeMap::<String, String>::new()).unwrap();
    let _ = map_ser;
}

#[test]
fn test_serde_de_direct_calls_and_visitor() {
    use serde::de::Deserializer;

    // Unit with empty string
    let empty_str = QsValue::from("");
    assert!(empty_str.deserialize_unit(serde::de::IgnoredAny).is_ok());

    // Newtype struct and tuple
    #[derive(Debug, PartialEq, Eq, Deserialize)]
    struct Id(u64);
    let id_val = QsValue::from("123");
    let parsed_id: Id = Id::deserialize(id_val).unwrap();
    assert_eq!(parsed_id, Id(123));

    #[derive(Debug, PartialEq, Eq, Deserialize)]
    struct Pair(u32, u32);
    let pair_val = QsValue::Array(vec![QsValue::from("1"), QsValue::from("2")]);
    let parsed_pair: Pair = Pair::deserialize(pair_val).unwrap();
    assert_eq!(parsed_pair, Pair(1, 2));

    // deserialize_bytes on scalar
    let scalar_bytes = QsValue::from("hello");
    assert!(
        scalar_bytes
            .deserialize_bytes(serde::de::IgnoredAny)
            .is_ok()
    );
}

#[test]
fn test_parser_stray_chars_and_nested_dots() {
    let q = "a[b]c=1&x.y[z].w=2";
    let val = parse_with_options(q, &ParseOptions::new().allow_dots(true)).unwrap();

    let a = val.get("a").unwrap();
    assert!(a.get("b").is_some());

    let x = val.get("x").unwrap();
    let y = x.get("y").unwrap();
    let z = y.get("z").unwrap();
    assert_eq!(z.get("w").and_then(QsValue::as_scalar), Some("2"));
}

#[test]
fn test_serde_json_to_qs_value_visitor_all_types() {
    let val_i64: QsValue = serde_json::from_str("-42").unwrap();
    assert_eq!(val_i64, QsValue::Scalar("-42".to_string()));

    let val_u64: QsValue = serde_json::from_str("42").unwrap();
    assert_eq!(val_u64, QsValue::Scalar("42".to_string()));

    let val_f64: QsValue = serde_json::from_str("12.5").unwrap();
    assert_eq!(val_f64, QsValue::Scalar("12.5".to_string()));

    let val_bool: QsValue = serde_json::from_str("true").unwrap();
    assert_eq!(val_bool, QsValue::Scalar("true".to_string()));

    let val_str: QsValue = serde_json::from_str("\"hello\"").unwrap();
    assert_eq!(val_str, QsValue::Scalar("hello".to_string()));

    let val_null: QsValue = serde_json::from_str("null").unwrap();
    assert_eq!(val_null, QsValue::Null);

    let val_seq: QsValue = serde_json::from_str("[1, 2]").unwrap();
    assert!(matches!(val_seq, QsValue::Array(_)));

    let val_map: QsValue = serde_json::from_str("{\"a\": 1}").unwrap();
    assert!(matches!(val_map, QsValue::Object(_)));
}

#[test]
fn test_parse_key_starting_with_bracket() {
    let q = "[0]=a&[1]=b";
    let val = parse(q).unwrap();
    assert_eq!(val.get("0").and_then(QsValue::as_scalar), Some("a"));
    assert_eq!(val.get("1").and_then(QsValue::as_scalar), Some("b"));

    let q_empty = "[]=x";
    let val_empty = parse(q_empty).unwrap();
    let arr = val_empty.as_array().unwrap();
    assert!(arr.contains(&QsValue::Scalar("x".to_string())));
}

#[test]
fn test_parse_scalar_to_array_on_indexed_or_bracket() {
    let q = "a=foo&a[1]=bar";
    let val = parse(q).unwrap();
    let a = val.get("a").and_then(QsValue::as_array).unwrap();
    assert_eq!(a[0].as_scalar(), Some("foo"));
    assert_eq!(a[1].as_scalar(), Some("bar"));

    let q2 = "b=foo&b[]=bar";
    let val2 = parse(q2).unwrap();
    let b = val2.get("b").and_then(QsValue::as_array).unwrap();
    assert_eq!(b[0].as_scalar(), Some("foo"));
    assert_eq!(b[1].as_scalar(), Some("bar"));
}

#[test]
fn test_parse_empty_bracket_with_nested_tail() {
    let q = "users[][name]=Alice&users[][name]=Bob";
    let val = parse(q).unwrap();
    let users = val.get("users").and_then(QsValue::as_array).unwrap();
    assert_eq!(users.len(), 2);
    assert_eq!(
        users[0].get("name").and_then(QsValue::as_scalar),
        Some("Alice")
    );
    assert_eq!(
        users[1].get("name").and_then(QsValue::as_scalar),
        Some("Bob")
    );
}

#[test]
fn test_parse_sparse_array_converted_to_object_with_nulls() {
    let q = "items[2]=third&items[custom]=second";
    let val = parse(q).unwrap();
    let items = val.get("items").unwrap();
    assert!(matches!(items, QsValue::Object(_)));
    assert_eq!(items.get("2").and_then(QsValue::as_scalar), Some("third"));
    assert_eq!(
        items.get("custom").and_then(QsValue::as_scalar),
        Some("second")
    );
}

#[test]
fn test_parse_empty_key_ignored() {
    let q = "=bar&foo=baz";
    let val = parse(q).unwrap();
    assert!(val.get("").is_none());
    assert_eq!(val.get("foo").and_then(QsValue::as_scalar), Some("baz"));
}
