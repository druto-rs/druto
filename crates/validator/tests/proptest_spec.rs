//! Property-based testing for validator invariants using proptest.

use proptest::prelude::*;
use serde_json::json;
use validator::prelude::*;

proptest! {
    #[test]
    fn prop_string_length_invariants(s in "\\PC{0,50}") {
        let char_count = s.chars().count();
        let schema = string().min_len(5).max_len(20);
        let value = json!(s);

        if (5..=20).contains(&char_count) {
            prop_assert!(schema.validate(&value).is_ok());
        } else if char_count < 5 {
            let err = schema.validate(&value).unwrap_err();
            prop_assert_eq!(err.issues()[0].rule.as_str(), "min_len");
        } else {
            let err = schema.validate(&value).unwrap_err();
            prop_assert_eq!(err.issues()[0].rule.as_str(), "max_len");
        }
    }

    #[test]
    fn prop_integer_range_invariants(n in -1000i64..1000i64) {
        let schema = integer().range(10, 100);
        let value = json!(n);

        if (10..=100).contains(&n) {
            prop_assert_eq!(schema.validate(&value).unwrap(), n);
        } else {
            prop_assert!(schema.validate(&value).is_err());
        }
    }

    #[test]
    fn prop_object_strip_unknown_fields(
        known in "[a-z]{3,10}",
        unknown_key in "[A-Z]{3,10}",
        unknown_val in 0i64..1000i64,
    ) {
        let schema = object().field("known", string());
        let payload = json!({
            "known": known,
            unknown_key.clone(): unknown_val,
        });

        let validated = schema.validate(&payload).unwrap();
        // Invariant: The known field is always present
        prop_assert_eq!(validated.get("known").and_then(|v| v.as_str()), Some(known.as_str()));
        // Invariant: Fail-closed strip policy guarantees unknown keys are never present
        prop_assert!(validated.get(&unknown_key).is_none());
    }
}
