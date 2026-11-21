//! Declarative macro DSL for constructing validation schemas.

/// Constructs an [`ObjectSchema`](crate::schema::ObjectSchema) using a declarative manifest syntax.
///
/// # Examples
///
/// ```rust
/// use validator::schema;
///
/// let user_schema = validator::schema! {
///     username: schema::string().min_len(3).max_len(30),
///     email: schema::string().email(),
///     age: schema::integer().min(18),
///     bio?: schema::string().max_len(500),
/// };
/// ```
#[macro_export]
macro_rules! schema {
    ( $( $tokens:tt )* ) => {{
        let mut druto_object = $crate::schema::object();
        $crate::schema_step!(druto_object; $( $tokens )*);
        druto_object
    }};
}

#[doc(hidden)]
#[macro_export]
macro_rules! schema_step {
    // Base case: empty
    ($target:ident;) => {};

    // Trailing comma
    ($target:ident; , $( $rest:tt )*) => {
        $crate::schema_step!($target; $( $rest )*);
    };

    // Optional field syntax: name?: expr, ...
    ($target:ident; $field:ident ?: $schema:expr, $( $rest:tt )*) => {
        $target = $target.optional_field(stringify!($field), $schema);
        $crate::schema_step!($target; $( $rest )*);
    };

    // Optional field syntax (no trailing comma)
    ($target:ident; $field:ident ?: $schema:expr) => {
        $target = $target.optional_field(stringify!($field), $schema);
    };

    // Required field syntax: name: expr, ...
    ($target:ident; $field:ident : $schema:expr, $( $rest:tt )*) => {
        $target = $target.field(stringify!($field), $schema);
        $crate::schema_step!($target; $( $rest )*);
    };

    // Required field syntax (no trailing comma)
    ($target:ident; $field:ident : $schema:expr) => {
        $target = $target.field(stringify!($field), $schema);
    };
}

/// Constructs a composite [`RequestSchema`](crate::request::RequestSchema) using declarative syntax.
///
/// # Examples
///
/// ```rust
/// use validator::{request_schema, schema};
///
/// let query_schema = validator::schema! {
///     page: schema::integer().min(1),
/// };
///
/// let body_schema = validator::schema! {
///     title: schema::string().min_len(3),
/// };
///
/// let req_schema = request_schema! {
///     query: query_schema,
///     body: body_schema,
/// };
/// ```
#[macro_export]
macro_rules! request_schema {
    ( $( $tokens:tt )* ) => {{
        let mut druto_request = $crate::request::request();
        $crate::request_schema_step!(druto_request; $( $tokens )*);
        druto_request
    }};
}

#[doc(hidden)]
#[macro_export]
macro_rules! request_schema_step {
    // Base case: empty
    ($target:ident;) => {};

    // Trailing comma
    ($target:ident; , $( $rest:tt )*) => {
        $crate::request_schema_step!($target; $( $rest )*);
    };

    // params
    ($target:ident; params: $schema:expr, $( $rest:tt )*) => {
        $target = $target.params($schema);
        $crate::request_schema_step!($target; $( $rest )*);
    };
    ($target:ident; params: $schema:expr) => {
        $target = $target.params($schema);
    };

    // query
    ($target:ident; query: $schema:expr, $( $rest:tt )*) => {
        $target = $target.query($schema);
        $crate::request_schema_step!($target; $( $rest )*);
    };
    ($target:ident; query: $schema:expr) => {
        $target = $target.query($schema);
    };

    // headers
    ($target:ident; headers: $schema:expr, $( $rest:tt )*) => {
        $target = $target.headers($schema);
        $crate::request_schema_step!($target; $( $rest )*);
    };
    ($target:ident; headers: $schema:expr) => {
        $target = $target.headers($schema);
    };

    // cookies
    ($target:ident; cookies: $schema:expr, $( $rest:tt )*) => {
        $target = $target.cookies($schema);
        $crate::request_schema_step!($target; $( $rest )*);
    };
    ($target:ident; cookies: $schema:expr) => {
        $target = $target.cookies($schema);
    };

    // body
    ($target:ident; body: $schema:expr, $( $rest:tt )*) => {
        $target = $target.body($schema);
        $crate::request_schema_step!($target; $( $rest )*);
    };
    ($target:ident; body: $schema:expr) => {
        $target = $target.body($schema);
    };
}
