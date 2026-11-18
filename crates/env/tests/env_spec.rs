use std::{
    fs::{self, Permissions},
    time::Duration,
};

use env::{Env, EnvEditor, EnvError, EnvLoader, EnvParser, FieldValidator, Rule, Secret};

#[test]
fn test_parser_syntax_edge_cases() {
    let input = r#"
# Comment at the top
EMPTY=
SPACES=   spaced string   
SINGLE_QUOTES='hello $NOT_EXPANDED \n world'
DOUBLE_QUOTES="hello \n \t \"quoted\" \$NOT_EXPANDED world"
export EXPORTED=true
MULTILINE="line 1
line 2
line 3"
SINGLE_MULTILINE='part 1
part 2'
INLINE_COMMENT=foo # this is a comment
"#;

    let map = EnvParser::parse_to_map(input).unwrap();
    assert_eq!(map.get("EMPTY").unwrap(), "");
    assert_eq!(map.get("SPACES").unwrap(), "spaced string");
    assert_eq!(
        map.get("SINGLE_QUOTES").unwrap(),
        "hello $NOT_EXPANDED \\n world"
    );
    assert_eq!(
        map.get("DOUBLE_QUOTES").unwrap(),
        "hello \n \t \"quoted\" \\$NOT_EXPANDED world"
    );
    assert_eq!(map.get("EXPORTED").unwrap(), "true");
    assert_eq!(map.get("MULTILINE").unwrap(), "line 1\nline 2\nline 3");
    assert_eq!(map.get("SINGLE_MULTILINE").unwrap(), "part 1\npart 2");
    assert_eq!(map.get("INLINE_COMMENT").unwrap(), "foo");
}

#[test]
fn test_interpolation_basics_and_fallbacks() {
    let input = r#"
HOST=127.0.0.1
PORT=8080
BASE_URL=https://${HOST}:$PORT/api
FALLBACK_EXISTING=${HOST:-localhost}
FALLBACK_MISSING=${UNDEFINED_VAR:-default_val}
NESTED_FALLBACK=${UNDEFINED_VAR:-$HOST}
ESCAPED=abc\$def
"#;

    let map = EnvLoader::new().load_from_str(input).unwrap();
    assert_eq!(map.get("BASE_URL").unwrap(), "https://127.0.0.1:8080/api");
    assert_eq!(map.get("FALLBACK_EXISTING").unwrap(), "127.0.0.1");
    assert_eq!(map.get("FALLBACK_MISSING").unwrap(), "default_val");
    assert_eq!(map.get("NESTED_FALLBACK").unwrap(), "127.0.0.1");
    assert_eq!(map.get("ESCAPED").unwrap(), "abc$def");
}

#[test]
fn test_cyclic_dependency_detection() {
    let input = r#"
VAR_A=${VAR_B}
VAR_B=${VAR_C}
VAR_C=${VAR_A}
"#;

    let result = EnvLoader::new().load_from_str(input);
    match result {
        Err(EnvError::CyclicDependency { cycle }) => {
            assert!(cycle.contains(&"VAR_A".to_string()));
            assert!(cycle.contains(&"VAR_B".to_string()));
            assert!(cycle.contains(&"VAR_C".to_string()));
        }
        other => panic!("expected cyclic dependency error, got {other:?}"),
    }
}

#[test]
fn test_file_resolver() {
    let temp_dir = std::env::temp_dir().join("druto_test_secret");
    fs::create_dir_all(&temp_dir).unwrap();
    let secret_file = temp_dir.join("db_password.txt");
    fs::write(&secret_file, "super_secret_p@ssword\n").unwrap();

    let input = format!("DB_PASSWORD=file:{}", secret_file.display());
    let map = EnvLoader::new().load_from_str(&input).unwrap();
    assert_eq!(map.get("DB_PASSWORD").unwrap(), "super_secret_p@ssword");

    fs::remove_file(secret_file).unwrap();
    fs::remove_dir(temp_dir).unwrap();
}

#[test]
fn test_schema_validation_success_and_types() {
    let input = r#"
PORT=3000
HOST=127.0.0.1
IS_DEBUG=true
TIMEOUT=15s
MAX_BODY=10MB
APP_ENV=production
API_URL=https://api.druto.dev
NUM_WORKERS=8
PERCENTAGE=99.5
APP_KEY=12345678901234567890123456789012
TAGS=web,async,rust
"#;

    let env = Env::rules()
        .rule("PORT", Rule::port())
        .rule("HOST", Rule::string().not_empty())
        .rule("IS_DEBUG", Rule::boolean())
        .rule("TIMEOUT", Rule::duration())
        .rule("MAX_BODY", Rule::bytes())
        .rule(
            "APP_ENV",
            Rule::enum_values(&["development", "production", "test"]),
        )
        .rule("API_URL", Rule::url())
        .rule("NUM_WORKERS", Rule::<usize>::integer().min(1).max(32))
        .rule("PERCENTAGE", Rule::<f64>::float().range(0.0, 100.0))
        .rule("APP_KEY", Rule::secret(Rule::string().min_length(32)))
        .rule("TAGS", Rule::<Vec<String>>::list())
        .rule("OPTIONAL_VAR", Rule::<String>::string().optional())
        .rule("DEFAULTED_VAR", Rule::<u16>::port().default_value(8080))
        .parse_and_validate(input)
        .unwrap();

    assert_eq!(*env.get::<u16>("PORT").unwrap(), 3000);
    assert_eq!(env.get::<String>("HOST").unwrap(), "127.0.0.1");
    assert!(*env.get::<bool>("IS_DEBUG").unwrap());
    assert_eq!(
        *env.get::<Duration>("TIMEOUT").unwrap(),
        Duration::from_secs(15)
    );
    assert_eq!(*env.get::<u64>("MAX_BODY").unwrap(), 10 * 1024 * 1024);
    assert_eq!(env.get::<String>("APP_ENV").unwrap(), "production");
    assert_eq!(
        env.get::<String>("API_URL").unwrap(),
        "https://api.druto.dev"
    );
    assert_eq!(*env.get::<usize>("NUM_WORKERS").unwrap(), 8);
    assert_eq!(*env.get::<f64>("PERCENTAGE").unwrap(), 99.5);
    assert_eq!(*env.get::<u16>("DEFAULTED_VAR").unwrap(), 8080);
    assert_eq!(env.get::<Option<String>>("OPTIONAL_VAR").unwrap(), &None);

    let secret = env.get::<Secret<String>>("APP_KEY").unwrap();
    assert_eq!(format!("{secret:?}"), "[REDACTED]");
    assert_eq!(format!("{secret}"), "[REDACTED]");
    assert_eq!(secret.expose_secret(), "12345678901234567890123456789012");

    let tags = env.get::<Vec<String>>("TAGS").unwrap();
    assert_eq!(tags, &["web", "async", "rust"]);
}

#[test]
fn test_validation_error_aggregation_and_redaction() {
    let input = r#"
PORT=invalid_port
NUM_WORKERS=100
SECRET_KEY=short
APP_ENV=invalid_env
"#;

    let result = Env::rules()
        .rule("PORT", Rule::port())
        .rule("NUM_WORKERS", Rule::<usize>::integer().max(16))
        .rule("SECRET_KEY", Rule::secret(Rule::string().min_length(32)))
        .rule(
            "APP_ENV",
            Rule::enum_values(&["development", "production", "test"]),
        )
        .rule("MISSING_VAR", Rule::string())
        .parse_and_validate(input);

    match result {
        Err(EnvError::ValidationFailed { errors }) => {
            assert_eq!(errors.len(), 5);
            let display_report = format!("{}", EnvError::ValidationFailed { errors });

            assert!(display_report.contains("PORT"));
            assert!(display_report.contains("NUM_WORKERS"));
            assert!(display_report.contains("SECRET_KEY"));
            assert!(display_report.contains("APP_ENV"));
            assert!(display_report.contains("MISSING_VAR"));

            // Must NEVER leak the secret value "short" or invalid values in cleartext
            assert!(!display_report.contains("short"));
        }
        other => panic!("expected ValidationFailed with aggregated errors, got {other:?}"),
    }
}

#[test]
fn test_cascading_file_priority() {
    let temp_dir = std::env::temp_dir().join("druto_test_cascading");
    drop(fs::remove_dir_all(&temp_dir));
    fs::create_dir_all(&temp_dir).unwrap();

    fs::write(temp_dir.join(".env"), "FOO=base\nBAR=base\nBAZ=base\n").unwrap();
    fs::write(temp_dir.join(".env.production"), "BAR=prod\nBAZ=prod\n").unwrap();
    fs::write(temp_dir.join(".env.local"), "BAZ=local\n").unwrap();
    fs::write(temp_dir.join(".env.production.local"), "BAZ=prod_local\n").unwrap();

    let map = EnvLoader::new()
        .with_base_dir(&temp_dir)
        .with_env("production")
        .load()
        .unwrap();

    assert_eq!(map.get("FOO").unwrap(), "base");
    assert_eq!(map.get("BAR").unwrap(), "prod");
    assert_eq!(map.get("BAZ").unwrap(), "prod_local");

    fs::remove_dir_all(temp_dir).unwrap();
}

#[test]
fn test_editor_modifications_and_example_sync() {
    let temp_dir = std::env::temp_dir().join("druto_test_editor");
    drop(fs::remove_dir_all(&temp_dir));
    fs::create_dir_all(&temp_dir).unwrap();

    let env_file = temp_dir.join(".env");
    let example_file = temp_dir.join(".env.example");

    let initial = "# My Database Configuration\nDB_HOST=127.0.0.1 # inline comment\nDB_PORT=5432\n";
    fs::write(&env_file, initial).unwrap();

    let mut editor = EnvEditor::load(&env_file).unwrap();
    assert_eq!(editor.get("DB_HOST").unwrap(), "127.0.0.1");

    editor.set("DB_PORT", "5433");
    editor.set("APP_KEY", "secret key with spaces");
    editor.save().unwrap();

    editor.sync_example(&example_file, "APP_KEY").unwrap();
    editor.sync_example(&example_file, "DB_HOST").unwrap();

    let reloaded = fs::read_to_string(&env_file).unwrap();
    assert!(reloaded.contains("# My Database Configuration"));
    assert!(reloaded.contains("DB_PORT=5433"));
    assert!(reloaded.contains("# inline comment"));
    assert!(reloaded.contains("APP_KEY=\"secret key with spaces\""));

    let example_contents = fs::read_to_string(&example_file).unwrap();
    assert!(example_contents.contains("APP_KEY="));
    assert!(example_contents.contains("DB_HOST="));

    fs::remove_dir_all(temp_dir).unwrap();
}

#[cfg(feature = "serde")]
#[test]
fn test_serde_struct_deserialization() {
    #[derive(Debug, serde::Deserialize, PartialEq)]
    struct AppConfig {
        host: String,
        port: u16,
        is_debug: bool,
        tags: Vec<String>,
        optional_note: Option<String>,
    }

    let input = r#"
host=localhost
port=9000
is_debug=true
tags=alpha,beta,gamma
"#;

    let env = EnvLoader::new().load_from_str(input).unwrap();
    let env_container = Env::new(env, std::collections::HashMap::new());

    let config: AppConfig = env_container.deserialize().unwrap();
    assert_eq!(
        config,
        AppConfig {
            host: "localhost".to_string(),
            port: 9000,
            is_debug: true,
            tags: vec!["alpha".to_string(), "beta".to_string(), "gamma".to_string()],
            optional_note: None,
        }
    );
}

#[cfg(feature = "regex")]
#[test]
fn test_regex_rule() {
    let input = "SLUG=my-cool-post-123\nINVALID_SLUG=My Cool Post!";

    let env = Env::rules()
        .rule("SLUG", Rule::string().regex("^[a-z0-9-]+$"))
        .parse_and_validate(input)
        .unwrap();
    assert_eq!(env.get::<String>("SLUG").unwrap(), "my-cool-post-123");

    let failure = Env::rules()
        .rule("INVALID_SLUG", Rule::string().regex("^[a-z0-9-]+$"))
        .parse_and_validate(input);

    assert!(failure.is_err());
}

#[test]
fn test_boolean_and_port_variants() {
    let input = r#"
BOOL_TRUE=true
BOOL_ONE=1
BOOL_YES=yes
BOOL_ON=on
BOOL_FALSE=false
BOOL_ZERO=0
BOOL_NO=no
BOOL_OFF=off
PORT_MIN=1
PORT_MAX=65535
"#;

    let env = Env::rules()
        .rule("BOOL_TRUE", Rule::boolean())
        .rule("BOOL_ONE", Rule::boolean())
        .rule("BOOL_YES", Rule::boolean())
        .rule("BOOL_ON", Rule::boolean())
        .rule("BOOL_FALSE", Rule::boolean())
        .rule("BOOL_ZERO", Rule::boolean())
        .rule("BOOL_NO", Rule::boolean())
        .rule("BOOL_OFF", Rule::boolean())
        .rule("PORT_MIN", Rule::port())
        .rule("PORT_MAX", Rule::port())
        .parse_and_validate(input)
        .unwrap();

    assert!(*env.get::<bool>("BOOL_TRUE").unwrap());
    assert!(*env.get::<bool>("BOOL_ONE").unwrap());
    assert!(*env.get::<bool>("BOOL_YES").unwrap());
    assert!(*env.get::<bool>("BOOL_ON").unwrap());
    assert!(!*env.get::<bool>("BOOL_FALSE").unwrap());
    assert!(!*env.get::<bool>("BOOL_ZERO").unwrap());
    assert!(!*env.get::<bool>("BOOL_NO").unwrap());
    assert!(!*env.get::<bool>("BOOL_OFF").unwrap());
    assert_eq!(*env.get::<u16>("PORT_MIN").unwrap(), 1);
    assert_eq!(*env.get::<u16>("PORT_MAX").unwrap(), 65535);
}

#[test]
fn test_export_to_process() {
    let input = "TEST_DRUTO_EXPORT_KEY=exported_secret_value";
    let env = Env::rules()
        .rule("TEST_DRUTO_EXPORT_KEY", Rule::string())
        .parse_and_validate(input)
        .unwrap();

    // Safety: single-threaded test execution
    unsafe {
        env.export_to_process();
    }

    assert_eq!(
        std::env::var("TEST_DRUTO_EXPORT_KEY").unwrap(),
        "exported_secret_value"
    );
    unsafe {
        std::env::remove_var("TEST_DRUTO_EXPORT_KEY");
    }
}

#[test]
fn test_rules_string_and_numeric_bounds() {
    let input = r#"
PREFIXED=druto_app
SUFFIXED=config_file.json
TOO_LONG=abcdef
NUM_LOW=5
NUM_HIGH=50
FLOAT_OUT=105.2
INVALID_URL_SCHEME=httpexample.com
INVALID_URL_HOST=https://
LIST_INVALID=1,two,3
DURATION_INVALID=500lightyears
BYTES_INVALID=100zenabytes
"#;

    let res = Env::rules()
        .rule("PREFIXED", Rule::string().starts_with("druto_"))
        .rule("SUFFIXED", Rule::string().ends_with(".json"))
        .rule("TOO_LONG", Rule::string().max_length(3))
        .rule("NUM_LOW", Rule::<i32>::integer().min(10))
        .rule("NUM_HIGH", Rule::<i32>::integer().max(20))
        .rule("FLOAT_OUT", Rule::<f64>::float().range(0.0, 100.0))
        .rule("INVALID_URL_SCHEME", Rule::url())
        .rule("INVALID_URL_HOST", Rule::url())
        .rule("LIST_INVALID", Rule::<Vec<i32>>::list())
        .rule("DURATION_INVALID", Rule::duration())
        .rule("BYTES_INVALID", Rule::bytes())
        .parse_and_validate(input);

    match res {
        Err(EnvError::ValidationFailed { errors }) => {
            assert_eq!(errors.len(), 9);
        }
        other => panic!("expected validation errors, got {other:?}"),
    }
}

#[test]
fn test_duration_and_bytes_parsing_units() {
    let input = r#"
D_MS=250ms
D_S=30s
D_M=5m
D_H=2h
D_D=1d
B_B=100b
B_KB=2kb
B_MB=4mb
B_GB=1gb
B_TB=1tb
"#;

    let env = Env::rules()
        .rule("D_MS", Rule::duration())
        .rule("D_S", Rule::duration())
        .rule("D_M", Rule::duration())
        .rule("D_H", Rule::duration())
        .rule("D_D", Rule::duration())
        .rule("B_B", Rule::bytes())
        .rule("B_KB", Rule::bytes())
        .rule("B_MB", Rule::bytes())
        .rule("B_GB", Rule::bytes())
        .rule("B_TB", Rule::bytes())
        .parse_and_validate(input)
        .unwrap();

    assert_eq!(
        *env.get::<Duration>("D_MS").unwrap(),
        Duration::from_millis(250)
    );
    assert_eq!(
        *env.get::<Duration>("D_S").unwrap(),
        Duration::from_secs(30)
    );
    assert_eq!(
        *env.get::<Duration>("D_M").unwrap(),
        Duration::from_secs(300)
    );
    assert_eq!(
        *env.get::<Duration>("D_H").unwrap(),
        Duration::from_secs(7200)
    );
    assert_eq!(
        *env.get::<Duration>("D_D").unwrap(),
        Duration::from_secs(86400)
    );

    assert_eq!(*env.get::<u64>("B_B").unwrap(), 100);
    assert_eq!(*env.get::<u64>("B_KB").unwrap(), 2 * 1024);
    assert_eq!(*env.get::<u64>("B_MB").unwrap(), 4 * 1024 * 1024);
    assert_eq!(*env.get::<u64>("B_GB").unwrap(), 1024 * 1024 * 1024);
    assert_eq!(*env.get::<u64>("B_TB").unwrap(), 1024 * 1024 * 1024 * 1024);
}

#[test]
fn test_secret_conversions_and_display() {
    let secret = Secret::from("my_password".to_string());
    assert_eq!(format!("{secret:?}"), "[REDACTED]");
    assert_eq!(format!("{secret}"), "[REDACTED]");
    assert_eq!(secret.expose_secret(), "my_password");
    assert_eq!(secret.into_inner(), "my_password");

    #[cfg(feature = "serde")]
    {
        use serde::{
            Deserialize,
            de::{IntoDeserializer, value::Error as DeError},
        };
        let deserializer: serde::de::value::StrDeserializer<'_, DeError> =
            "pass123".into_deserializer();
        let deserialized = Secret::<String>::deserialize(deserializer).unwrap();
        assert_eq!(deserialized.expose_secret(), "pass123");
    }
}

#[test]
fn test_env_container_queries_and_errors() {
    let input = "KEY_ONE=val1\nKEY_INT=42";
    let env = Env::rules()
        .rule("KEY_ONE", Rule::string())
        .rule("KEY_INT", Rule::<i32>::integer())
        .parse_and_validate(input)
        .unwrap();

    assert!(env.contains("KEY_ONE"));
    assert_eq!(env.get_raw("KEY_ONE"), Some("val1"));
    assert_eq!(env.get_raw("NON_EXISTENT"), None);
    assert!(!env.raw_map().is_empty());

    let mismatch = env.get::<bool>("KEY_ONE");
    assert!(matches!(mismatch, Err(EnvError::TypeMismatch { .. })));

    let not_found = env.get::<String>("MISSING");
    assert!(matches!(not_found, Err(EnvError::NotFound { .. })));

    let loader = Env::loader();
    drop(loader);
}

#[test]
fn test_error_display_formatting() {
    let io_err = EnvError::Io(std::io::Error::new(
        std::io::ErrorKind::NotFound,
        "file missing",
    ));
    assert!(format!("{io_err}").contains("I/O error"));

    let parse_err = EnvError::Parse {
        line: 12,
        message: "bad syntax".into(),
    };
    assert!(format!("{parse_err}").contains("parse error at line 12"));

    let resolver_err = EnvError::Resolver {
        key: "SECRET".into(),
        message: "read fail".into(),
    };
    assert!(format!("{resolver_err}").contains("resolver failed for `SECRET`"));

    let not_found_err = EnvError::NotFound {
        key: "APP_KEY".into(),
    };
    assert!(format!("{not_found_err}").contains("`APP_KEY` not found"));

    let type_err = EnvError::TypeMismatch {
        key: "PORT".into(),
        expected: "u16",
    };
    assert!(format!("{type_err}").contains("expected u16"));
}

#[test]
fn test_loader_with_explicit_file_and_override() {
    let temp_dir = std::env::temp_dir().join("druto_test_explicit");
    drop(fs::remove_dir_all(&temp_dir));
    fs::create_dir_all(&temp_dir).unwrap();

    let custom_file = temp_dir.join("custom.env");
    fs::write(&custom_file, "CUSTOM_VAR=loaded_explicitly\n").unwrap();

    let map = EnvLoader::new()
        .with_file(&custom_file)
        .override_system(true)
        .load()
        .unwrap();

    assert_eq!(map.get("CUSTOM_VAR").unwrap(), "loaded_explicitly");

    fs::remove_dir_all(temp_dir).unwrap();
}

#[test]
fn test_editor_unset_and_render() {
    let input = "# Header\nKEY1=val1\nKEY2=val2\n";
    let mut editor = EnvEditor::parse(input);
    assert_eq!(editor.get("KEY1"), Some("val1"));
    editor.unset("KEY1");
    assert_eq!(editor.get("KEY1"), None);
    let rendered = editor.render();
    assert!(!rendered.contains("KEY1"));
    assert!(rendered.contains("KEY2=val2"));
}

#[cfg(feature = "serde")]
#[test]
fn test_serde_all_numeric_types() {
    #[derive(Debug, serde::Deserialize, PartialEq)]
    struct NumericTypes {
        v_i8: i8,
        v_i16: i16,
        v_i32: i32,
        v_i64: i64,
        v_u8: u8,
        v_u16: u16,
        v_u32: u32,
        v_u64: u64,
        v_f32: f32,
        v_f64: f64,
    }

    let input = r#"
v_i8=-8
v_i16=-16
v_i32=-32
v_i64=-64
v_u8=8
v_u16=16
v_u32=32
v_u64=64
v_f32=1.25
v_f64=4.5
"#;

    let env_map = EnvLoader::new().load_from_str(input).unwrap();
    let env = Env::new(env_map, std::collections::HashMap::new());
    let numbers: NumericTypes = env.deserialize().unwrap();

    assert_eq!(numbers.v_i8, -8);
    assert_eq!(numbers.v_i16, -16);
    assert_eq!(numbers.v_i32, -32);
    assert_eq!(numbers.v_i64, -64);
    assert_eq!(numbers.v_u8, 8);
    assert_eq!(numbers.v_u16, 16);
    assert_eq!(numbers.v_u32, 32);
    assert_eq!(numbers.v_u64, 64);
    assert!((numbers.v_f32 - 1.25).abs() < f32::EPSILON);
    assert!((numbers.v_f64 - 4.5).abs() < f64::EPSILON);
}

#[test]
fn test_parser_syntax_errors_and_escapes() {
    let no_equals = EnvParser::parse("INVALID_LINE_NO_EQUALS");
    assert!(matches!(no_equals, Err(EnvError::Parse { .. })));

    let empty_key = EnvParser::parse("=value_without_key");
    assert!(matches!(empty_key, Err(EnvError::Parse { .. })));

    let unclosed_single = EnvParser::parse("KEY='unclosed single quote");
    assert!(matches!(unclosed_single, Err(EnvError::Parse { .. })));

    let unclosed_double = EnvParser::parse("KEY=\"unclosed double quote");
    assert!(matches!(unclosed_double, Err(EnvError::Parse { .. })));

    let escapes = r#"
ESC_TEST="line\rwith\tescapes\\and\"quotes\"\$and_dollar\x_other"
"#;
    let parsed = EnvParser::parse_to_map(escapes).unwrap();
    let val = parsed.get("ESC_TEST").unwrap();
    assert!(val.contains('\r'));
    assert!(val.contains('\t'));
    assert!(val.contains('\\'));
    assert!(val.contains('"'));
    assert!(val.contains("$and_dollar"));
    assert!(val.contains("\\x"));
}

#[test]
fn test_rules_missing_values_and_error_branches() {
    let empty = std::collections::HashMap::new();

    let rule_int = Rule::<i32>::integer();
    assert!(rule_int.validate("KEY", &empty).is_err());

    let rule_float = Rule::<f64>::float();
    assert!(rule_float.validate("KEY", &empty).is_err());

    let rule_bool = Rule::boolean();
    assert!(rule_bool.validate("KEY", &empty).is_err());

    let rule_port = Rule::port();
    assert!(rule_port.validate("KEY", &empty).is_err());

    let rule_duration = Rule::duration();
    assert!(rule_duration.validate("KEY", &empty).is_err());

    let rule_bytes = Rule::bytes();
    assert!(rule_bytes.validate("KEY", &empty).is_err());

    let rule_list = Rule::<Vec<String>>::list();
    assert!(rule_list.validate("KEY", &empty).is_err());

    let rule_url = Rule::url();
    assert!(rule_url.validate("KEY", &empty).is_err());

    let rule_enum = Rule::enum_values(&["dev", "prod"]);
    assert!(rule_enum.validate("KEY", &empty).is_err());

    let input_nums = "INT_MIN=5\nINT_MAX=25\nFLOAT_MIN=2.5\nFLOAT_MAX=15.0\nINT_RANGE=15";
    let env_nums = Env::rules()
        .rule("INT_MIN", Rule::<i32>::integer().min(1))
        .rule("INT_MAX", Rule::<i32>::integer().max(30))
        .rule("FLOAT_MIN", Rule::<f64>::float().min(1.0))
        .rule("FLOAT_MAX", Rule::<f64>::float().max(20.0))
        .rule("INT_RANGE", Rule::<i32>::integer().range(10, 20))
        .parse_and_validate(input_nums)
        .unwrap();

    assert_eq!(*env_nums.get::<i32>("INT_MIN").unwrap(), 5);
    assert_eq!(*env_nums.get::<i32>("INT_MAX").unwrap(), 25);
    assert_eq!(*env_nums.get::<f64>("FLOAT_MIN").unwrap(), 2.5);
    assert_eq!(*env_nums.get::<f64>("FLOAT_MAX").unwrap(), 15.0);
    assert_eq!(*env_nums.get::<i32>("INT_RANGE").unwrap(), 15);

    let failed_bounds = Env::rules()
        .rule("FLOAT_MIN", Rule::<f64>::float().min(10.0))
        .rule("FLOAT_MAX", Rule::<f64>::float().max(5.0))
        .rule("INT_RANGE", Rule::<i32>::integer().range(20, 30))
        .rule("STR_STARTS", Rule::string().starts_with("api_"))
        .rule("STR_ENDS", Rule::string().ends_with(".tar.gz"))
        .rule("STR_EMPTY", Rule::string().not_empty())
        .parse_and_validate("FLOAT_MIN=5.0\nFLOAT_MAX=10.0\nINT_RANGE=5\nSTR_STARTS=web_\nSTR_ENDS=file.zip\nSTR_EMPTY=");

    assert!(failed_bounds.is_err());
}

#[test]
fn test_duration_and_bytes_parsing_error_branches() {
    let invalid_inputs = [
        ("D_EMPTY", ""),
        ("D_NONUM", "seconds"),
        ("D_NEG", "-10s"),
        ("D_BADUNIT", "10unknowns"),
        ("D_DOTONLY", ".s"),
        ("B_EMPTY", ""),
        ("B_NONUM", "bytes"),
        ("B_NEG", "-50kb"),
        ("B_BADUNIT", "50unknownbytes"),
        ("B_DOTONLY", ".kb"),
    ];

    for (key, val) in invalid_inputs {
        let mut map = std::collections::HashMap::new();
        map.insert(key.to_string(), val.to_string());

        if key.starts_with('D') {
            assert!(Rule::duration().validate(key, &map).is_err());
        } else {
            assert!(Rule::bytes().validate(key, &map).is_err());
        }
    }
}

#[test]
fn test_editor_extended_paths() {
    let input = "# Comment line\nexport EXPORTED_KEY='single value' # single comment\nRAW_LINE_NO_EQUALS\n\nEMPTY_KEY=\n";
    let editor = EnvEditor::parse(input);

    assert_eq!(editor.get("EXPORTED_KEY"), Some("single value"));
    assert_eq!(editor.get("EMPTY_KEY"), Some(""));

    let empty_line = env::EditorLine::Empty;
    assert_eq!(empty_line.render(), "");

    let raw_line = env::EditorLine::Raw("RAW_ENTRY".to_string());
    assert_eq!(raw_line.render(), "RAW_ENTRY");

    let no_path_save = editor.save();
    assert!(no_path_save.is_err());

    let non_existent_load = EnvEditor::load("/non/existent/path/.env");
    assert!(non_existent_load.is_err());

    let temp_dir = std::env::temp_dir().join("druto_test_editor_sync");
    drop(fs::remove_dir_all(&temp_dir));
    fs::create_dir_all(&temp_dir).unwrap();

    let example_path = temp_dir.join(".env.example");
    fs::write(&example_path, "EXISTING_KEY=\n").unwrap();
    editor.sync_example(&example_path, "EXISTING_KEY").unwrap();
    let contents = fs::read_to_string(&example_path).unwrap();
    assert_eq!(contents, "EXISTING_KEY=\n");

    fs::remove_dir_all(temp_dir).unwrap();
}

#[test]
fn test_loader_environment_resolution_and_defaults() {
    let default_loader = EnvLoader::default();
    assert_eq!(default_loader.resolve_active_env(), "development");

    let test_loader = EnvLoader::new().with_env("test");
    let paths = test_loader.get_cascaded_paths();
    assert!(!paths.iter().any(|p| p.ends_with(".env.local")));

    unsafe {
        std::env::set_var("DRUTO_ENV", "staging");
    }
    assert_eq!(EnvLoader::new().resolve_active_env(), "staging");
    unsafe {
        std::env::remove_var("DRUTO_ENV");
    }

    unsafe {
        std::env::set_var("APP_ENV", "testing_env");
    }
    assert_eq!(EnvLoader::new().resolve_active_env(), "testing_env");
    unsafe {
        std::env::remove_var("APP_ENV");
    }

    let temp_dir = std::env::temp_dir().join("druto_test_load_validate");
    drop(fs::remove_dir_all(&temp_dir));
    fs::create_dir_all(&temp_dir).unwrap();
    fs::write(temp_dir.join(".env"), "PORT=5000\n").unwrap();

    let loader = EnvLoader::new().with_base_dir(&temp_dir);
    let validated_env = Env::rules()
        .rule("PORT", Rule::port())
        .load_and_validate(&loader)
        .unwrap();

    assert_eq!(*validated_env.get::<u16>("PORT").unwrap(), 5000);
    fs::remove_dir_all(temp_dir).unwrap();
}

#[test]
fn test_interpolator_missing_file_error() {
    let input = "SECRET_KEY=file:/tmp/definitely_missing_file_druto_test_12345.txt";
    let res = EnvLoader::new().load_from_str(input);
    assert!(matches!(res, Err(EnvError::Resolver { .. })));
}

#[test]
fn test_error_source_and_display_coverage() {
    use std::error::Error;

    let io_raw = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "access denied");
    let io_err: EnvError = io_raw.into();
    assert!(io_err.source().is_some());

    let not_found_err = EnvError::NotFound {
        key: "X".to_string(),
    };
    assert!(not_found_err.source().is_none());

    let cyclic_err = EnvError::CyclicDependency {
        cycle: vec!["A".to_string(), "B".to_string(), "A".to_string()],
    };
    assert!(format!("{cyclic_err}").contains("cyclic dependency"));

    let val_err = env::ValidationError::new("KEY", "msg");
    assert_eq!(format!("{val_err}"), "KEY: msg");

    #[cfg(feature = "serde")]
    {
        let deser_err = EnvError::Deserialize("deserialization failed".to_string());
        assert!(format!("{deser_err}").contains("deserialization error: deserialization failed"));
    }
}

#[test]
fn test_loader_path_branches() {
    let loader_rel = EnvLoader::new().with_file("rel.env");
    let paths_rel = loader_rel.get_cascaded_paths();
    assert_eq!(paths_rel.len(), 1);

    let loader_abs = EnvLoader::new().with_file("/abs/path/file.env");
    let paths_abs = loader_abs.get_cascaded_paths();
    assert_eq!(paths_abs.len(), 1);

    let loader_testing = EnvLoader::new().with_env("testing");
    let paths_testing = loader_testing.get_cascaded_paths();
    assert!(!paths_testing.iter().any(|p| p.ends_with(".env.local")));

    let loader_empty = EnvLoader::new().with_env("");
    let paths_empty = loader_empty.get_cascaded_paths();
    assert_eq!(paths_empty.len(), 2);

    let temp_dir = std::env::temp_dir().join("druto_test_abs_load");
    drop(fs::remove_dir_all(&temp_dir));
    fs::create_dir_all(&temp_dir).unwrap();

    let abs_file = temp_dir.join("abs_file.env");
    fs::write(&abs_file, "LOADED_ABS=1\n").unwrap();
    let res = EnvLoader::new().with_file(&abs_file).load().unwrap();
    assert_eq!(res.get("LOADED_ABS").unwrap(), "1");

    fs::remove_dir_all(temp_dir).unwrap();
}

#[cfg(feature = "serde")]
#[test]
fn test_secret_serialize_coverage() {
    use serde::Serialize;

    struct TestSerializer;
    impl serde::Serializer for TestSerializer {
        type Ok = &'static str;
        type Error = serde::de::value::Error;
        type SerializeSeq = serde::ser::Impossible<&'static str, Self::Error>;
        type SerializeTuple = serde::ser::Impossible<&'static str, Self::Error>;
        type SerializeTupleStruct = serde::ser::Impossible<&'static str, Self::Error>;
        type SerializeTupleVariant = serde::ser::Impossible<&'static str, Self::Error>;
        type SerializeMap = serde::ser::Impossible<&'static str, Self::Error>;
        type SerializeStruct = serde::ser::Impossible<&'static str, Self::Error>;
        type SerializeStructVariant = serde::ser::Impossible<&'static str, Self::Error>;

        fn serialize_bool(self, _: bool) -> Result<Self::Ok, Self::Error> {
            unimplemented!()
        }

        fn serialize_i8(self, _: i8) -> Result<Self::Ok, Self::Error> {
            unimplemented!()
        }
        fn serialize_i16(self, _: i16) -> Result<Self::Ok, Self::Error> {
            unimplemented!()
        }
        fn serialize_i32(self, _: i32) -> Result<Self::Ok, Self::Error> {
            unimplemented!()
        }
        fn serialize_i64(self, _: i64) -> Result<Self::Ok, Self::Error> {
            unimplemented!()
        }
        fn serialize_u8(self, _: u8) -> Result<Self::Ok, Self::Error> {
            unimplemented!()
        }
        fn serialize_u16(self, _: u16) -> Result<Self::Ok, Self::Error> {
            unimplemented!()
        }
        fn serialize_u32(self, _: u32) -> Result<Self::Ok, Self::Error> {
            unimplemented!()
        }
        fn serialize_u64(self, _: u64) -> Result<Self::Ok, Self::Error> {
            unimplemented!()
        }
        fn serialize_f32(self, _: f32) -> Result<Self::Ok, Self::Error> {
            unimplemented!()
        }
        fn serialize_f64(self, _: f64) -> Result<Self::Ok, Self::Error> {
            unimplemented!()
        }
        fn serialize_char(self, _: char) -> Result<Self::Ok, Self::Error> {
            unimplemented!()
        }
        fn serialize_str(self, v: &str) -> Result<Self::Ok, Self::Error> {
            assert_eq!(v, "[REDACTED]");
            Ok("[REDACTED]")
        }
        fn serialize_bytes(self, _: &[u8]) -> Result<Self::Ok, Self::Error> {
            unimplemented!()
        }
        fn serialize_none(self) -> Result<Self::Ok, Self::Error> {
            unimplemented!()
        }
        fn serialize_some<T: ?Sized + Serialize>(self, _: &T) -> Result<Self::Ok, Self::Error> {
            unimplemented!()
        }
        fn serialize_unit(self) -> Result<Self::Ok, Self::Error> {
            unimplemented!()
        }
        fn serialize_unit_struct(self, _: &'static str) -> Result<Self::Ok, Self::Error> {
            unimplemented!()
        }
        fn serialize_unit_variant(
            self,
            _: &'static str,
            _: u32,
            _: &'static str,
        ) -> Result<Self::Ok, Self::Error> {
            unimplemented!()
        }
        fn serialize_newtype_struct<T: ?Sized + Serialize>(
            self,
            _: &'static str,
            _: &T,
        ) -> Result<Self::Ok, Self::Error> {
            unimplemented!()
        }
        fn serialize_newtype_variant<T: ?Sized + Serialize>(
            self,
            _: &'static str,
            _: u32,
            _: &'static str,
            _: &T,
        ) -> Result<Self::Ok, Self::Error> {
            unimplemented!()
        }
        fn serialize_seq(self, _: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
            unimplemented!()
        }
        fn serialize_tuple(self, _: usize) -> Result<Self::SerializeTuple, Self::Error> {
            unimplemented!()
        }
        fn serialize_tuple_struct(
            self,
            _: &'static str,
            _: usize,
        ) -> Result<Self::SerializeTupleStruct, Self::Error> {
            unimplemented!()
        }
        fn serialize_tuple_variant(
            self,
            _: &'static str,
            _: u32,
            _: &'static str,
            _: usize,
        ) -> Result<Self::SerializeTupleVariant, Self::Error> {
            unimplemented!()
        }
        fn serialize_map(self, _: Option<usize>) -> Result<Self::SerializeMap, Self::Error> {
            unimplemented!()
        }
        fn serialize_struct(
            self,
            _: &'static str,
            _: usize,
        ) -> Result<Self::SerializeStruct, Self::Error> {
            unimplemented!()
        }
        fn serialize_struct_variant(
            self,
            _: &'static str,
            _: u32,
            _: &'static str,
            _: usize,
        ) -> Result<Self::SerializeStructVariant, Self::Error> {
            unimplemented!()
        }
    }

    let secret = Secret::new("top_secret".to_string());
    let serialized = secret.serialize(TestSerializer).unwrap();
    assert_eq!(serialized, "[REDACTED]");
}

#[test]
fn test_env_raw_contains_and_deserialization_failures() {
    use std::collections::{BTreeMap, HashMap};

    let mut raw = HashMap::new();
    raw.insert("ONLY_RAW_KEY".to_string(), "val".to_string());
    let env_raw = Env::new(raw, HashMap::new());
    assert!(env_raw.contains("ONLY_RAW_KEY"));

    #[cfg(feature = "serde")]
    {
        #[derive(Debug, serde::Deserialize)]
        struct FailingBool {
            flag: bool,
        }
        let map = EnvLoader::new().load_from_str("flag=invalid_bool").unwrap();
        let env_fail = Env::new(map, HashMap::new());
        assert!(env_fail.deserialize::<FailingBool>().is_err());
        let dummy = FailingBool { flag: true };
        assert!(dummy.flag);

        let map_ok = EnvLoader::new().load_from_str("K1=V1\nK2=V2").unwrap();
        let env_ok = Env::new(map_ok, HashMap::new());
        let map_deser: BTreeMap<String, String> = env_ok.deserialize().unwrap();
        assert_eq!(map_deser.get("K1").unwrap(), "V1");
    }
}

#[test]
fn test_loader_explicit_file_error() {
    let bad_loader = EnvLoader::new().with_file("/definitely/non_existent_explicit_path/test.env");
    assert!(bad_loader.load().is_err());
}

#[test]
fn test_secret_derived_traits() {
    use std::collections::HashSet;

    let s1 = Secret::new("alpha".to_string());
    let s2 = Secret::new("beta".to_string());

    assert!(s1 < s2);
    assert_eq!(s1.clone(), s1);

    let mut set = HashSet::new();
    set.insert(s1.clone());
    assert!(set.contains(&s1));
}

#[cfg(feature = "serde")]
#[test]
fn test_serde_bool_variants_and_options() {
    #[derive(Debug, serde::Deserialize, PartialEq)]
    struct ExtraConfig {
        b1: bool,
        b2: bool,
        b3: bool,
        b4: bool,
        opt_empty: Option<String>,
        opt_some: Option<String>,
        ports: Vec<u16>,
    }

    let input = r#"
b1=0
b2=false
b3=no
b4=off
opt_empty=
opt_some=present
ports=80,443,8080
"#;

    let map = EnvLoader::new().load_from_str(input).unwrap();
    let env = Env::new(map, std::collections::HashMap::new());
    let cfg: ExtraConfig = env.deserialize().unwrap();

    assert_eq!(
        cfg,
        ExtraConfig {
            b1: false,
            b2: false,
            b3: false,
            b4: false,
            opt_empty: None,
            opt_some: Some("present".to_string()),
            ports: vec![80, 443, 8080],
        }
    );
}

#[test]
fn test_env_debug_formatting() {
    let mut map = std::collections::HashMap::new();
    map.insert("KEY".to_string(), "SECRET".to_string());
    let env = Env::new(map, std::collections::HashMap::new());
    let debug = format!("{env:?}");
    assert!(debug.contains("Env"));
    assert!(debug.contains("KEY"));
    assert!(!debug.contains("SECRET"));
}

#[test]
fn test_editor_full_paths() {
    let editor = EnvEditor::parse("export FOO=bar\nexporting=123\nUNQUOTED='unclosed");
    assert!(editor.render().contains("export FOO=bar"));
    assert_eq!(editor.get("exporting"), Some("123"));

    let from_str_editor = "A=B".parse::<EnvEditor>().unwrap();
    assert_eq!(from_str_editor.get("A"), Some("B"));

    let mut unquoted_editor = EnvEditor::parse("A=old");
    unquoted_editor.set("A", "new value with space");
    assert_eq!(unquoted_editor.get("A"), Some("new value with space"));
    assert!(
        unquoted_editor
            .render()
            .contains("A=\"new value with space\"")
    );
}

#[test]
fn test_interpolator_nested_and_relative_resolver() {
    let input = "VAR_X=${UNDEFINED:-${ALSO_UNDEFINED:-deep_fallback}}";
    let map = EnvLoader::new().load_from_str(input).unwrap();
    assert_eq!(map.get("VAR_X").unwrap(), "deep_fallback");

    let temp_dir = std::env::temp_dir().join("druto_test_rel_secret");
    drop(fs::remove_dir_all(&temp_dir));
    fs::create_dir_all(&temp_dir).unwrap();

    let secret_file = temp_dir.join("rel_secret.txt");
    fs::write(&secret_file, "relative_password\n").unwrap();

    let rel_input = "SECRET=file:rel_secret.txt";
    let loaded = EnvLoader::new()
        .with_base_dir(&temp_dir)
        .load_from_str(rel_input)
        .unwrap();

    assert_eq!(loaded.get("SECRET").unwrap(), "relative_password");
    fs::remove_dir_all(temp_dir).unwrap();
}

#[test]
fn test_loader_relative_explicit_and_unreadable() {
    let temp_dir = std::env::temp_dir().join("druto_test_rel_explicit");
    drop(fs::remove_dir_all(&temp_dir));
    fs::create_dir_all(&temp_dir).unwrap();

    let rel_env = temp_dir.join("rel.env");
    fs::write(&rel_env, "REL_KEY=rel_val\n").unwrap();

    let map = EnvLoader::new()
        .with_base_dir(&temp_dir)
        .with_file("rel.env")
        .load()
        .unwrap();

    assert_eq!(map.get("REL_KEY").unwrap(), "rel_val");

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let unreadable_dir = std::env::temp_dir().join("druto_test_unreadable");
        drop(fs::remove_dir_all(&unreadable_dir));
        fs::create_dir_all(&unreadable_dir).unwrap();

        let unreadable_file = unreadable_dir.join(".env");
        fs::write(&unreadable_file, "SECRET=1\n").unwrap();
        fs::set_permissions(&unreadable_file, Permissions::from_mode(0o000)).unwrap();

        let err = EnvLoader::new().with_base_dir(&unreadable_dir).load();
        assert!(err.is_err());

        fs::set_permissions(&unreadable_file, Permissions::from_mode(0o644)).unwrap();
        fs::remove_dir_all(unreadable_dir).unwrap();
    }

    fs::remove_dir_all(temp_dir).unwrap();
}

#[test]
fn test_loader_override_system_in_load_from_str() {
    unsafe {
        std::env::set_var("DRUTO_OVERRIDE_STR_TEST", "from_system");
    }

    let map = EnvLoader::new()
        .override_system(true)
        .load_from_str("DRUTO_OVERRIDE_STR_TEST=from_file")
        .unwrap();

    assert_eq!(map.get("DRUTO_OVERRIDE_STR_TEST").unwrap(), "from_file");

    unsafe {
        std::env::remove_var("DRUTO_OVERRIDE_STR_TEST");
    }
}

#[test]
fn test_parser_export_prefix_without_space() {
    let map = EnvParser::parse_to_map("export_custom=hello").unwrap();
    assert_eq!(map.get("export_custom").unwrap(), "hello");
}

#[test]
fn test_rules_branch_completions() {
    let mut map = std::collections::HashMap::new();
    map.insert("PORT_VALID".to_string(), "8080".to_string());
    map.insert("PORT_INVALID".to_string(), "abc".to_string());
    map.insert("STR_PRESENT".to_string(), "content".to_string());
    map.insert("STR_SHORT".to_string(), "ok".to_string());
    map.insert("BOOL_BAD".to_string(), "not_a_bool".to_string());
    map.insert("INT_BAD".to_string(), "not_an_int".to_string());
    map.insert("FLOAT_BAD".to_string(), "not_a_float".to_string());
    map.insert("D_NEG".to_string(), "-10s".to_string());
    map.insert("B_NEG".to_string(), "-100kb".to_string());

    let val_port = Rule::port().default_value(3000);
    assert_eq!(
        val_port
            .validate("PORT_VALID", &map)
            .unwrap()
            .downcast_ref::<u16>(),
        Some(&8080)
    );
    assert!(val_port.validate("PORT_INVALID", &map).is_err());

    let val_opt = Rule::string().optional();
    assert_eq!(
        val_opt
            .validate("STR_PRESENT", &map)
            .unwrap()
            .downcast_ref::<Option<String>>(),
        Some(&Some("content".to_string()))
    );

    let val_max = Rule::string().max_length(5);
    assert!(val_max.validate("STR_SHORT", &map).is_ok());

    let val_list = Rule::<Vec<String>>::list();
    let mut list_map = std::collections::HashMap::new();
    list_map.insert("LIST".to_string(), "item1,,item2".to_string());
    assert_eq!(
        val_list
            .validate("LIST", &list_map)
            .unwrap()
            .downcast_ref::<Vec<String>>(),
        Some(&vec!["item1".to_string(), "item2".to_string()])
    );

    assert!(Rule::boolean().validate("BOOL_BAD", &map).is_err());
    assert!(Rule::<i32>::integer().validate("INT_BAD", &map).is_err());
    assert!(Rule::<f64>::float().validate("FLOAT_BAD", &map).is_err());
    assert!(Rule::duration().validate("D_NEG", &map).is_err());
    assert!(Rule::bytes().validate("B_NEG", &map).is_err());
}

#[test]
fn test_overflow_and_multiline_recommendations() {
    let mut map = std::collections::HashMap::new();
    map.insert("D_HUGE".to_string(), "99999999999999999999999d".to_string());
    map.insert(
        "D_MS_HUGE".to_string(),
        "99999999999999999999999ms".to_string(),
    );
    map.insert(
        "B_HUGE".to_string(),
        "99999999999999999999999tb".to_string(),
    );

    assert!(Rule::duration().validate("D_HUGE", &map).is_err());
    assert!(Rule::duration().validate("D_MS_HUGE", &map).is_err());
    assert!(Rule::bytes().validate("B_HUGE", &map).is_err());

    let multiline_input = "FIRST=1\nMULTILINE=\"part 1\npart 2\npart 3\" # comment\nLAST=2";
    let editor = EnvEditor::parse(multiline_input);
    assert_eq!(editor.get("MULTILINE"), Some("part 1\npart 2\npart 3"));
    assert_eq!(editor.get("FIRST"), Some("1"));
    assert_eq!(editor.get("LAST"), Some("2"));

    let rendered = editor.render();
    assert!(rendered.contains("MULTILINE=\"part 1\npart 2\npart 3\" # comment"));
}
