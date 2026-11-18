//! General full-stack string utilities.

pub mod format;
pub mod mask;
pub mod pad;
pub mod truncate;

pub use format::{SentenceOptions, ordinal, sentence, sentence_with_options};
pub use mask::{mask, mask_email};
pub use pad::{finish, pad_both, pad_end, pad_start, start, unwrap, wrap};
pub use truncate::{ExcerptOptions, TruncateOptions, excerpt, truncate, words};
