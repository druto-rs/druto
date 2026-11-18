//! RFC 6265 compliant cookie parsing, serialization, and plain/signed/encrypted cookie jars.

#[doc(hidden)]
mod dependencies {
    use crypto as _;
    use serde as _;
    use time as _;
}
