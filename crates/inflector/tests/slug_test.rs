use inflector::{StrExt, slug::*};

#[test]
fn test_latin_diacritics_transliteration() {
    assert_eq!(slugify("Café Déjà Vu"), "cafe-deja-vu");
    assert_eq!(slugify("München"), "munchen");
    assert_eq!(slugify("El Niño"), "el-nino");
    assert_eq!(slugify("Crème Brûlée"), "creme-brulee");
    assert_eq!(slugify("Strauß"), "strauss");
    assert_eq!(slugify("Łódź"), "lodz");
    assert_eq!(slugify("São Paulo"), "sao-paulo");
    assert_eq!(slugify("Zürich"), "zurich");
}

#[test]
fn test_symbol_expansion_and_sanitization() {
    assert_eq!(slugify("Tom & Jerry"), "tom-and-jerry");
    assert_eq!(slugify("contact@domain.com"), "contact-at-domain-com");
    assert_eq!(slugify("100% Cotton & Wool"), "100-cotton-and-wool");
    assert_eq!(
        slugify("   leading and trailing   "),
        "leading-and-trailing"
    );
    assert_eq!(
        slugify("multiple---hyphens___and...dots"),
        "multiple-hyphens-and-dots"
    );
}

#[test]
fn test_custom_slug_options() {
    let options = SlugOptions::new()
        .with_delimiter('_')
        .with_lowercase(false)
        .with_replacement('$', "dollar");

    assert_eq!(
        slugify_with_options("Price: $50", &options),
        "Price_dollar_50"
    );

    let disabled_symbols = SlugOptions::new().with_symbol_expansion(false);
    assert_eq!(
        slugify_with_options("Cats & Dogs", &disabled_symbols),
        "cats-dogs"
    );
}

#[test]
fn test_unicode_retention() {
    let unicode_opts = SlugOptions::new().with_allow_unicode(true);
    assert_eq!(
        slugify_with_options("Druto フレームワーク Rocks", &unicode_opts),
        "druto-フレームワーク-rocks"
    );
}

#[test]
fn test_slug_fluent_extension() {
    assert_eq!("My First Blog Post!".to_slug(), "my-first-blog-post");
}

#[test]
fn test_all_transliterations() {
    let input = "æ Æ ç Ç đ Đ è È ì Ì ł Ł ñ Ñ ò Ò œ Œ ś Ś ß ť Ť þ Þ ù Ù ý Ý ź Ź";
    let slug = slugify(input);
    assert_eq!(
        slug,
        "ae-ae-c-c-d-d-e-e-i-i-l-l-n-n-o-o-oe-oe-s-s-ss-t-t-th-th-u-u-y-y-z-z"
    );
}
