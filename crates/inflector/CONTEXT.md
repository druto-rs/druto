# `inflector` Crate Context

## Purpose

`inflector` provides English grammatical inflections (pluralization and singularization), casing transformations, and
string utilities for the Druto framework. It enables convention-over-configuration throughout Druto (e.g. mapping model
names to database tables, generating URLs, CLI scaffolding).

## Reference Architecture

This crate is the Rust equivalent of:

- **AdonisJS**: `@poppinss/string`
- **Laravel**: `illuminate/support/Str` and `illuminate/support/Pluralizer`
- **Rails**: ActiveSupport `Inflector`

In full-stack frameworks, convention-over-configuration relies heavily on reliable string inflection:

- Model `User` $\to$ Table `users`
- Model `Person` $\to$ Table `people`
- Model `Category` $\to$ Table `categories`
- Identifier `user_profile_id` $\to$ Property `userProfileId`

## Core Responsibilities

1. **Pluralization & Singularization**:
    - English grammar rules: regular endings (`-s`, `-es`, `-ies`, `-ves`).
    - Irregular nouns (`person` $\to$ `people`, `child` $\to$ `children`, `man` $\to$ `men`, `ox` $\to$ `oxen`,
      `mouse` $\to$ `mice`).
    - Uncountable nouns (`money`, `information`, `rice`, `series`, `species`).
    - Custom rule registration (allowing applications to define domain-specific irregular forms).
2. **Casing Transformations & Naming Conventions**:
    - `snake_case` (e.g., `user_account_id`) & `SCREAMING_SNAKE_CASE` (e.g., `USER_ACCOUNT_ID`)
    - `camelCase` (e.g., `userAccountId`)
    - `kebab-case` (e.g., `user-account-id`)
    - `PascalCase` / `StudlyCaps` (e.g., `UserAccountId`)
    - `Title Case` (e.g., `User Account Id`)
    - `Train-Case` / `Header-Case` (e.g., `User-Account-Id`)
    - `Sentence case` (e.g., `User account id`)
    - Framework naming conventions: `classify` (`users` $\to$ `User`), `tableize` (`UserProfile` $\to$ `user_profiles`),
      `foreign_key` (`User` $\to$ `user_id`), `foreign_key_with_suffix`.
    - Zero-allocation predicates (`is_snake_case`, `is_camel_case`, `is_pascal_case`, etc.) and `Cow<'_, str>` zero-copy
      return types.
3. **URL Slugification**:
    - Transliterate unicode and accented characters to ASCII equivalents.
    - Strip unwanted symbols and replace whitespace with configurable delimiters (e.g. `-`).
    - Configurable options via `SlugOptions` (case folding, custom replacements, delimiter options).
4. **General String Helpers**:
    - Truncation with ellipsis: `truncate`, `words`, and contextual `excerpt`.
    - Sensitive data masking: `mask`, `mask_email`.
    - Padding and boundary guards: `pad_start`, `pad_end`, `pad_both`, `start`, `finish`, `wrap`, `unwrap`.
    - List and numeric formatting: `ordinal` (e.g. `1` $\to$ `1st`), `sentence` (joining lists with Oxford comma /
      custom connector).
5. **Fluent Pipeline & Extensions**:
    - `StringPipeline` for chaining multi-step string transformations.
    - `StrExt` trait providing fluent methods on string slices.

## Boundaries & Non-Responsibilities

- **Not a Full i18n Suite**: Focuses specifically on English grammar rules for framework naming conventions and string
  utilities, not multi-language translation catalogs.

## Public API Surface

- **Grammar & Inflection**: `Inflector`, `pluralize`, `pluralize_with_count`, `singularize`, `is_plural`, `is_singular`,
  `is_uncountable`.
- **Casing Transformations**: `to_snake_case`, `to_screaming_snake_case`, `to_camel_case`, `to_kebab_case`,
  `to_pascal_case`, `to_title_case`, `to_train_case`, `to_sentence_case`.
- **Casing Predicates**: `is_snake_case`, `is_screaming_snake_case`, `is_camel_case`, `is_kebab_case`, `is_pascal_case`,
  `is_title_case`, `is_train_case`, `is_sentence_case`.
- **Framework Conventions**: `classify`, `tableize`, `foreign_key`, `foreign_key_with_suffix`.
- **Slugification**: `slugify`, `SlugOptions`.
- **Utilities**: `truncate`, `excerpt`, `words`, `mask`, `mask_email`, `pad_start`, `pad_end`, `pad_both`, `start`,
  `finish`, `wrap`, `unwrap`, `ordinal`, `sentence`.
- **Pipeline & Extension Trait**: `StringPipeline`, `string_pipeline`, `StrExt`.
