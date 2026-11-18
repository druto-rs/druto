//! Zero-allocation parameter storage and on-demand percent decoding.

use std::{borrow::Cow, fmt, str::FromStr};

use crate::error::MatcherError;

const STACK_PARAMS_CAPACITY: usize = 16;

/// Zero-allocation parameter collection storing borrowed path slices.
///
/// Stores up to 16 route parameters directly on the stack without heap allocation.
#[derive(Clone)]
pub struct Params<'a> {
    stack: [(&'a str, &'a str); STACK_PARAMS_CAPACITY],
    count: usize,
    heap: Option<Vec<(&'a str, &'a str)>>,
}

impl<'a> Default for Params<'a> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a> Params<'a> {
    /// Creates an empty parameter collection.
    #[must_use]
    pub fn new() -> Self {
        Self {
            stack: [("", ""); STACK_PARAMS_CAPACITY],
            count: 0,
            heap: None,
        }
    }

    /// Appends a key-value parameter pair.
    pub fn push(&mut self, key: &'a str, value: &'a str) {
        if let Some(ref mut heap_vec) = self.heap {
            heap_vec.push((key, value));
            self.count += 1;
        } else if self.count < STACK_PARAMS_CAPACITY {
            self.stack[self.count] = (key, value);
            self.count += 1;
        } else {
            let mut heap_vec = Vec::with_capacity(self.count + 4);
            heap_vec.extend_from_slice(&self.stack[..self.count]);
            heap_vec.push((key, value));
            self.heap = Some(heap_vec);
            self.count += 1;
        }
    }

    /// Look up a raw borrowed parameter value by name.
    #[must_use]
    pub fn get(&self, name: &str) -> Option<&'a str> {
        for (param_key, value) in self.iter() {
            if param_key == name {
                return Some(value);
            }
        }
        None
    }

    /// Explicit alias for [`Params::get`].
    #[must_use]
    pub fn get_raw(&self, name: &str) -> Option<&'a str> {
        self.get(name)
    }

    /// Returns the parameter value, percent-decoding it on demand.
    ///
    /// Adheres to RFC 3986 path decoding (preserves literal `+` characters).
    /// If the value contains no percent escapes, it borrows the slice with zero allocation.
    pub fn get_decoded(&self, name: &str) -> Result<Option<Cow<'a, str>>, MatcherError> {
        self.get(name)
            .map(|raw| percent_decode_path(raw, name))
            .transpose()
    }

    /// Re-binds parameter keys to route-specific parameter names.
    pub fn apply_param_names(&mut self, names: &'a [String]) {
        if let Some(ref mut heap_vec) = self.heap {
            for (idx, name) in names.iter().enumerate() {
                if idx < heap_vec.len() {
                    heap_vec[idx].0 = name.as_str();
                }
            }
        } else {
            for (idx, name) in names.iter().enumerate() {
                if idx < self.count {
                    self.stack[idx].0 = name.as_str();
                }
            }
        }
    }

    /// Parses the parameter into a typed value using [`FromStr`].
    pub fn parse<U: FromStr>(&self, name: &str) -> Result<U, MatcherError> {
        let raw = self
            .get(name)
            .ok_or_else(|| MatcherError::MissingParameter {
                name: name.to_string(),
                route: "path lookup".to_string(),
            })?;

        raw.parse::<U>()
            .map_err(|_| MatcherError::ConstraintFailed {
                param: name.to_string(),
                value: raw.to_string(),
                constraint: std::any::type_name::<U>().to_string(),
            })
    }

    /// Returns the number of extracted parameters.
    #[must_use]
    pub fn len(&self) -> usize {
        self.count
    }

    /// Returns true if no parameters were extracted.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.count == 0
    }

    /// Iterates over borrowed `(key, value)` parameter pairs.
    pub fn iter(&self) -> impl Iterator<Item = (&'a str, &'a str)> {
        match &self.heap {
            Some(heap_vec) => ParamsIter::Heap(heap_vec.iter()),
            None => ParamsIter::Stack(self.stack[..self.count].iter()),
        }
    }

    /// Truncates the parameter collection to a previous length.
    pub fn truncate(&mut self, len: usize) {
        if len < self.count {
            self.count = len;
            if let Some(ref mut heap_vec) = self.heap {
                heap_vec.truncate(len);
            }
        }
    }

    /// Clears all parameters for reuse.
    pub fn clear(&mut self) {
        self.count = 0;
        self.heap = None;
    }
}

enum ParamsIter<'a, 'b> {
    Stack(std::slice::Iter<'b, (&'a str, &'a str)>),
    Heap(std::slice::Iter<'b, (&'a str, &'a str)>),
}

impl<'a, 'b> Iterator for ParamsIter<'a, 'b> {
    type Item = (&'a str, &'a str);

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            Self::Stack(iter) => iter.next().copied(),
            Self::Heap(iter) => iter.next().copied(),
        }
    }
}

impl<'a> fmt::Debug for Params<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_map().entries(self.iter()).finish()
    }
}

impl<'a> PartialEq for Params<'a> {
    fn eq(&self, other: &Self) -> bool {
        if self.len() != other.len() {
            return false;
        }
        for (k1, v1) in self.iter() {
            if other.get(k1) != Some(v1) {
                return false;
            }
        }
        true
    }
}

impl<'a> Eq for Params<'a> {}

#[cfg(feature = "serde")]
impl<'a> serde::Serialize for Params<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeMap;
        let mut map = serializer.serialize_map(Some(self.len()))?;
        for (k, v) in self.iter() {
            map.serialize_entry(k, v)?;
        }
        map.end()
    }
}

/// Query string parameters borrowed from the original URI.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueryParams<'a> {
    raw: &'a str,
}

impl<'a> QueryParams<'a> {
    /// Creates a query parameter view over a raw query string.
    #[must_use]
    pub fn new(raw: &'a str) -> Self {
        Self { raw }
    }

    /// Returns the raw unparsed query string slice.
    #[must_use]
    pub fn raw(&self) -> &'a str {
        self.raw
    }

    /// Returns the first raw value for the given query parameter name.
    #[must_use]
    pub fn get(&self, name: &str) -> Option<&'a str> {
        self.iter()
            .find_map(|(key, val)| (key == name).then_some(val))
    }

    /// Returns the first percent-decoded value for the query parameter.
    ///
    /// Decodes `+` to space according to `application/x-www-form-urlencoded`.
    pub fn get_decoded(&self, name: &str) -> Result<Option<Cow<'a, str>>, MatcherError> {
        let Some(raw_val) = self.get(name) else {
            return Ok(None);
        };
        percent_decode_query(raw_val, name).map(Some)
    }

    /// Iterates over all key-value pairs in the query string.
    pub fn iter(&self) -> impl Iterator<Item = (&'a str, &'a str)> {
        self.raw
            .split('&')
            .filter(|pair| !pair.is_empty())
            .map(|pair| match pair.split_once('=') {
                Some((k, v)) => (k, v),
                None => (pair, ""),
            })
    }
}

/// Decodes percent-encoded bytes for URL path parameters adhering to RFC 3986.
///
/// Unlike query strings, `+` is an unreserved sub-delimiter in path segments and is preserved.
pub fn percent_decode_path<'a>(input: &'a str, param: &str) -> Result<Cow<'a, str>, MatcherError> {
    percent_decode_internal(input, param, false)
}

/// Decodes percent-encoded bytes for query strings according to `application/x-www-form-urlencoded`.
///
/// Converts `+` to space `' '` alongside decoding `%XX` sequences.
pub fn percent_decode_query<'a>(input: &'a str, param: &str) -> Result<Cow<'a, str>, MatcherError> {
    percent_decode_internal(input, param, true)
}

/// Decodes percent-encoded bytes into a UTF-8 string slice or owned string.
///
/// By default, decodes path parameter segments preserving literal `+`.
pub fn percent_decode<'a>(input: &'a str, param: &str) -> Result<Cow<'a, str>, MatcherError> {
    percent_decode_path(input, param)
}

fn percent_decode_internal<'a>(
    input: &'a str,
    param: &str,
    decode_plus: bool,
) -> Result<Cow<'a, str>, MatcherError> {
    if !input.contains('%') && (!decode_plus || !input.contains('+')) {
        return Ok(Cow::Borrowed(input));
    }

    let mut bytes = Vec::with_capacity(input.len());
    let mut byte_iter = input.bytes();

    while let Some(byte) = byte_iter.next() {
        if byte == b'%' {
            let high = byte_iter.next().ok_or_else(|| MatcherError::DecodeError {
                param: param.to_string(),
                value: input.to_string(),
            })?;
            let low = byte_iter.next().ok_or_else(|| MatcherError::DecodeError {
                param: param.to_string(),
                value: input.to_string(),
            })?;
            let val = hex_to_u8(high, low).ok_or_else(|| MatcherError::DecodeError {
                param: param.to_string(),
                value: input.to_string(),
            })?;
            bytes.push(val);
        } else if decode_plus && byte == b'+' {
            bytes.push(b' ');
        } else {
            bytes.push(byte);
        }
    }

    let decoded = String::from_utf8(bytes).map_err(|_| MatcherError::DecodeError {
        param: param.to_string(),
        value: input.to_string(),
    })?;

    Ok(Cow::Owned(decoded))
}

/// Converts two ASCII hex characters into a single byte.
fn hex_to_u8(high: u8, low: u8) -> Option<u8> {
    let h = hex_char_val(high)?;
    let l = hex_char_val(low)?;
    Some((h << 4) | l)
}

fn hex_char_val(ch: u8) -> Option<u8> {
    match ch {
        b'0'..=b'9' => Some(ch - b'0'),
        b'a'..=b'f' => Some(ch - b'a' + 10),
        b'A'..=b'F' => Some(ch - b'A' + 10),
        _ => None,
    }
}
