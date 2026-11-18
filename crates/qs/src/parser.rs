//! Nested query string parser.

use std::collections::BTreeMap;

use crate::{
    encoding::decode_component,
    error::QsError,
    options::{Duplicates, ParseOptions},
    value::QsValue,
};

#[derive(Debug, Clone, PartialEq, Eq)]
enum PathSegment {
    Field(String),
    Index(usize),
    EmptyBracket,
}

/// Parses a query string into a dynamic [`QsValue`] tree using the provided options.
///
/// # Errors
///
/// Returns a [`QsError`] if recursion depth exceeds `max_depth` (when `strict_depth` is true),
/// or if parameter count exceeds `parameter_limit`.
pub fn parse_query(raw_query: &str, options: &ParseOptions) -> Result<QsValue, QsError> {
    let mut query = raw_query;
    if options.get_ignore_query_prefix() && query.starts_with('?') {
        query = &query[1..];
    }

    if query.is_empty() {
        return Ok(QsValue::Object(BTreeMap::new()));
    }

    let delimiter = options.get_delimiter();
    let parts: Vec<&str> = query
        .split(delimiter as char)
        .filter(|part| !part.is_empty())
        .collect();

    if parts.len() > options.get_parameter_limit() {
        return Err(QsError::ParameterLimitExceeded(
            options.get_parameter_limit(),
        ));
    }

    let mut root = QsValue::Object(BTreeMap::new());

    for part in parts {
        let (raw_key, raw_val_opt) = split_key_value(part);

        let key = normalize_key(
            raw_key,
            options.get_allow_dots(),
            options.get_decode_dot_in_keys(),
        );

        let value = match raw_val_opt {
            Some(raw_val) => {
                let decoded_val = decode_component(raw_val);
                if options.get_comma() && decoded_val.contains(',') {
                    let items: Vec<QsValue> = decoded_val
                        .split(',')
                        .map(|item| QsValue::Scalar(item.to_string()))
                        .collect();
                    QsValue::Array(items)
                } else {
                    QsValue::Scalar(decoded_val)
                }
            }
            None => {
                if options.get_strict_null_handling() {
                    QsValue::Null
                } else {
                    QsValue::Scalar(String::new())
                }
            }
        };

        let segments = parse_key_segments(&key, options)?;
        if segments.is_empty() {
            continue;
        }

        insert_path(&mut root, &segments, value, options.get_duplicates());
    }

    if !options.get_allow_sparse() {
        compact_sparse(&mut root);
    }

    Ok(root)
}

fn split_key_value(part: &str) -> (&str, Option<&str>) {
    let equals_pos = if let Some(bracket_pos) = part.find("]=") {
        bracket_pos + 1
    } else {
        match part.find('=') {
            Some(pos) => pos,
            None => return (part, None),
        }
    };

    let key = &part[..equals_pos];
    let val = &part[equals_pos + 1..];
    (key, Some(val))
}

fn normalize_key(raw_key: &str, allow_dots: bool, decode_dot_in_keys: bool) -> String {
    let mut key = raw_key
        .replace("%5B", "[")
        .replace("%5b", "[")
        .replace("%5D", "]")
        .replace("%5d", "]");

    if decode_dot_in_keys {
        key = key.replace("%2E", ".").replace("%2e", ".");
    }

    if allow_dots {
        transform_dots_to_brackets(&key)
    } else {
        key
    }
}

fn transform_dots_to_brackets(key: &str) -> String {
    let mut result = String::with_capacity(key.len() + 8);
    let mut chars = key.chars().peekable();
    let mut in_bracket = false;

    while let Some(ch) = chars.next() {
        match ch {
            '[' => {
                in_bracket = true;
                result.push(ch);
            }
            ']' => {
                in_bracket = false;
                result.push(ch);
            }
            '.' if !in_bracket => {
                result.push('[');
                while let Some(&next_ch) = chars.peek() {
                    if next_ch == '.' || next_ch == '[' || next_ch == ']' {
                        break;
                    }
                    result.push(chars.next().expect("character checked via peek"));
                }
                result.push(']');
            }
            _ => {
                result.push(ch);
            }
        }
    }

    result
}

fn parse_key_segments(key: &str, options: &ParseOptions) -> Result<Vec<PathSegment>, QsError> {
    if key.is_empty() {
        return Ok(Vec::new());
    }

    if options.get_depth() == 0 {
        return Ok(vec![PathSegment::Field(key.to_string())]);
    }

    let first_bracket = match key.find('[') {
        Some(pos) => pos,
        None => return Ok(vec![PathSegment::Field(decode_component(key))]),
    };

    let parent = &key[..first_bracket];
    let mut segments = Vec::new();
    if !parent.is_empty() {
        segments.push(PathSegment::Field(decode_component(parent)));
    }

    let mut current_pos = first_bracket;
    let mut depth_count = 0;
    let max_depth = options.get_depth();

    while current_pos < key.len() {
        if key[current_pos..].starts_with('[') {
            if depth_count >= max_depth {
                if options.get_strict_depth() {
                    return Err(QsError::DepthLimitExceeded(max_depth));
                }
                let remainder = &key[current_pos..];
                segments.push(PathSegment::Field(decode_component(remainder)));
                break;
            }

            if let Some(close_pos) = key[current_pos + 1..].find(']') {
                let actual_close = current_pos + 1 + close_pos;
                let inside = &key[current_pos + 1..actual_close];
                depth_count += 1;

                if inside.is_empty() {
                    segments.push(PathSegment::EmptyBracket);
                } else if let Ok(index) = inside.parse::<usize>() {
                    if index <= options.get_array_limit() {
                        segments.push(PathSegment::Index(index));
                    } else {
                        segments.push(PathSegment::Field(index.to_string()));
                    }
                } else {
                    segments.push(PathSegment::Field(decode_component(inside)));
                }

                current_pos = actual_close + 1;
            } else {
                let remainder = &key[current_pos..];
                segments.push(PathSegment::Field(decode_component(remainder)));
                break;
            }
        } else {
            let next_bracket = key[current_pos..]
                .find('[')
                .unwrap_or(key[current_pos..].len());
            let chunk = &key[current_pos..current_pos + next_bracket];
            segments.push(PathSegment::Field(decode_component(chunk)));
            current_pos += next_bracket;
        }
    }

    Ok(segments)
}

fn array_to_object(list: Vec<QsValue>) -> BTreeMap<String, QsValue> {
    let mut map = BTreeMap::new();
    for (idx, item) in list.into_iter().enumerate() {
        if !item.is_null() {
            map.insert(idx.to_string(), item);
        }
    }
    map
}

fn ensure_array(current: &mut QsValue) {
    match current {
        QsValue::Array(_) => {}
        QsValue::Null => {
            *current = QsValue::Array(Vec::new());
        }
        _ => {
            let prev = std::mem::replace(current, QsValue::Null);
            *current = QsValue::Array(vec![prev]);
        }
    }
}

fn insert_path(
    current: &mut QsValue,
    segments: &[PathSegment],
    value: QsValue,
    duplicates: Duplicates,
) {
    if segments.is_empty() {
        merge_values(current, value, duplicates);
        return;
    }

    let head = &segments[0];
    let tail = &segments[1..];

    match head {
        PathSegment::Field(field_name) => {
            match current {
                QsValue::Array(_) => {
                    if let QsValue::Array(list) = std::mem::replace(current, QsValue::Null) {
                        *current = QsValue::Object(array_to_object(list));
                    }
                }
                QsValue::Object(_) => {}
                _ => {
                    *current = QsValue::Object(BTreeMap::new());
                }
            }

            if let QsValue::Object(map) = current {
                let entry = map.entry(field_name.clone()).or_insert(QsValue::Null);
                insert_path(entry, tail, value, duplicates);
            }
        }
        PathSegment::Index(idx) => {
            if let QsValue::Object(map) = current {
                let entry = map.entry(idx.to_string()).or_insert(QsValue::Null);
                insert_path(entry, tail, value, duplicates);
            } else {
                ensure_array(current);

                if let QsValue::Array(list) = current {
                    while list.len() <= *idx {
                        list.push(QsValue::Null);
                    }
                    insert_path(&mut list[*idx], tail, value, duplicates);
                }
            }
        }
        PathSegment::EmptyBracket => {
            ensure_array(current);

            if let QsValue::Array(list) = current {
                if tail.is_empty() {
                    list.push(value);
                } else {
                    let mut new_item = QsValue::Null;
                    insert_path(&mut new_item, tail, value, duplicates);
                    list.push(new_item);
                }
            }
        }
    }
}

fn merge_values(target: &mut QsValue, incoming: QsValue, duplicates: Duplicates) {
    if target.is_null() {
        *target = incoming;
        return;
    }

    match duplicates {
        Duplicates::Combine => match target {
            QsValue::Array(existing_list) => match incoming {
                QsValue::Array(incoming_list) => existing_list.extend(incoming_list),
                other => existing_list.push(other),
            },
            _ => {
                let previous = std::mem::replace(target, QsValue::Null);
                match incoming {
                    QsValue::Array(mut incoming_list) => {
                        incoming_list.insert(0, previous);
                        *target = QsValue::Array(incoming_list);
                    }
                    other => {
                        *target = QsValue::Array(vec![previous, other]);
                    }
                }
            }
        },
        Duplicates::First => {
            // Retain original value
        }
        Duplicates::Last => {
            *target = incoming;
        }
    }
}

fn compact_sparse(value: &mut QsValue) {
    match value {
        QsValue::Array(list) => {
            for item in list.iter_mut() {
                compact_sparse(item);
            }
            list.retain(|item| !item.is_null());
        }
        QsValue::Object(map) => {
            for item in map.values_mut() {
                compact_sparse(item);
            }
        }
        _ => {}
    }
}
