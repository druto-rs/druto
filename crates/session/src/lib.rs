//! Multi-driver HTTP session manager, flash message storage, and session fixation defense.

#[doc(hidden)]
mod dependencies {
    use cookie as _;
    use crypto as _;
    use pipeline as _;
    use serde as _;
    use serde_json as _;
}
