//! Comprehensive test suite for the `matcher` crate.

use std::str::FromStr;

use matcher::{Constraint, Matcher, MatcherConfig, MatcherError, Method, MethodMatcher, Segment};
use proptest::prelude::*;

#[test]
fn test_basic_static_routing() {
    let mut matcher = Matcher::new();
    matcher.insert("/", 1).unwrap();
    matcher.insert("/users", 2).unwrap();
    matcher.insert("/api/v1/posts", 3).unwrap();

    let root_match = matcher.find("/").unwrap();
    assert_eq!(*root_match.payload, 1);
    assert_eq!(root_match.pattern, "/");
    assert!(root_match.params.is_empty());

    let users_match = matcher.find("/users").unwrap();
    assert_eq!(*users_match.payload, 2);

    let posts_match = matcher.find("/api/v1/posts").unwrap();
    assert_eq!(*posts_match.payload, 3);

    let not_found = matcher.find("/unknown");
    assert!(matches!(not_found, Err(MatcherError::NotFound { .. })));
}

#[test]
fn test_dual_parameter_syntax_colon_and_brace() {
    let mut matcher = Matcher::new();
    // Colon syntax
    matcher.insert("/users/:id", "colon").unwrap();
    // Brace syntax
    matcher
        .insert("/posts/{postId}/comments/{commentId}", "brace")
        .unwrap();

    let user = matcher.find("/users/42").unwrap();
    assert_eq!(*user.payload, "colon");
    assert_eq!(user.params.get("id"), Some("42"));
    assert_eq!(user.params.get_raw("id"), Some("42"));

    let comment = matcher.find("/posts/101/comments/202").unwrap();
    assert_eq!(*comment.payload, "brace");
    assert_eq!(comment.params.get("postId"), Some("101"));
    assert_eq!(comment.params.get("commentId"), Some("202"));
    assert_eq!(comment.params.len(), 2);
}

#[test]
fn test_optional_parameters_both_syntaxes() {
    let mut matcher = Matcher::new();
    matcher.insert("/catalog/:category/:item?", 10).unwrap();

    // With optional parameter present
    let with_opt = matcher.find("/catalog/books/rust-in-action").unwrap();
    assert_eq!(*with_opt.payload, 10);
    assert_eq!(with_opt.params.get("category"), Some("books"));
    assert_eq!(with_opt.params.get("item"), Some("rust-in-action"));

    // With optional parameter omitted
    let without_opt = matcher.find("/catalog/books").unwrap();
    assert_eq!(*without_opt.payload, 10);
    assert_eq!(without_opt.params.get("category"), Some("books"));
    assert_eq!(without_opt.params.get("item"), None);

    // Brace optional parameter
    let mut brace_matcher = Matcher::new();
    brace_matcher.insert("/profile/{section?}", 20).unwrap();

    let root_prof = brace_matcher.find("/profile").unwrap();
    assert_eq!(*root_prof.payload, 20);
    assert_eq!(root_prof.params.get("section"), None);

    let sub_prof = brace_matcher.find("/profile/security").unwrap();
    assert_eq!(*sub_prof.payload, 20);
    assert_eq!(sub_prof.params.get("section"), Some("security"));
}

#[test]
fn test_wildcard_catch_all_variations() {
    let mut matcher = Matcher::new();
    matcher.insert("/static/*filepath", "file").unwrap();
    matcher.insert("/catch-all/**", "catch_all").unwrap();
    matcher.insert("/docs/{slug*}", "docs").unwrap();

    let file_match = matcher.find("/static/css/themes/dark.css").unwrap();
    assert_eq!(*file_match.payload, "file");
    assert_eq!(
        file_match.params.get("filepath"),
        Some("css/themes/dark.css")
    );

    let catch_match = matcher.find("/catch-all/a/b/c/d").unwrap();
    assert_eq!(*catch_match.payload, "catch_all");
    assert_eq!(catch_match.params.get("path"), Some("a/b/c/d"));

    let doc_match = matcher.find("/docs/guide/v2/installation").unwrap();
    assert_eq!(*doc_match.payload, "docs");
    assert_eq!(doc_match.params.get("slug"), Some("guide/v2/installation"));
}

#[test]
fn test_builtin_constraints_num_uuid_slug_alpha_alphanumeric() {
    let mut matcher = Matcher::new();
    matcher.insert("/users/:id(num)", "num").unwrap();
    matcher.insert("/orders/:uuid(uuid)", "uuid").unwrap();
    matcher.insert("/posts/:slug(slug)", "slug").unwrap();
    matcher.insert("/categories/:name(alpha)", "alpha").unwrap();
    matcher
        .insert("/codes/:code(alphanumeric)", "alphanum")
        .unwrap();

    // Num
    assert!(matcher.find("/users/12345").is_ok());
    assert!(matcher.find("/users/abc").is_err());
    assert!(matcher.find("/users/123a").is_err());

    // Uuid
    assert!(
        matcher
            .find("/orders/550e8400-e29b-41d4-a716-446655440000")
            .is_ok()
    );
    assert!(matcher.find("/orders/not-a-uuid").is_err());
    assert!(matcher.find("/orders/550e8400-e29b-41d4-a716").is_err());

    // Slug
    assert!(matcher.find("/posts/my-first-post").is_ok());
    assert!(matcher.find("/posts/hello_world_123").is_ok());
    assert!(matcher.find("/posts/-invalid-start").is_err());
    assert!(matcher.find("/posts/invalid-end-").is_err());

    // Alpha
    assert!(matcher.find("/categories/technology").is_ok());
    assert!(matcher.find("/categories/tech123").is_err());

    // Alphanumeric
    assert!(matcher.find("/codes/PROMO2026").is_ok());
    assert!(matcher.find("/codes/PROMO-2026").is_err());
}

#[cfg(feature = "regex")]
#[test]
fn test_regex_constraints_inline_and_builder() {
    let mut matcher = Matcher::new();
    // Inline regex syntax
    matcher.insert("/items/:id(^[0-9]{4}$)", "inline").unwrap();

    // Builder programmatic regex syntax
    matcher
        .route("/versions/:ver")
        .where_regex("ver", r"^v\d+\.\d+$")
        .unwrap()
        .handler("builder")
        .unwrap();

    let item_ok = matcher.find("/items/1024").unwrap();
    assert_eq!(*item_ok.payload, "inline");
    assert!(matcher.find("/items/102").is_err());
    assert!(matcher.find("/items/10245").is_err());

    let ver_ok = matcher.find("/versions/v1.0").unwrap();
    assert_eq!(*ver_ok.payload, "builder");
    assert!(matcher.find("/versions/1.0").is_err());
}

#[test]
fn test_custom_predicate_constraint() {
    let mut matcher = Matcher::new();
    matcher
        .route("/even/:num")
        .where_custom("num", "even_number", |val| {
            val.parse::<i64>().map(|n| n % 2 == 0).unwrap_or(false)
        })
        .handler("even")
        .unwrap();

    assert!(matcher.find("/even/42").is_ok());
    assert!(matcher.find("/even/43").is_err());
    assert!(matcher.find("/even/abc").is_err());
}

#[test]
fn test_deterministic_precedence_insertion_order_independence() {
    // Priority order: Static > Regex/Constrained > Param > Wildcard
    // Test with Registration Order 1: Reverse specificity
    let mut matcher_rev = Matcher::new();
    matcher_rev.insert("/items/*all", "wildcard").unwrap();
    matcher_rev.insert("/items/:id", "param").unwrap();
    matcher_rev.insert("/items/:id(num)", "num").unwrap();
    matcher_rev.insert("/items/special", "static").unwrap();

    // Exact static must win over all dynamic and wildcard routes
    assert_eq!(
        *matcher_rev.find("/items/special").unwrap().payload,
        "static"
    );
    // Numeric constraint must win over unconstrained param
    assert_eq!(*matcher_rev.find("/items/999").unwrap().payload, "num");
    // Dynamic param must win over wildcard
    assert_eq!(*matcher_rev.find("/items/abc").unwrap().payload, "param");
    // Multi-segment must hit wildcard
    assert_eq!(
        *matcher_rev.find("/items/abc/xyz").unwrap().payload,
        "wildcard"
    );

    // Test with Registration Order 2: Normal specificity
    let mut matcher_norm = Matcher::new();
    matcher_norm.insert("/items/special", "static").unwrap();
    matcher_norm.insert("/items/:id(num)", "num").unwrap();
    matcher_norm.insert("/items/:id", "param").unwrap();
    matcher_norm.insert("/items/*all", "wildcard").unwrap();

    assert_eq!(
        *matcher_norm.find("/items/special").unwrap().payload,
        "static"
    );
    assert_eq!(*matcher_norm.find("/items/999").unwrap().payload, "num");
    assert_eq!(*matcher_norm.find("/items/abc").unwrap().payload, "param");
    assert_eq!(
        *matcher_norm.find("/items/abc/xyz").unwrap().payload,
        "wildcard"
    );
}

#[test]
fn test_conflicting_route_detection() {
    let mut matcher = Matcher::new();
    matcher.insert("/users/:id", 1).unwrap();

    // Colliding identical specificity at the same segment
    let res = matcher.insert("/users/:name", 2);
    assert!(matches!(res, Err(MatcherError::ConflictingRoute { .. })));

    // Duplicate exact static route
    let mut static_matcher = Matcher::new();
    static_matcher.insert("/api/v1", 1).unwrap();
    let dup = static_matcher.insert("/api/v1", 2);
    assert!(matches!(dup, Err(MatcherError::ConflictingRoute { .. })));
}

#[test]
fn test_invalid_pattern_syntax_errors() {
    let mut matcher = Matcher::new();

    // Wildcard not at the end
    let mid_wild = matcher.insert("/files/*all/details", 1);
    assert!(matches!(mid_wild, Err(MatcherError::InvalidPattern { .. })));

    // Empty param name
    let empty_param = matcher.insert("/users/:", 1);
    assert!(matches!(
        empty_param,
        Err(MatcherError::InvalidPattern { .. })
    ));

    // Duplicate param name
    let dup_param = matcher.insert("/orgs/:id/members/:id", 1);
    assert!(matches!(
        dup_param,
        Err(MatcherError::InvalidPattern { .. })
    ));

    // Required param following an optional param
    let req_after_opt = matcher.insert("/users/:id?/posts/:postId", 1);
    assert!(matches!(
        req_after_opt,
        Err(MatcherError::InvalidPattern { .. })
    ));

    // Static segment following an optional param
    let static_after_opt = matcher.insert("/users/:id?/profile", 1);
    assert!(matches!(
        static_after_opt,
        Err(MatcherError::InvalidPattern { .. })
    ));

    // Unclosed parenthesis in constraint
    let unclosed_paren = matcher.insert("/users/:id(num", 1);
    assert!(matches!(
        unclosed_paren,
        Err(MatcherError::InvalidPattern { .. })
    ));
}

#[test]
fn test_normalization_and_redundant_slashes() {
    let mut matcher = Matcher::new();
    matcher.insert("/api/users", 100).unwrap();

    // Redundant consecutive slashes are collapsed
    let matched = matcher.find("///api////users").unwrap();
    assert_eq!(*matched.payload, 100);
}

#[test]
fn test_flexible_and_strict_trailing_slash_configuration() {
    // Default flexible trailing slash mode
    let mut flexible = Matcher::new();
    flexible.insert("/hello", "greeting").unwrap();
    assert!(flexible.find("/hello").is_ok());
    assert!(flexible.find("/hello/").is_ok());

    // Strict trailing slash mode
    let strict_config = MatcherConfig {
        flexible_trailing_slash: false,
        case_sensitive: true,
    };
    let mut strict = Matcher::with_config(strict_config);
    strict.insert("/hello", "no_slash").unwrap();
    strict.insert("/world/", "with_slash").unwrap();

    assert!(strict.find("/hello").is_ok());
    assert!(strict.find("/hello/").is_err());
    assert!(strict.find("/world/").is_ok());
    assert!(strict.find("/world").is_err());
}

#[test]
fn test_case_sensitivity_configuration() {
    // Default: case-sensitive
    let mut case_sensitive = Matcher::new();
    case_sensitive.insert("/Dashboard", 1).unwrap();
    assert!(case_sensitive.find("/Dashboard").is_ok());
    assert!(case_sensitive.find("/dashboard").is_err());

    // Case-insensitive
    let ci_config = MatcherConfig {
        flexible_trailing_slash: true,
        case_sensitive: false,
    };
    let mut case_insensitive = Matcher::with_config(ci_config);
    case_insensitive.insert("/Dashboard", 1).unwrap();
    assert!(case_insensitive.find("/dashboard").is_ok());
    assert!(case_insensitive.find("/DASHBOARD").is_ok());
}

#[test]
fn test_unicode_path_support() {
    let mut matcher = Matcher::new();
    matcher.insert("/café/:item", "cafe").unwrap();
    matcher.insert("/日本語/:term", "japanese").unwrap();

    let cafe_match = matcher.find("/café/croissant").unwrap();
    assert_eq!(*cafe_match.payload, "cafe");
    assert_eq!(cafe_match.params.get("item"), Some("croissant"));

    let jp_match = matcher.find("/日本語/東京").unwrap();
    assert_eq!(*jp_match.payload, "japanese");
    assert_eq!(jp_match.params.get("term"), Some("東京"));
}

#[test]
fn test_percent_decoding_and_on_demand_zero_alloc() {
    let mut matcher = Matcher::new();
    matcher.insert("/search/:query", "search").unwrap();

    // Value without percent encoding borrows slice with zero allocation
    let plain = matcher.find("/search/rust").unwrap();
    let plain_decoded = plain.params.get_decoded("query").unwrap().unwrap();
    assert!(matches!(plain_decoded, std::borrow::Cow::Borrowed(_)));
    assert_eq!(plain_decoded, "rust");

    // Value with percent encoding decodes into owned string per RFC 3986 path rules (literal '+' is preserved)
    let encoded = matcher.find("/search/rust%20lang%2Bspeed+fast").unwrap();
    let decoded = encoded.params.get_decoded("query").unwrap().unwrap();
    assert!(matches!(decoded, std::borrow::Cow::Owned(_)));
    assert_eq!(decoded, "rust lang+speed+fast");

    // Query string percent decoding decodes '+' to space per application/x-www-form-urlencoded
    let query_match = matcher
        .find("/search/target?tag=rust+lang%20rocks")
        .unwrap();
    let qp = query_match.query_params().unwrap();
    let qp_decoded = qp.get_decoded("tag").unwrap().unwrap();
    assert_eq!(qp_decoded, "rust lang rocks");

    // Malformed percent encoding triggers error
    let malformed = matcher.find("/search/bad%2").unwrap();
    assert!(malformed.params.get_decoded("query").is_err());
}

#[test]
fn test_query_string_isolation_and_parameter_parsing() {
    let mut matcher = Matcher::new();
    matcher.insert("/items/:id", 1).unwrap();

    let uri = "/items/42?category=electronics&sort=desc&available";
    let matched = matcher.find(uri).unwrap();

    assert_eq!(*matched.payload, 1);
    assert_eq!(matched.path, "/items/42");
    assert_eq!(
        matched.raw_query,
        Some("category=electronics&sort=desc&available")
    );

    let query_params = matched.query_params().unwrap();
    assert_eq!(query_params.get("category"), Some("electronics"));
    assert_eq!(query_params.get("sort"), Some("desc"));
    assert_eq!(query_params.get("available"), Some(""));
    assert_eq!(query_params.get("missing"), None);
}

#[test]
fn test_reverse_routing_build_path() {
    let mut matcher = Matcher::new();
    matcher
        .insert_named("users.show", "/users/:id(num)", "show")
        .unwrap();
    matcher
        .insert_named("posts.filter", "/posts/:category/:tag?", "posts")
        .unwrap();
    matcher
        .insert_named("files.serve", "/files/*filepath", "files")
        .unwrap();

    // Valid path generation
    let user_path = matcher.build_path("users.show", &[("id", "123")]).unwrap();
    assert_eq!(user_path, "/users/123");

    // Optional parameter provided
    let posts_full = matcher
        .build_path("posts.filter", &[("category", "tech"), ("tag", "rust")])
        .unwrap();
    assert_eq!(posts_full, "/posts/tech/rust");

    // Optional parameter omitted
    let posts_sparse = matcher
        .build_path("posts.filter", &[("category", "tech")])
        .unwrap();
    assert_eq!(posts_sparse, "/posts/tech");

    // Wildcard path generation
    let file_path = matcher
        .build_path("files.serve", &[("filepath", "docs/spec.pdf")])
        .unwrap();
    assert_eq!(file_path, "/files/docs/spec.pdf");

    // Missing required parameter
    let missing_err = matcher.build_path("users.show", &[]);
    assert!(matches!(
        missing_err,
        Err(MatcherError::MissingParameter { .. })
    ));

    // Constraint failure in reverse routing
    let constraint_err = matcher.build_path("users.show", &[("id", "not-a-number")]);
    assert!(matches!(
        constraint_err,
        Err(MatcherError::ConstraintFailed { .. })
    ));

    // Non-existent route name
    let not_found_err = matcher.build_path("unknown.route", &[]);
    assert!(matches!(
        not_found_err,
        Err(MatcherError::RouteNotFound { .. })
    ));
}

#[test]
fn test_typed_parameter_parsing() {
    let mut matcher = Matcher::new();
    matcher.insert("/calc/:a/:b/:enabled", ()).unwrap();

    let matched = matcher.find("/calc/100/-25/true").unwrap();
    let a: u64 = matched.params.parse("a").unwrap();
    let b: i32 = matched.params.parse("b").unwrap();
    let enabled: bool = matched.params.parse("enabled").unwrap();

    assert_eq!(a, 100);
    assert_eq!(b, -25);
    assert!(enabled);

    // Parsing non-numeric into integer fails with ConstraintFailed
    let invalid = matcher.find("/calc/abc/1/false").unwrap();
    assert!(matches!(
        invalid.params.parse::<u64>("a"),
        Err(MatcherError::ConstraintFailed { .. })
    ));
}

#[test]
fn test_stack_parameter_capacity_and_heap_overflow() {
    let mut matcher = Matcher::new();
    let pattern = "/p/:a/:b/:c/:d/:e/:f/:g/:h/:i/:j/:k/:l/:m/:n/:o/:p/:q";
    matcher.insert(pattern, "overflow").unwrap();

    let uri = "/p/1/2/3/4/5/6/7/8/9/10/11/12/13/14/15/16/17";
    let matched = matcher.find(uri).unwrap();

    assert_eq!(matched.params.len(), 17);
    assert_eq!(matched.params.get("a"), Some("1"));
    assert_eq!(matched.params.get("p"), Some("16"));
    assert_eq!(matched.params.get("q"), Some("17"));
}

#[test]
fn test_method_matcher_all_verbs_and_405_detection() {
    let mut router = MethodMatcher::new();
    router.get("/users", "list_users").unwrap();
    router.post("/users", "create_user").unwrap();
    router.put("/users/:id", "update_user").unwrap();
    router.delete("/users/:id", "delete_user").unwrap();
    router.query("/search", "safe_query").unwrap();
    router.purge("/cache", "clear_cache").unwrap();
    router
        .insert(Method::Propfind, "/webdav/files", "webdav_propfind")
        .unwrap();

    // GET
    let get_res = router.find(&Method::Get, "/users").unwrap();
    assert_eq!(*get_res.payload, "list_users");
    assert_eq!(*get_res.matched_method(), Method::Get);

    // POST
    let post_res = router.find(&Method::Post, "/users").unwrap();
    assert_eq!(*post_res.payload, "create_user");

    // RFC 9110 HEAD fallback to GET
    let head_res = router.find(&Method::Head, "/users").unwrap();
    assert_eq!(*head_res.payload, "list_users");

    // QUERY
    let query_res = router.find(&Method::Query, "/search").unwrap();
    assert_eq!(*query_res.payload, "safe_query");

    // PURGE
    let purge_res = router.find(&Method::Purge, "/cache").unwrap();
    assert_eq!(*purge_res.payload, "clear_cache");

    // WebDAV
    let webdav_res = router.find(&Method::Propfind, "/webdav/files").unwrap();
    assert_eq!(*webdav_res.payload, "webdav_propfind");

    // 405 Method Not Allowed tracking
    let not_allowed = router.find(&Method::Delete, "/users");
    match not_allowed {
        Err(MatcherError::MethodNotAllowed {
            method,
            path,
            allowed,
        }) => {
            assert_eq!(method, Method::Delete);
            assert_eq!(path, "/users");
            assert!(allowed.contains(&Method::Get));
            assert!(allowed.contains(&Method::Post));
            assert!(allowed.contains(&Method::Head));
        }
        other => panic!("expected 405 MethodNotAllowed, got {other:?}"),
    }

    // Completely unknown route gives 404 NotFound
    let not_found = router.find(&Method::Get, "/nonexistent");
    assert!(matches!(not_found, Err(MatcherError::NotFound { .. })));
}

#[test]
fn test_method_matcher_wildcard_any_and_custom() {
    let mut router = MethodMatcher::new();
    router
        .insert_any("/proxy/*upstream", "proxy_handler")
        .unwrap();

    let custom_verb = Method::Custom("CUSTOM_SYNC".to_string());
    let res = router.find(&custom_verb, "/proxy/api/v2").unwrap();
    assert_eq!(*res.payload, "proxy_handler");
    assert_eq!(*res.matched_method(), Method::Any);
}

#[test]
fn test_method_from_str_and_bytes_parsing() {
    assert_eq!(Method::from_bytes(b"get"), Method::Get);
    assert_eq!(Method::from_bytes(b"POST"), Method::Post);
    assert_eq!(Method::from_bytes(b"query"), Method::Query);
    assert_eq!(Method::from_bytes(b"purge"), Method::Purge);
    assert_eq!(Method::from_bytes(b"propfind"), Method::Propfind);
    assert_eq!(Method::from_bytes(b"lock"), Method::Lock);
    assert_eq!(Method::from_bytes(b"report"), Method::Report);
    assert_eq!(Method::from_bytes(b"mkcalendar"), Method::Mkcalendar);
    assert_eq!(Method::from_bytes(b"search"), Method::Search);
    assert_eq!(Method::from_bytes(b"acl"), Method::Acl);
    assert_eq!(Method::from_bytes(b"ban"), Method::Ban);
    assert_eq!(Method::from_bytes(b"*"), Method::Any);
    assert_eq!(
        Method::from_str("CUSTOM_VERB").unwrap(),
        Method::Custom("CUSTOM_VERB".to_string())
    );
}

#[test]
fn test_segment_and_constraint_derived_traits_and_display() {
    let seg_static = Segment::static_segment("users");
    assert_eq!(seg_static.to_string(), "users");
    assert_eq!(seg_static.specificity_rank(), 4);

    let seg_param = Segment::param("id");
    assert_eq!(seg_param.to_string(), ":id");
    assert_eq!(seg_param.specificity_rank(), 2);

    let seg_opt = Segment::optional_param("id");
    assert_eq!(seg_opt.to_string(), ":id?");

    let seg_constrained = Segment::constrained_param("id", false, Constraint::num());
    assert_eq!(seg_constrained.to_string(), ":id(num)");
    assert_eq!(seg_constrained.specificity_rank(), 3);

    let seg_wildcard = Segment::wildcard("rest");
    assert_eq!(seg_wildcard.to_string(), "*rest");
    assert_eq!(seg_wildcard.specificity_rank(), 1);

    assert!(seg_opt.is_optional());
    assert!(!seg_static.is_optional());

    let num_c = Constraint::num();
    assert_eq!(num_c.precedence_rank(), 40);
    assert_eq!(num_c.description(), "num");

    let alpha_c = Constraint::alpha();
    assert_eq!(alpha_c.precedence_rank(), 30);
    assert_eq!(alpha_c.description(), "alpha");

    let alphanum_c = Constraint::alphanumeric();
    assert_eq!(alphanum_c.precedence_rank(), 20);
    assert_eq!(alphanum_c.description(), "alphanumeric");

    let slug_c = Constraint::slug();
    assert_eq!(slug_c.precedence_rank(), 10);
    assert_eq!(slug_c.description(), "slug");

    let uuid_c = Constraint::uuid();
    assert_eq!(uuid_c.precedence_rank(), 50);
    assert_eq!(uuid_c.description(), "uuid");
}

#[test]
fn test_error_display_formatting() {
    let conflict = MatcherError::ConflictingRoute {
        existing: "/users/:id".to_string(),
        new_pattern: "/users/:name".to_string(),
        reason: "ambiguous specificity".to_string(),
    };
    assert!(conflict.to_string().contains("route conflict"));

    let invalid = MatcherError::InvalidPattern {
        pattern: "/:".to_string(),
        reason: "empty parameter name".to_string(),
    };
    assert!(invalid.to_string().contains("invalid route pattern"));

    let constraint_err = MatcherError::ConstraintFailed {
        param: "id".to_string(),
        value: "abc".to_string(),
        constraint: "num".to_string(),
    };
    assert!(
        constraint_err
            .to_string()
            .contains("constraint `num` failed")
    );

    let missing = MatcherError::MissingParameter {
        name: "id".to_string(),
        route: "/users/:id".to_string(),
    };
    assert!(missing.to_string().contains("missing required parameter"));

    let not_found = MatcherError::NotFound {
        path: "/404".to_string(),
    };
    assert!(not_found.to_string().contains("no route matched path"));

    let decode_err = MatcherError::DecodeError {
        param: "tag".to_string(),
        value: "%zz".to_string(),
    };
    assert!(decode_err.to_string().contains("failed to percent-decode"));

    let method_err = MatcherError::MethodNotAllowed {
        method: Method::Post,
        path: "/users".to_string(),
        allowed: vec![Method::Get, Method::Head],
    };
    assert!(
        method_err
            .to_string()
            .contains("method `POST` not allowed for path `/users`")
    );
}

#[test]
fn test_all_http_methods_comprehensive() {
    let all_methods = [
        (Method::Get, "GET"),
        (Method::Post, "POST"),
        (Method::Put, "PUT"),
        (Method::Delete, "DELETE"),
        (Method::Patch, "PATCH"),
        (Method::Head, "HEAD"),
        (Method::Options, "OPTIONS"),
        (Method::Connect, "CONNECT"),
        (Method::Trace, "TRACE"),
        (Method::Query, "QUERY"),
        (Method::Purge, "PURGE"),
        (Method::Propfind, "PROPFIND"),
        (Method::Proppatch, "PROPPATCH"),
        (Method::Mkcol, "MKCOL"),
        (Method::Copy, "COPY"),
        (Method::Move, "MOVE"),
        (Method::Lock, "LOCK"),
        (Method::Unlock, "UNLOCK"),
        (Method::Report, "REPORT"),
        (Method::Mkcalendar, "MKCALENDAR"),
        (Method::Search, "SEARCH"),
        (Method::Acl, "ACL"),
        (Method::Ban, "BAN"),
        (Method::Any, "*"),
        (Method::Custom("CUSTOM_VERB".to_string()), "CUSTOM_VERB"),
    ];

    for (method, expected_str) in all_methods {
        assert_eq!(method.as_str(), expected_str);
        assert_eq!(method.to_string(), expected_str);

        let parsed = Method::from_bytes(expected_str.as_bytes());
        assert_eq!(parsed, method);

        let from_str_parsed = Method::from_str(expected_str).unwrap();
        assert_eq!(from_str_parsed, method);

        assert!(method.matches(&method));
        assert!(Method::Any.matches(&method));
        assert!(method.matches(&Method::Any));
    }

    assert!(!Method::Get.matches(&Method::Post));
}

#[test]
fn test_method_matcher_convenience_methods_and_config() {
    let mut default_router = MethodMatcher::default();
    default_router.put("/resource/:id", "put").unwrap();
    default_router.delete("/resource/:id", "delete").unwrap();
    default_router.patch("/resource/:id", "patch").unwrap();
    default_router.head("/resource/:id", "head").unwrap();
    default_router.options("/resource/:id", "options").unwrap();
    default_router
        .match_methods(&[Method::Mkcol, Method::Copy], "/folders", "webdav")
        .unwrap();

    let config = MatcherConfig {
        flexible_trailing_slash: false,
        case_sensitive: false,
    };
    let mut configured_router = MethodMatcher::with_config(config);
    configured_router.get("/config-test", "ok").unwrap();
    assert!(configured_router.find(&Method::Get, "/CONFIG-TEST").is_ok());

    assert_eq!(
        *default_router
            .find(&Method::Put, "/resource/1")
            .unwrap()
            .payload,
        "put"
    );
    assert_eq!(
        *default_router
            .find(&Method::Delete, "/resource/1")
            .unwrap()
            .payload,
        "delete"
    );
    assert_eq!(
        *default_router
            .find(&Method::Patch, "/resource/1")
            .unwrap()
            .payload,
        "patch"
    );
    assert_eq!(
        *default_router
            .find(&Method::Head, "/resource/1")
            .unwrap()
            .payload,
        "head"
    );
    assert_eq!(
        *default_router
            .find(&Method::Options, "/resource/1")
            .unwrap()
            .payload,
        "options"
    );
    assert_eq!(
        *default_router
            .find(&Method::Mkcol, "/folders")
            .unwrap()
            .payload,
        "webdav"
    );
    assert_eq!(
        *default_router
            .find(&Method::Copy, "/folders")
            .unwrap()
            .payload,
        "webdav"
    );

    default_router.report("/calendars/:id", "report").unwrap();
    default_router.search("/search/:id", "search").unwrap();
    default_router
        .mkcalendar("/calendars/new", "mkcal")
        .unwrap();
    default_router.acl("/acls/:id", "acl").unwrap();
    default_router.ban("/cache/:pattern", "ban").unwrap();

    assert_eq!(
        *default_router
            .find(&Method::Report, "/calendars/1")
            .unwrap()
            .payload,
        "report"
    );
    assert_eq!(
        *default_router
            .find(&Method::Search, "/search/1")
            .unwrap()
            .payload,
        "search"
    );
    assert_eq!(
        *default_router
            .find(&Method::Mkcalendar, "/calendars/new")
            .unwrap()
            .payload,
        "mkcal"
    );
    assert_eq!(
        *default_router
            .find(&Method::Acl, "/acls/1")
            .unwrap()
            .payload,
        "acl"
    );
    assert_eq!(
        *default_router
            .find(&Method::Ban, "/cache/all")
            .unwrap()
            .payload,
        "ban"
    );
}

#[test]
fn test_method_safety_and_idempotence() {
    let safe_methods = [
        Method::Get,
        Method::Head,
        Method::Options,
        Method::Trace,
        Method::Query,
        Method::Propfind,
        Method::Report,
        Method::Search,
    ];
    for m in &safe_methods {
        assert!(m.is_safe());
        assert!(m.is_idempotent());
    }

    let idempotent_only = [
        Method::Put,
        Method::Delete,
        Method::Mkcol,
        Method::Unlock,
        Method::Acl,
        Method::Purge,
        Method::Ban,
    ];
    for m in &idempotent_only {
        assert!(!m.is_safe());
        assert!(m.is_idempotent());
    }

    let non_idempotent = [
        Method::Post,
        Method::Patch,
        Method::Connect,
        Method::Proppatch,
        Method::Copy,
        Method::Move,
        Method::Lock,
        Method::Mkcalendar,
        Method::Any,
        Method::Custom("CUSTOM".to_string()),
    ];
    for m in &non_idempotent {
        assert!(!m.is_safe());
        assert!(!m.is_idempotent());
    }
}

#[test]
fn test_route_builder_all_constraint_shortcuts() {
    let mut matcher = Matcher::default();
    matcher
        .route("/users/:id")
        .name("users.id")
        .where_num("id")
        .handler(1)
        .unwrap();

    matcher
        .route("/docs/:uuid")
        .where_uuid("uuid")
        .handler(2)
        .unwrap();

    matcher
        .route("/blogs/:slug")
        .where_slug("slug")
        .handler(3)
        .unwrap();

    matcher
        .route("/alpha/:code")
        .where_alpha("code")
        .handler(4)
        .unwrap();

    matcher
        .route("/alphanumeric/:code")
        .where_alphanumeric("code")
        .handler(5)
        .unwrap();

    assert!(matcher.find("/users/123").is_ok());
    assert!(matcher.find("/users/abc").is_err());
    assert!(
        matcher
            .find("/docs/550e8400-e29b-41d4-a716-446655440000")
            .is_ok()
    );
    assert!(matcher.find("/blogs/good-slug_name").is_ok());
    assert!(matcher.find("/alpha/ABC").is_ok());
    assert!(matcher.find("/alphanumeric/ABC123").is_ok());
    assert!(matcher.config().flexible_trailing_slash);
}

#[test]
fn test_params_methods_and_traits() {
    use matcher::Params;

    let mut params = Params::default();
    assert!(params.is_empty());
    assert_eq!(params.len(), 0);

    params.push("k1", "v1");
    params.push("k2", "v2");
    assert_eq!(params.len(), 2);
    assert!(!params.is_empty());

    let mut other = Params::new();
    other.push("k1", "v1");
    other.push("k2", "v2");
    assert_eq!(params, other);

    let debug_repr = format!("{params:?}");
    assert!(debug_repr.contains("k1"));

    params.truncate(1);
    assert_eq!(params.len(), 1);
    assert_eq!(params.get("k2"), None);

    params.clear();
    assert!(params.is_empty());

    // Test query params methods
    let uri = "/items?name=test%20query&empty=&malformed=%zz";
    let mut test_matcher = Matcher::new();
    test_matcher.insert("/items", ()).unwrap();
    let m = test_matcher.find(uri).unwrap();
    let qp = m.query_params().unwrap();

    assert_eq!(qp.raw(), "name=test%20query&empty=&malformed=%zz");
    assert_eq!(qp.get("name"), Some("test%20query"));
    assert_eq!(qp.get_decoded("name").unwrap().unwrap(), "test query");
    assert_eq!(qp.get("empty"), Some(""));
    assert!(qp.get_decoded("malformed").is_err());
    assert_eq!(qp.get_decoded("absent").unwrap(), None);
}

#[test]
fn test_parser_additional_syntax_branches() {
    use matcher::RouteParser;

    // Empty route strings
    assert!(RouteParser::parse("").unwrap().is_empty());
    assert!(RouteParser::parse("/").unwrap().is_empty());

    // Empty braces
    let empty_braces = RouteParser::parse("/users/{}");
    assert!(matches!(
        empty_braces,
        Err(MatcherError::InvalidPattern { .. })
    ));

    // Unclosed braces
    let unclosed = RouteParser::parse("/users/{id");
    assert!(matches!(unclosed, Err(MatcherError::InvalidPattern { .. })));

    // Empty name before `:` in brace parameter
    let empty_name_brace = RouteParser::parse("/users/{:num}");
    assert!(matches!(
        empty_name_brace,
        Err(MatcherError::InvalidPattern { .. })
    ));

    // Missing param name before constraint in colon parameter
    let missing_name_colon = RouteParser::parse("/users/:(num)");
    assert!(matches!(
        missing_name_colon,
        Err(MatcherError::InvalidPattern { .. })
    ));
}

#[test]
fn test_segment_and_constraint_equality_and_debug() {
    let seg1 = Segment::static_segment("users");
    let seg2 = Segment::static_segment("users");
    let seg3 = Segment::static_segment("posts");
    let seg_p1 = Segment::param("id");
    let seg_p2 = Segment::param("slug");
    let seg_c1 = Segment::constrained_param("id", false, Constraint::num());
    let seg_c2 = Segment::constrained_param("id", false, Constraint::num());
    let seg_w1 = Segment::wildcard("path");
    let seg_w2 = Segment::wildcard("rest");

    assert!(seg1.has_identical_specificity(&seg2));
    assert!(!seg1.has_identical_specificity(&seg3));
    assert!(seg_p1.has_identical_specificity(&seg_p2));
    assert!(seg_c1.has_identical_specificity(&seg_c2));
    assert!(seg_w1.has_identical_specificity(&seg_w2));
    assert!(!seg1.has_identical_specificity(&seg_p1));

    let custom_c = Constraint::custom("is_odd", |v| v.len() % 2 == 1);
    assert_eq!(custom_c.precedence_rank(), 1);
    assert_eq!(custom_c.description(), "is_odd");
    assert_eq!(format!("{custom_c:?}"), "Constraint(\"is_odd\")");

    let custom_c2 = Constraint::custom("is_odd", |v| v.len() % 2 == 1);
    assert_eq!(custom_c, custom_c2);

    let num_c = Constraint::num();
    let num_c2 = Constraint::num();
    assert_eq!(num_c, num_c2);
    assert_ne!(num_c, custom_c);

    // BuiltinConstraint empty string check
    use matcher::BuiltinConstraint;
    assert!(!BuiltinConstraint::Num.matches(""));
    assert!(!BuiltinConstraint::Alpha.matches(""));
    assert!(!BuiltinConstraint::Alphanumeric.matches(""));
    assert!(!BuiltinConstraint::Uuid.matches(""));
    assert!(!BuiltinConstraint::Slug.matches(""));
}

#[test]
fn test_matcher_root_build_path_and_empty_wildcard() {
    let mut matcher = Matcher::new();
    matcher.insert_named("root", "/", "home").unwrap();
    assert_eq!(matcher.build_path("root", &[]).unwrap(), "/");

    let mut wild_matcher = Matcher::new();
    wild_matcher.insert("/assets/*path", "assets").unwrap();
    let res = wild_matcher.find("/assets/").unwrap();
    assert_eq!(*res.payload, "assets");
    assert_eq!(res.params.get("path"), Some(""));
}

#[test]
fn test_comprehensive_branch_coverage_and_edge_cases() {
    // 1. UUID invalid character
    assert!(
        !matcher::constraint::BuiltinConstraint::Uuid
            .matches("550e8400-e29b-41d4-a716-44665544000z")
    );
    assert!(
        !matcher::constraint::BuiltinConstraint::Uuid
            .matches("550e8400-e29b-41d4-a716-446655440000-extra")
    );

    // 2. Slug starting/ending with underscore
    assert!(!matcher::constraint::BuiltinConstraint::Slug.matches("_leading"));
    assert!(!matcher::constraint::BuiltinConstraint::Slug.matches("trailing_"));

    // 3. Constraint PartialEq and Precedence
    let custom1 = Constraint::custom("rule_a", |_| true);
    let custom2 = Constraint::custom("rule_b", |_| true);
    assert_ne!(custom1, custom2);
    assert_eq!(custom1.precedence_rank(), 1);

    #[cfg(feature = "regex")]
    {
        let reg1 = Constraint::regex("^[0-9]+$").unwrap();
        let reg2 = Constraint::regex("^[a-z]+$").unwrap();
        assert_ne!(reg1, reg2);
        assert_eq!(reg1.precedence_rank(), 100);
    }

    // 4. MissingParameter for Wildcard in build_path
    let mut matcher = Matcher::new();
    matcher.insert_named("wild", "/docs/*path", ()).unwrap();
    let err = matcher.build_path("wild", &[]);
    assert!(matches!(err, Err(MatcherError::MissingParameter { .. })));

    // 5. Params heap overflow push, truncate, and PartialEq
    let mut params1 = matcher::Params::new();
    let mut params2 = matcher::Params::new();
    for i in 0..20 {
        let key: &'static str = Box::leak(format!("k{i}").into_boxed_str());
        let val: &'static str = Box::leak(format!("v{i}").into_boxed_str());
        params1.push(key, val);
        params2.push(key, val);
    }
    assert_eq!(params1, params2);
    assert_eq!(params1.len(), 20);

    // Mismatched length and mismatched values
    let mut params3 = matcher::Params::new();
    params3.push("k0", "v0");
    assert_ne!(params1, params3);
    params3.clear();
    params3.push("k0", "wrong");
    let mut single = matcher::Params::new();
    single.push("k0", "v0");
    assert_ne!(single, params3);

    // Truncate heap params
    params1.truncate(5);
    assert_eq!(params1.len(), 5);

    // Parse missing parameter error
    assert!(matches!(
        params1.parse::<u32>("missing"),
        Err(MatcherError::MissingParameter { .. })
    ));

    // Percent decode errors: truncated hex and invalid UTF-8
    assert!(matcher::params::percent_decode("%", "test").is_err());
    assert!(matcher::params::percent_decode("%2", "test").is_err());
    assert!(matcher::params::percent_decode("%ZZ", "test").is_err());
    assert!(matcher::params::percent_decode("%FF%FE", "test").is_err());

    // 6. Tree collisions: duplicate constrained params, duplicate unconstrained params, duplicate wildcards
    let mut tree_matcher = Matcher::new();
    tree_matcher.insert("/users/:id(num)", 1).unwrap();
    let col_num = tree_matcher.insert("/users/:code(num)", 2);
    assert!(matches!(
        col_num,
        Err(MatcherError::ConflictingRoute { .. })
    ));

    tree_matcher.insert("/files/*all", 1).unwrap();
    let col_wild = tree_matcher.insert("/files/*rest", 2);
    assert!(matches!(
        col_wild,
        Err(MatcherError::ConflictingRoute { .. })
    ));

    // 7. RouteNotFound Display
    let not_found_err = MatcherError::RouteNotFound {
        name: "admin.dashboard".to_string(),
    };
    assert!(
        not_found_err
            .to_string()
            .contains("named route `admin.dashboard` not found")
    );

    // 8. Parser unclosed brace and invalid constraint string
    let bad_constraint = matcher::RouteParser::parse("/items/:id([a-z)");
    assert!(bad_constraint.is_err());

    // 9. Backtracking in tree lookup
    let mut backtrack_matcher = Matcher::new();
    backtrack_matcher
        .insert("/api/:version(num)/users", "v_num")
        .unwrap();
    backtrack_matcher
        .insert("/api/:version/users", "v_any")
        .unwrap();
    let res = backtrack_matcher.find("/api/beta/users").unwrap();
    assert_eq!(*res.payload, "v_any");
    assert_eq!(res.params.get("version"), Some("beta"));

    // 10. Additional corner-case branches
    // UUID with non-hyphen at delimiter indices
    assert!(
        !matcher::constraint::BuiltinConstraint::Uuid
            .matches("012345678901234567890123456789012345")
    );

    // Lowercase hex character in percent_decode
    let hex_lower = matcher::params::percent_decode("%2f", "test").unwrap();
    assert_eq!(hex_lower, "/");

    // Required param after optional param
    let req_after_opt_direct = matcher::RouteParser::parse("/users/:id?/details");
    assert!(req_after_opt_direct.is_err());

    // Anonymous wildcard "*"
    let anon_wild = matcher::RouteParser::parse("/files/*").unwrap();
    assert_eq!(anon_wild.last().unwrap().to_string(), "*wildcard");

    // Brace parameter with constraint
    let brace_constrained = matcher::RouteParser::parse("/users/{id:num}").unwrap();
    assert_eq!(brace_constrained.last().unwrap().to_string(), ":id(num)");

    // Multiple constrained children with different specificity ranks
    let mut multi_constrained = Matcher::new();
    multi_constrained.insert("/items/:id(num)", 1).unwrap();
    #[cfg(feature = "regex")]
    {
        multi_constrained
            .insert("/items/:id(^[0-9]{3}$)", 2)
            .unwrap();
    }

    // Constrained and param backtracking
    let mut backtrack_all = Matcher::new();
    backtrack_all.insert("/catalog/:id(num)/edit", 1).unwrap();
    backtrack_all.insert("/catalog/:slug/view", 2).unwrap();
    assert!(backtrack_all.find("/catalog/123/missing").is_err());
    assert!(backtrack_all.find("/catalog/books/missing").is_err());

    // Missing param get_decoded
    assert_eq!(res.params.get_decoded("missing_param").unwrap(), None);

    // Non-hex character in percent_decode
    assert!(matcher::params::percent_decode("%g1", "test").is_err());

    // RouteNode default directly
    let default_node = matcher::tree::RouteNode::<()>::default();
    assert!(
        default_node
            .lookup("/test", &["test"], &mut matcher::Params::new(), true)
            .is_none()
    );

    // Regex constraint description
    #[cfg(feature = "regex")]
    {
        let reg = Constraint::regex("^[0-9]+$").unwrap();
        assert!(reg.description().contains("regex"));
    }

    // Build path with single optional parameter omitted
    let mut opt_build_matcher = Matcher::new();
    opt_build_matcher
        .insert_named("opt_single", "/:id?", ())
        .unwrap();
    assert_eq!(
        opt_build_matcher.build_path("opt_single", &[]).unwrap(),
        "/"
    );

    // Parser colon optional with empty name
    let empty_colon_opt = matcher::RouteParser::parse("/users/:?");
    assert!(empty_colon_opt.is_err());

    // Parser regex with surrounding slashes
    #[cfg(feature = "regex")]
    {
        let slash_regex = matcher::RouteParser::parse("/items/:id(/^[0-9]+$/)").unwrap();
        assert_eq!(slash_regex.len(), 2);
    }

    // Param backtracking when downstream fails
    let mut fail_downstream = Matcher::new();
    fail_downstream
        .insert("/catalog/:cat/items", "items")
        .unwrap();
    assert!(fail_downstream.find("/catalog/books/unknown").is_err());

    // Duplicate unconstrained param collision
    let mut dup_param_matcher = Matcher::new();
    dup_param_matcher.insert("/users/:id", 1).unwrap();
    let dup_param_err = dup_param_matcher.insert("/users/:name", 2);
    assert!(matches!(
        dup_param_err,
        Err(MatcherError::ConflictingRoute { .. })
    ));
}

#[test]
fn test_regex_full_segment_anchoring() {
    let mut matcher = Matcher::new();
    // Register unanchored regex pattern
    matcher.insert("/users/{id:[0-9]+}", "numeric").unwrap();

    // Pure numeric matches
    let ok = matcher.find("/users/12345").unwrap();
    assert_eq!(*ok.payload, "numeric");
    assert_eq!(ok.params.get("id"), Some("12345"));

    // Strings with numbers embedded must NOT match the numeric route
    assert!(matcher.find("/users/abc123def").is_err());
    assert!(matcher.find("/users/123abc").is_err());
    assert!(matcher.find("/users/abc123").is_err());
}

#[test]
fn test_distinct_parameter_names_on_shared_dynamic_prefix() {
    let mut matcher = Matcher::new();
    // Route 1 uses `postId`
    matcher
        .insert("/posts/:postId/comments", "comments")
        .unwrap();
    // Route 2 uses `id` on the shared dynamic parent segment
    matcher.insert("/posts/:id/author", "author").unwrap();

    let comments_match = matcher.find("/posts/42/comments").unwrap();
    assert_eq!(*comments_match.payload, "comments");
    assert_eq!(comments_match.params.get("postId"), Some("42"));
    assert_eq!(comments_match.params.get("id"), None);

    let author_match = matcher.find("/posts/42/author").unwrap();
    assert_eq!(*author_match.payload, "author");
    assert_eq!(author_match.params.get("id"), Some("42"));
    assert_eq!(author_match.params.get("postId"), None);
}

#[test]
fn test_builtin_constraint_precedence_independence() {
    // Slug route registered first, UUID route registered second:
    // Uuid (50) must take precedence over Slug (10)
    let mut matcher = Matcher::new();
    matcher.insert("/items/:id(slug)", "slug").unwrap();
    matcher.insert("/items/:id(uuid)", "uuid").unwrap();
    matcher.insert("/items/:id(num)", "num").unwrap();

    let uuid_val = "123e4567-e89b-12d3-a456-426614174000";
    let req_uuid = format!("/items/{uuid_val}");
    let uuid_match = matcher.find(&req_uuid).unwrap();
    assert_eq!(*uuid_match.payload, "uuid");
    assert_eq!(uuid_match.params.get("id"), Some(uuid_val));

    let num_match = matcher.find("/items/98765").unwrap();
    assert_eq!(*num_match.payload, "num");
    assert_eq!(num_match.params.get("id"), Some("98765"));

    let slug_match = matcher.find("/items/my-cool-post").unwrap();
    assert_eq!(*slug_match.payload, "slug");
    assert_eq!(slug_match.params.get("id"), Some("my-cool-post"));
}

#[test]
fn test_method_matcher_insert_named_and_build_path() {
    let mut router = MethodMatcher::new();
    router
        .insert_named("user.show", Method::Get, "/users/:id", "show_user")
        .unwrap();

    let path = router.build_path("user.show", &[("id", "42")]).unwrap();
    assert_eq!(path, "/users/42");

    let missing = router.build_path("nonexistent", &[]);
    assert!(matches!(missing, Err(MatcherError::RouteNotFound { .. })));
}

#[test]
fn test_method_trait_conversions() {
    let m = Method::Get;
    assert_eq!(m.as_ref(), "GET");

    let from_str_ref = Method::from("post");
    assert_eq!(from_str_ref, Method::Post);

    let from_string = Method::from("delete".to_string());
    assert_eq!(from_string, Method::Delete);
}

proptest! {
    #![proptest_config(proptest::test_runner::Config {
        failure_persistence: None,
        ..proptest::test_runner::Config::default()
    })]

    #[test]
    fn test_property_random_static_paths(segments in proptest::collection::vec("[a-z0-9]{1,10}", 1..6)) {
        let path = format!("/{}", segments.join("/"));
        let mut matcher = Matcher::new();
        matcher.insert(&path, 42).unwrap();

        let matched = matcher.find(&path).unwrap();
        prop_assert_eq!(*matched.payload, 42);
        prop_assert_eq!(matched.path, path.as_str());
    }

    #[test]
    fn test_property_random_parameter_values(val in "[a-zA-Z0-9_-]{1,20}") {
        let mut matcher = Matcher::new();
        matcher.insert("/items/:val", 99).unwrap();

        let req_path = format!("/items/{val}");
        let matched = matcher.find(&req_path).unwrap();
        prop_assert_eq!(*matched.payload, 99);
        prop_assert_eq!(matched.params.get("val"), Some(val.as_str()));
    }
}
