use inflector::{
    StrBuilder, StrExt,
    grammar::{
        Inflector, add_irregular, add_plural_rule, add_singular_rule, add_uncountable, is_plural,
        is_singular, pluralize, singularize,
    },
    slug::{SlugOptions, slugify_with_options},
    utils::{
        ExcerptOptions, SentenceOptions, TruncateOptions, excerpt, finish, mask_email, pad_both,
        pad_end, pad_start, sentence_with_options, start, truncate, unwrap, words, wrap,
    },
};

#[test]
fn test_builder_additional_coverage() {
    let table = StrBuilder::new("UserProfile").tableize().into_string();
    assert_eq!(table, "user_profiles");

    let default_builder = StrBuilder::default();
    assert_eq!(default_builder.into_string(), "");
}

#[test]
fn test_casing_empty_and_edge_inputs() {
    use inflector::case::*;

    assert_eq!(to_camel_case(""), "");
    assert_eq!(to_pascal_case(""), "");
    assert_eq!(to_sentence_case(""), "");
    assert_eq!(to_snake_case(""), "");
    assert_eq!(to_title_case(""), "");
    assert_eq!(to_train_case(""), "");
    assert_eq!(to_kebab_case(""), "");

    assert_eq!(to_camel_case("   "), "");
    assert_eq!(to_pascal_case("   "), "");
    assert_eq!(to_sentence_case("   "), "");
    assert_eq!(to_snake_case("   "), "");
    assert_eq!(to_title_case("   "), "");
    assert_eq!(to_train_case("   "), "");
    assert_eq!(to_kebab_case("   "), "");

    // Acronym at end with no lowercase following
    assert_eq!(to_snake_case("UserAPI"), "user_api");
    assert_eq!(to_camel_case("UserAPI"), "userApi");
    assert_eq!(to_pascal_case("UserAPI"), "UserApi");
}

#[test]
fn test_str_ext_additional_methods() {
    assert_eq!("HelloWorld".to_snake_case(), "hello_world");
    assert_eq!("hello_world".to_pascal_case(), "HelloWorld");
    assert_eq!("hello-world".tableize(), "hello_worlds");
    assert_eq!("hello_worlds".classify(), "HelloWorld");
    assert_eq!("user".foreign_key(), "user_id");
}

#[test]
fn test_case_preserver_edge_cases() {
    assert_eq!(pluralize("123"), "123s");
    assert_eq!(pluralize(""), "");
    assert_eq!(singularize(""), "");
    assert_eq!(pluralize("A"), "As");
    assert_eq!(pluralize("a"), "as");
}

#[test]
fn test_grammar_rules_and_predicates_edges() {
    let default_inflector = Inflector::default();
    assert_eq!(default_inflector.pluralize("user"), "users");

    // Trailing separators
    assert_eq!(pluralize("user_"), "user_s");
    assert_eq!(singularize("user_s"), "user_");

    // Suffix rules for calf, scarf, loaf, thief
    assert_eq!(pluralize("calf"), "calves");
    assert_eq!(singularize("calves"), "calf");
    assert_eq!(pluralize("scarf"), "scarves");
    assert_eq!(singularize("scarves"), "scarf");
    assert_eq!(pluralize("loaf"), "loaves");
    assert_eq!(singularize("loaves"), "loaf");
    assert_eq!(pluralize("thief"), "thieves");
    assert_eq!(singularize("thieves"), "thief");

    // -o endings
    assert_eq!(pluralize("veto"), "vetoes");
    assert_eq!(singularize("vetoes"), "veto");
    assert_eq!(pluralize("torpedo"), "torpedoes");
    assert_eq!(singularize("torpedoes"), "torpedo");
    assert_eq!(pluralize("embargo"), "embargoes");
    assert_eq!(singularize("embargoes"), "embargo");

    // -y with single char
    assert_eq!(pluralize("y"), "ys");

    // Predicates for irregulars
    assert!(!is_plural("person"));
    assert!(!is_singular("people"));
    assert!(!is_plural("child"));
    assert!(!is_singular("children"));

    // Predicates for regular nouns
    assert!(!is_plural("user"));
    assert!(is_plural("users"));
    assert!(is_singular("user"));
    assert!(!is_singular("users"));

    // Invalid regex error handling
    assert!(add_plural_rule(r"([invalid", "err").is_err());
    assert!(add_singular_rule(r"([invalid", "err").is_err());

    // Global add_plural_rule
    add_plural_rule(r"([a-zA-Z0-9_]*)globperson$", "${1}globs").unwrap();
    assert_eq!(pluralize("superglobperson"), "superglobs");

    // Global add_singular_rule
    add_singular_rule(r"([a-zA-Z0-9_]*)globs$", "${1}globperson").unwrap();
    assert_eq!(singularize("superglobs"), "superglobperson");

    // Global add_irregular and add_uncountable
    add_irregular("matrixz", "matrixzes");
    assert_eq!(pluralize("matrixz"), "matrixzes");
    assert_eq!(singularize("matrixzes"), "matrixz");

    add_uncountable("equipmentz");
    assert_eq!(pluralize("equipmentz"), "equipmentz");
    assert_eq!(singularize("equipmentz"), "equipmentz");
}

#[test]
fn test_slug_all_char_mappings() {
    let options = SlugOptions::new()
        .with_delimiter('-')
        .with_lowercase(false)
        .with_allow_unicode(false)
        .with_symbol_expansion(true);

    let sample = "À Á Â Ã Ä Å Ā Ă Ą Æ Ç Ć Ĉ Ċ Č Đ Ð È É Ê Ë Ē Ĕ Ė Ę Ě Ì Í Î Ï Ĩ Ī Ĭ Į İ Ł Ñ Ń Ņ Ň Ò Ó Ô Õ Ö Ø Ō Ŏ Ő Œ Ś Ŝ Ş Š ß Ť Ţ Þ Ù Ú Û Ü Ũ Ū Ŭ Ů Ű Ų Ý Ŷ Ÿ Ź Ż Ž";
    let slug = slugify_with_options(sample, &options);
    assert!(!slug.is_empty());
}

#[test]
fn test_utils_all_edge_cases() {
    // SentenceOptions with_separator
    let sentence_opts = SentenceOptions::new()
        .with_separator("; ")
        .with_pair_separator(" and ")
        .with_last_separator("; and ");
    assert_eq!(
        sentence_with_options(&["one", "two", "three"], &sentence_opts),
        "one; two; and three"
    );

    // Mask email edge cases
    assert_eq!(mask_email("@domain.com", '*'), "@domain.com");

    // Pad already longer
    assert_eq!(pad_start("long_string", 5, ' '), "long_string");
    assert_eq!(pad_end("long_string", 5, ' '), "long_string");
    assert_eq!(pad_both("long_string", 5, ' '), "long_string");

    // Truncate options with_suffix
    let trunc_opts = TruncateOptions::new().with_suffix("...");
    assert_eq!(truncate("hello", 10, &trunc_opts), "hello");

    // Words edge cases
    assert_eq!(words("", 5, "..."), "");
    assert_eq!(words("hello world", 0, "..."), "");

    // Excerpt options with_suffix and boundary branches
    let excerpt_opts = ExcerptOptions::new().with_radius(20).with_suffix("###");

    // Empty input / term
    assert_eq!(excerpt("", "term", &excerpt_opts), "");
    assert_eq!(excerpt("text", "", &excerpt_opts), "");

    // Term not found
    assert_eq!(
        excerpt("some sample text", "nonexistent", &excerpt_opts),
        ""
    );

    // Match at start (no prefix omission)
    let at_start = excerpt(
        "start of a long sentence that continues quite far",
        "start",
        &excerpt_opts,
    );
    assert!(!at_start.starts_with("###"));
    assert!(at_start.ends_with("###"));

    // Match at end (no suffix omission)
    let at_end = excerpt(
        "a long sentence that eventually leads to the finish",
        "finish",
        &excerpt_opts,
    );
    assert!(at_end.starts_with("###"));
    assert!(!at_end.ends_with("###"));

    // Match with neither prefix nor suffix omission
    let short_opts = ExcerptOptions::new().with_radius(100);
    let all_in = excerpt("short sentence with term inside", "term", &short_opts);
    assert_eq!(all_in, "short sentence with term inside");

    // Unwrap without match
    assert_eq!(unwrap("not-wrapped", "<", ">"), "not-wrapped");
    assert_eq!(
        unwrap("short", "very-long-prefix", "very-long-suffix"),
        "short"
    );

    // Start / finish already present
    assert_eq!(start("/path", "/"), "/path");
    assert_eq!(finish("url/", "/"), "url/");
    assert_eq!(wrap("val", "[", "]"), "[val]");
}

#[test]
fn test_final_coverage_boosters() {
    use inflector::case::*;

    assert_eq!(to_screaming_snake_case(""), "");
    assert_eq!(to_screaming_snake_case("   "), "");

    // pluralize and singularize with caves
    assert_eq!(singularize("caves"), "cave");
    assert_eq!(pluralize("cave"), "caves");

    // TitleCase with empty string
    use inflector::grammar::*;
    let default_inf = Inflector::new();
    assert_eq!(default_inf.pluralize("userAccount"), "userAccounts");
    assert_eq!(default_inf.singularize("userAccounts"), "userAccount");
}
