use inflector::{StrExt, grammar::*};

#[test]
fn test_irregular_roundtrip() {
    let pairs = [
        ("person", "people"),
        ("man", "men"),
        ("woman", "women"),
        ("child", "children"),
        ("ox", "oxen"),
        ("mouse", "mice"),
        ("louse", "lice"),
        ("foot", "feet"),
        ("tooth", "teeth"),
        ("goose", "geese"),
        ("die", "dice"),
        ("quiz", "quizzes"),
        ("matrix", "matrices"),
        ("vertex", "vertices"),
        ("index", "indices"),
        ("cactus", "cacti"),
        ("focus", "foci"),
        ("fungus", "fungi"),
        ("nucleus", "nuclei"),
        ("syllabus", "syllabi"),
        ("analysis", "analyses"),
        ("basis", "bases"),
        ("diagnosis", "diagnoses"),
        ("parenthesis", "parentheses"),
        ("thesis", "theses"),
        ("crisis", "crises"),
        ("criterion", "criteria"),
        ("phenomenon", "phenomena"),
        ("datum", "data"),
        ("medium", "media"),
        ("bacterium", "bacteria"),
        ("curriculum", "curricula"),
    ];

    for (singular, plural) in pairs {
        assert_eq!(
            pluralize(singular),
            plural,
            "Pluralizing '{singular}' failed"
        );
        assert_eq!(
            singularize(plural),
            singular,
            "Singularizing '{plural}' failed"
        );

        assert!(singular.is_singular(), "'{singular}' should be singular");
        assert!(plural.is_plural(), "'{plural}' should be plural");
    }
}

#[test]
fn test_regular_roundtrip() {
    let pairs = [
        ("user", "users"),
        ("car", "cars"),
        ("category", "categories"),
        ("city", "cities"),
        ("query", "queries"),
        ("day", "days"),
        ("boy", "boys"),
        ("key", "keys"),
        ("box", "boxes"),
        ("church", "churches"),
        ("dish", "dishes"),
        ("glass", "glasses"),
        ("status", "statuses"),
        ("virus", "viruses"),
        ("knife", "knives"),
        ("wife", "wives"),
        ("life", "lives"),
        ("half", "halves"),
        ("wolf", "wolves"),
        ("shelf", "shelves"),
        ("leaf", "leaves"),
        ("hero", "heroes"),
        ("potato", "potatoes"),
        ("tomato", "tomatoes"),
        ("echo", "echoes"),
    ];

    for (singular, plural) in pairs {
        assert_eq!(
            pluralize(singular),
            plural,
            "Pluralizing '{singular}' failed"
        );
        assert_eq!(
            singularize(plural),
            singular,
            "Singularizing '{plural}' failed"
        );

        assert!(singular.is_singular(), "'{singular}' should be singular");
        assert!(plural.is_plural(), "'{plural}' should be plural");
    }
}

#[test]
fn test_uncountable_roundtrip() {
    let uncountables = [
        "money",
        "information",
        "rice",
        "equipment",
        "series",
        "species",
        "sheep",
        "deer",
        "fish",
        "news",
        "police",
        "music",
        "traffic",
        "furniture",
        "baggage",
        "luggage",
        "advice",
        "software",
    ];

    for word in uncountables {
        assert_eq!(
            pluralize(word),
            word,
            "Pluralizing uncountable '{word}' must be unchanged"
        );
        assert_eq!(
            singularize(word),
            word,
            "Singularizing uncountable '{word}' must be unchanged"
        );

        assert!(
            word.is_singular(),
            "'{word}' should be detected as singular"
        );
        assert!(word.is_plural(), "'{word}' should be detected as plural");
    }
}

#[test]
fn test_casing_preservation() {
    assert_eq!(pluralize("Person"), "People");
    assert_eq!(singularize("People"), "Person");

    assert_eq!(pluralize("PERSON"), "PEOPLE");
    assert_eq!(singularize("PEOPLE"), "PERSON");

    assert_eq!(pluralize("Category"), "Categories");
    assert_eq!(singularize("Categories"), "Category");

    assert_eq!(pluralize("CATEGORY"), "CATEGORIES");
    assert_eq!(singularize("CATEGORIES"), "CATEGORY");

    assert_eq!(pluralize("Knife"), "Knives");
    assert_eq!(singularize("Knives"), "Knife");

    assert_eq!(pluralize("KNIFE"), "KNIVES");
    assert_eq!(singularize("KNIVES"), "KNIFE");
}

#[test]
fn test_compound_identifiers() {
    assert_eq!(pluralize("user_account"), "user_accounts");
    assert_eq!(singularize("user_accounts"), "user_account");

    assert_eq!(pluralize("admin-user"), "admin-users");
    assert_eq!(singularize("admin-users"), "admin-user");

    assert_eq!(pluralize("userAccount"), "userAccounts");
    assert_eq!(singularize("userAccounts"), "userAccount");

    assert_eq!(pluralize("UserAccount"), "UserAccounts");
    assert_eq!(singularize("UserAccounts"), "UserAccount");
}

#[test]
fn test_pluralize_with_count() {
    assert_eq!(pluralize_with_count("user", 0), "users");
    assert_eq!(pluralize_with_count("user", 1), "user");
    assert_eq!(pluralize_with_count("user", 2), "users");
    assert_eq!(pluralize_with_count("users", 1), "user");

    assert_eq!(pluralize_with_count_inclusive("account", 1), "1 account");
    assert_eq!(pluralize_with_count_inclusive("account", 3), "3 accounts");
}

#[test]
fn test_custom_rule_engine() {
    let mut custom = Inflector::new();
    custom.add_irregular("criterion", "criteria");
    custom.add_uncountable("jargon");

    assert_eq!(custom.pluralize("jargon"), "jargon");
    assert_eq!(custom.singularize("jargon"), "jargon");

    #[cfg(feature = "regex")]
    {
        custom
            .add_plural_rule(r"([a-zA-Z0-9_]*)person$", "${1}peeps")
            .unwrap();
        assert_eq!(custom.pluralize("salesperson"), "salespeeps");
    }
}

#[test]
fn test_global_inflector_registration() {
    add_uncountable("equipmentx");
    assert_eq!(pluralize("equipmentx"), "equipmentx");

    add_irregular("criterionx", "criteriax");
    assert_eq!(pluralize("criterionx"), "criteriax");
    assert_eq!(singularize("criteriax"), "criterionx");

    #[cfg(feature = "regex")]
    {
        add_singular_rule(r"([a-zA-Z0-9_]*)peeps$", "${1}person").unwrap();
        assert_eq!(singularize("salespeeps"), "salesperson");
    }
}
