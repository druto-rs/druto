//! Configuration options for parsing and serializing query strings.

/// Strategy for resolving colliding unbracketed keys in a query string.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Duplicates {
    /// Merges colliding keys into an array (default).
    #[default]
    Combine,
    /// Retains only the first occurrence of a colliding key.
    First,
    /// Overwrites previous values with the last occurrence.
    Last,
}

/// Serialization format for array values in query strings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ArrayFormat {
    /// Serializes with explicit numerical indices: `a[0]=b&a[1]=c` (default).
    #[default]
    Indices,
    /// Serializes with empty bracket notation: `a[]=b&a[]=c`.
    Brackets,
    /// Repeats the key without brackets: `a=b&a=c`.
    Repeat,
    /// Serializes items as comma-separated values: `a=b,c`.
    Comma,
}

/// Configuration options for query string parsing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseOptions {
    depth: usize,
    strict_depth: bool,
    parameter_limit: usize,
    array_limit: usize,
    allow_sparse: bool,
    allow_dots: bool,
    comma: bool,
    duplicates: Duplicates,
    ignore_query_prefix: bool,
    strict_null_handling: bool,
    delimiter: u8,
    decode_dot_in_keys: bool,
}

impl Default for ParseOptions {
    fn default() -> Self {
        Self {
            depth: 5,
            strict_depth: false,
            parameter_limit: 1000,
            array_limit: 20,
            allow_sparse: false,
            allow_dots: false,
            comma: false,
            duplicates: Duplicates::Combine,
            ignore_query_prefix: true,
            strict_null_handling: false,
            delimiter: b'&',
            decode_dot_in_keys: false,
        }
    }
}

impl ParseOptions {
    /// Creates a new `ParseOptions` instance with default values.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Maximum nesting depth allowed during bracket parsing.
    #[must_use]
    pub const fn depth(mut self, depth: usize) -> Self {
        self.depth = depth;
        self
    }

    /// Whether to return an error when nesting exceeds the depth limit.
    #[must_use]
    pub const fn strict_depth(mut self, strict: bool) -> Self {
        self.strict_depth = strict;
        self
    }

    /// Maximum number of parameters to parse.
    #[must_use]
    pub const fn parameter_limit(mut self, limit: usize) -> Self {
        self.parameter_limit = limit;
        self
    }

    /// Maximum index allowed when parsing explicit array indices.
    #[must_use]
    pub const fn array_limit(mut self, limit: usize) -> Self {
        self.array_limit = limit;
        self
    }

    /// Whether to allow sparse elements in parsed arrays.
    #[must_use]
    pub const fn allow_sparse(mut self, allow: bool) -> Self {
        self.allow_sparse = allow;
        self
    }

    /// Whether to interpret dot notation (`a.b=c`) as nested objects.
    #[must_use]
    pub const fn allow_dots(mut self, allow: bool) -> Self {
        self.allow_dots = allow;
        self
    }

    /// Whether to split comma-separated values into array elements.
    #[must_use]
    pub const fn comma(mut self, comma: bool) -> Self {
        self.comma = comma;
        self
    }

    /// Strategy for resolving colliding duplicate keys.
    #[must_use]
    pub const fn duplicates(mut self, duplicates: Duplicates) -> Self {
        self.duplicates = duplicates;
        self
    }

    /// Whether to ignore a leading `?` character in the query string.
    #[must_use]
    pub const fn ignore_query_prefix(mut self, ignore: bool) -> Self {
        self.ignore_query_prefix = ignore;
        self
    }

    /// Whether values without an `=` sign should be parsed as `QsValue::Null`.
    #[must_use]
    pub const fn strict_null_handling(mut self, strict: bool) -> Self {
        self.strict_null_handling = strict;
        self
    }

    /// Byte delimiter separating query parameters.
    #[must_use]
    pub const fn delimiter(mut self, delimiter: u8) -> Self {
        self.delimiter = delimiter;
        self
    }

    /// Whether to decode `%2E` to `.` in keys when dot notation is enabled.
    #[must_use]
    pub const fn decode_dot_in_keys(mut self, decode: bool) -> Self {
        self.decode_dot_in_keys = decode;
        self
    }

    /// Returns the maximum nesting depth.
    #[must_use]
    pub const fn get_depth(&self) -> usize {
        self.depth
    }

    /// Returns whether strict depth checking is enabled.
    #[must_use]
    pub const fn get_strict_depth(&self) -> bool {
        self.strict_depth
    }

    /// Returns the parameter count limit.
    #[must_use]
    pub const fn get_parameter_limit(&self) -> usize {
        self.parameter_limit
    }

    /// Returns the maximum array index limit.
    #[must_use]
    pub const fn get_array_limit(&self) -> usize {
        self.array_limit
    }

    /// Returns whether sparse arrays are allowed.
    #[must_use]
    pub const fn get_allow_sparse(&self) -> bool {
        self.allow_sparse
    }

    /// Returns whether dot notation is enabled.
    #[must_use]
    pub const fn get_allow_dots(&self) -> bool {
        self.allow_dots
    }

    /// Returns whether comma-separated array splitting is enabled.
    #[must_use]
    pub const fn get_comma(&self) -> bool {
        self.comma
    }

    /// Returns the duplicate key resolution strategy.
    #[must_use]
    pub const fn get_duplicates(&self) -> Duplicates {
        self.duplicates
    }

    /// Returns whether leading `?` is ignored.
    #[must_use]
    pub const fn get_ignore_query_prefix(&self) -> bool {
        self.ignore_query_prefix
    }

    /// Returns whether strict null handling is enabled.
    #[must_use]
    pub const fn get_strict_null_handling(&self) -> bool {
        self.strict_null_handling
    }

    /// Returns the parameter delimiter byte.
    #[must_use]
    pub const fn get_delimiter(&self) -> u8 {
        self.delimiter
    }

    /// Returns whether `%2E` is decoded to `.` in keys.
    #[must_use]
    pub const fn get_decode_dot_in_keys(&self) -> bool {
        self.decode_dot_in_keys
    }
}

/// Configuration options for query string serialization.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StringifyOptions {
    array_format: ArrayFormat,
    encode: bool,
    encode_values_only: bool,
    add_query_prefix: bool,
    skip_nulls: bool,
    strict_null_handling: bool,
    sort: bool,
    delimiter: u8,
    allow_dots: bool,
}

impl Default for StringifyOptions {
    fn default() -> Self {
        Self {
            array_format: ArrayFormat::Indices,
            encode: true,
            encode_values_only: false,
            add_query_prefix: false,
            skip_nulls: false,
            strict_null_handling: false,
            sort: true,
            delimiter: b'&',
            allow_dots: false,
        }
    }
}

impl StringifyOptions {
    /// Creates a new `StringifyOptions` instance with default values.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Serialization format for array values.
    #[must_use]
    pub const fn array_format(mut self, format: ArrayFormat) -> Self {
        self.array_format = format;
        self
    }

    /// Whether to percent-encode query keys and values according to RFC 3986.
    #[must_use]
    pub const fn encode(mut self, encode: bool) -> Self {
        self.encode = encode;
        self
    }

    /// Whether to percent-encode values only, keeping brackets in keys unencoded.
    #[must_use]
    pub const fn encode_values_only(mut self, encode_values_only: bool) -> Self {
        self.encode_values_only = encode_values_only;
        self
    }

    /// Whether to prepend a `?` prefix to the generated query string.
    #[must_use]
    pub const fn add_query_prefix(mut self, add_prefix: bool) -> Self {
        self.add_query_prefix = add_prefix;
        self
    }

    /// Whether to omit null values from the serialized output.
    #[must_use]
    pub const fn skip_nulls(mut self, skip: bool) -> Self {
        self.skip_nulls = skip;
        self
    }

    /// Whether null values should be serialized without an `=` sign (`key` vs `key=`).
    #[must_use]
    pub const fn strict_null_handling(mut self, strict: bool) -> Self {
        self.strict_null_handling = strict;
        self
    }

    /// Whether to sort object keys alphabetically for deterministic output.
    #[must_use]
    pub const fn sort(mut self, sort: bool) -> Self {
        self.sort = sort;
        self
    }

    /// Delimiter byte separating key-value pairs.
    #[must_use]
    pub const fn delimiter(mut self, delimiter: u8) -> Self {
        self.delimiter = delimiter;
        self
    }

    /// Whether to serialize nested objects using dot notation (`a.b=c`) instead of brackets.
    #[must_use]
    pub const fn allow_dots(mut self, allow: bool) -> Self {
        self.allow_dots = allow;
        self
    }

    /// Returns the array serialization format.
    #[must_use]
    pub const fn get_array_format(&self) -> ArrayFormat {
        self.array_format
    }

    /// Returns whether percent-encoding is enabled.
    #[must_use]
    pub const fn get_encode(&self) -> bool {
        self.encode
    }

    /// Returns whether only values are encoded.
    #[must_use]
    pub const fn get_encode_values_only(&self) -> bool {
        self.encode_values_only
    }

    /// Returns whether `?` prefix is added.
    #[must_use]
    pub const fn get_add_query_prefix(&self) -> bool {
        self.add_query_prefix
    }

    /// Returns whether null values are skipped.
    #[must_use]
    pub const fn get_skip_nulls(&self) -> bool {
        self.skip_nulls
    }

    /// Returns whether strict null serialization is enabled.
    #[must_use]
    pub const fn get_strict_null_handling(&self) -> bool {
        self.strict_null_handling
    }

    /// Returns whether keys are sorted alphabetically.
    #[must_use]
    pub const fn get_sort(&self) -> bool {
        self.sort
    }

    /// Returns the delimiter byte.
    #[must_use]
    pub const fn get_delimiter(&self) -> u8 {
        self.delimiter
    }

    /// Returns whether dot notation is used for object serialization.
    #[must_use]
    pub const fn get_allow_dots(&self) -> bool {
        self.allow_dots
    }
}
