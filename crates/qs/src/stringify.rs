//! Query string serializer for [`QsValue`].

use crate::{
    encoding::{encode_component, encode_key_preserving_brackets},
    options::{ArrayFormat, StringifyOptions},
    value::QsValue,
};

/// Serializes a [`QsValue`] tree into a query string according to the provided options.
#[must_use]
pub fn stringify_query(value: &QsValue, options: &StringifyOptions) -> String {
    let mut pairs = Vec::new();
    collect_pairs(value, "", options, &mut pairs);

    if pairs.is_empty() {
        return String::new();
    }

    let delimiter = options.get_delimiter() as char;
    let joined = pairs.join(&delimiter.to_string());

    if options.get_add_query_prefix() {
        format!("?{joined}")
    } else {
        joined
    }
}

fn collect_pairs(
    value: &QsValue,
    prefix: &str,
    options: &StringifyOptions,
    pairs: &mut Vec<String>,
) {
    match value {
        QsValue::Null => {
            if options.get_skip_nulls() || prefix.is_empty() {
                return;
            }
            let key = format_key(prefix, options);
            if options.get_strict_null_handling() {
                pairs.push(key);
            } else {
                pairs.push(format!("{key}="));
            }
        }
        QsValue::Scalar(text) => {
            if prefix.is_empty() {
                return;
            }
            let key = format_key(prefix, options);
            let val = format_value(text, options);
            pairs.push(format!("{key}={val}"));
        }
        QsValue::Array(items) => {
            if items.is_empty() {
                return;
            }

            if options.get_array_format() == ArrayFormat::Comma
                && items.iter().all(|item| matches!(item, QsValue::Scalar(_)))
            {
                let key = format_key(prefix, options);
                let joined_vals: Vec<String> = items
                    .iter()
                    .filter_map(|item| item.as_scalar())
                    .map(|text| format_value(text, options))
                    .collect();
                pairs.push(format!("{key}={}", joined_vals.join(",")));
                return;
            }

            for (idx, item) in items.iter().enumerate() {
                let child_prefix = match options.get_array_format() {
                    ArrayFormat::Indices => {
                        if prefix.is_empty() {
                            format!("[{idx}]")
                        } else {
                            format!("{prefix}[{idx}]")
                        }
                    }
                    ArrayFormat::Brackets => {
                        if prefix.is_empty() {
                            "[]".to_string()
                        } else {
                            format!("{prefix}[]")
                        }
                    }
                    ArrayFormat::Repeat | ArrayFormat::Comma => prefix.to_string(),
                };

                collect_pairs(item, &child_prefix, options, pairs);
            }
        }
        QsValue::Object(map) => {
            for (key, child) in map {
                let child_prefix = if prefix.is_empty() {
                    key.clone()
                } else if options.get_allow_dots() {
                    format!("{prefix}.{key}")
                } else {
                    format!("{prefix}[{key}]")
                };
                collect_pairs(child, &child_prefix, options, pairs);
            }
        }
    }
}

fn format_key(key: &str, options: &StringifyOptions) -> String {
    if !options.get_encode() {
        key.to_string()
    } else if options.get_encode_values_only() {
        encode_key_preserving_brackets(key)
    } else {
        encode_component(key)
    }
}

fn format_value(value: &str, options: &StringifyOptions) -> String {
    if options.get_encode() {
        encode_component(value)
    } else {
        value.to_string()
    }
}
