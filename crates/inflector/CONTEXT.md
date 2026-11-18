# `inflector` Crate Context

## Purpose

`inflector` provides English grammatical inflections (pluralization and singularization), casing transformations, and string utilities for the Druto framework. It enables convention-over-configuration throughout Druto (e.g. mapping model names to database tables, generating URLs, CLI scaffolding).

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
   - Irregular nouns (`person` $\to$ `people`, `child` $\to$ `children`, `man` $\to$ `men`, `ox` $\to$ `oxen`, `mouse` $\to$ `mice`).
   - Uncountable nouns (`money`, `information`, `rice`, `series`, `species`).
   - Custom rule registration (allowing applications to define domain-specific irregular forms).
2. **Casing Transformations**:
   - `snake_case` (e.g., `user_account_id`)
   - `camelCase` (e.g., `userAccountId`)
   - `kebab-case` (e.g., `user-account-id`)
   - `PascalCase` / `StudlyCaps` (e.g., `UserAccountId`)
   - `Title Case` (e.g., `User Account Id`)
3. **URL Slugification**:
   - Transliterate unicode and accented characters to ASCII equivalents.
   - Strip unwanted symbols and replace whitespace with configurable delimiters (e.g. `-`).
4. **General String Helpers**:
   - Truncation with ellipsis (`truncate`, `words`).
   - String masking (e.g., masking credit cards, emails, or tokens).
   - Padding and excerpting.

## Boundaries & Non-Responsibilities

- **Not a Full i18n Suite**: Focuses specifically on English grammar rules for framework naming conventions and string utilities, not multi-language translation catalogs.

## Target Public Surface

- `Inflector`: Core engine for pluralization and singularization.
- Casing conversion functions: `to_snake_case`, `to_camel_case`, `to_kebab_case`, `to_pascal_case`, `to_title_case`.
- `slugify`: URL-safe slug generator.
- Optional extension trait: `StrExt` implementing fluent methods on `&str`.
