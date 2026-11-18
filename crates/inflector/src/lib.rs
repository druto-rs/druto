//! English grammatical inflections, casing conventions, URL slugification, and string utilities.
//!
//! Part of the **Druto** web framework (Layer 0: Primitives).
//!
//! # Architecture & Capabilities
//! - **Zero-Allocation Casing**: Fast casing conversions (`to_snake_case`, `to_camel_case`,
//!   `to_kebab_case`, `to_pascal_case`, etc.) returning [`std::borrow::Cow<'_, str>`] that avoid
//!   heap allocations when the input already conforms to the target casing.
//! - **Acronym Recognition**: Automatically groups uppercase acronym sequences (e.g., `HTTPResponse` $\to$ `http_response`).
//! - **English Grammatical Inflections**: Pluralization and singularization with regular suffix rules,
//!   irregular noun dictionaries, uncountable noun lookups, count-based matching, and letter-casing preservation.
//! - **Custom Rule Engine**: Extendable [`Inflector`] engine supporting custom uncountable words,
//!   domain-specific irregulars, and dynamic regex patterns.
//! - **Framework Conventions**: High-level helpers (`tableize`, `classify`, `foreign_key`) for ORM and scaffolding.
//! - **URL Slugification**: Fast ASCII transliteration for Latin diacritics/accents, symbol expansions, and configurable delimiters.
//! - **String Utilities**: Sensitive data masking ([`mask()`], [`mask_email`]), truncation with whole-word preservation ([`truncate()`], [`words`]),
//!   context excerpting ([`excerpt`]), ordinalization ([`ordinal`]), and Oxford comma list sentences ([`sentence`]).
//! - **Ergonomic Tri-Surface API**: Direct top-level functions, fluent [`StrBuilder`] pipeline chaining, and the [`StrExt`] extension trait.

pub mod builder;
pub mod case;
pub mod ext;
pub mod grammar;
pub mod slug;
pub mod utils;

pub use builder::StrBuilder;
pub use case::{
    classify, foreign_key, foreign_key_with_suffix, is_camel_case, is_kebab_case, is_pascal_case,
    is_screaming_snake_case, is_sentence_case, is_snake_case, is_title_case, is_train_case,
    tableize, to_camel_case, to_kebab_case, to_pascal_case, to_screaming_snake_case,
    to_sentence_case, to_snake_case, to_title_case, to_train_case,
};
pub use ext::StrExt;
pub use grammar::{
    Inflector, add_irregular, add_uncountable, is_plural, is_singular, pluralize,
    pluralize_with_count, pluralize_with_count_inclusive, singularize,
};
#[cfg(feature = "regex")]
pub use grammar::{add_plural_rule, add_singular_rule};
pub use slug::{SlugOptions, slugify, slugify_with_options};
pub use utils::{
    ExcerptOptions, SentenceOptions, TruncateOptions, excerpt, finish, mask, mask_email, ordinal,
    pad_both, pad_end, pad_start, sentence, sentence_with_options, start, truncate, unwrap, words,
    wrap,
};

/// Prelude module for convenient glob-importing common types and traits.
pub mod prelude {
    pub use super::{
        builder::StrBuilder,
        ext::StrExt,
        grammar::Inflector,
        slug::SlugOptions,
        utils::{ExcerptOptions, SentenceOptions, TruncateOptions},
    };
}
