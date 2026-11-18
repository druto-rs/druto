use inflector::{StrBuilder, slug::SlugOptions, utils::TruncateOptions};

#[test]
fn test_builder_multi_step_pipelines() {
    let table = StrBuilder::new("AdminUserProfile")
        .snake_case()
        .pluralize()
        .into_string();
    assert_eq!(table, "admin_user_profiles");

    let model = StrBuilder::new("admin_user_profiles")
        .singularize()
        .pascal_case()
        .into_string();
    assert_eq!(model, "AdminUserProfile");

    let fk = StrBuilder::new("UserProfile").foreign_key().into_string();
    assert_eq!(fk, "user_profile_id");

    let slug = StrBuilder::new("  Rust & Druto Framework: Fast!  ")
        .slugify()
        .into_string();
    assert_eq!(slug, "rust-and-druto-framework-fast");

    let guarded_url = StrBuilder::new("https://example.com/api")
        .finish("/")
        .wrap("<", ">")
        .into_string();
    assert_eq!(guarded_url, "<https://example.com/api/>");
}

#[test]
fn test_builder_all_methods() {
    let screaming = StrBuilder::from("user_id").screaming_snake_case();
    assert_eq!(screaming.as_str(), "USER_ID");
    assert_eq!(format!("{screaming}"), "USER_ID");
    assert_eq!(screaming.as_ref(), "USER_ID");
    assert_eq!(&*screaming, "USER_ID");

    let kebab = StrBuilder::new("user_id").kebab_case().into_string();
    assert_eq!(kebab, "user-id");

    let train = StrBuilder::new("user_id").train_case().into_string();
    assert_eq!(train, "User-Id");

    let title = StrBuilder::new("user_id").title_case().into_string();
    assert_eq!(title, "User Id");

    let sentence = StrBuilder::new("user_id").sentence_case().into_string();
    assert_eq!(sentence, "User id");

    let classify = StrBuilder::new("users").classify().into_string();
    assert_eq!(classify, "User");

    let count_sing = StrBuilder::new("boxes")
        .pluralize_with_count(1)
        .into_string();
    assert_eq!(count_sing, "box");

    let count_plur = StrBuilder::new("box").pluralize_with_count(2).into_string();
    assert_eq!(count_plur, "boxes");

    let slug_opts = StrBuilder::new("Custom Title")
        .slugify_with_options(&SlugOptions::new().with_delimiter('_'))
        .into_string();
    assert_eq!(slug_opts, "custom_title");

    let trunc = StrBuilder::new("Very long string that needs truncation")
        .truncate(12)
        .into_string();
    assert_eq!(trunc, "Very long...");

    let trunc_opts = StrBuilder::new("Another long string")
        .truncate_with_options(15, &TruncateOptions::new().with_suffix("##"))
        .into_string();
    assert_eq!(trunc_opts, "Another long ##");

    let words = StrBuilder::new("One two three four five")
        .words(3, "...")
        .into_string();
    assert_eq!(words, "One two three...");

    let masked = StrBuilder::new("secret-value")
        .mask('*', 2, Some(4))
        .into_string();
    assert_eq!(masked, "se****-value");

    let started = StrBuilder::new("path").start("/").into_string();
    assert_eq!(started, "/path");

    let from_str: StrBuilder = "test".into();
    assert_eq!(from_str.into_string(), "test");

    let from_string: StrBuilder = String::from("test").into();
    assert_eq!(from_string.into_string(), "test");
}
