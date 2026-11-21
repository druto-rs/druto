//! Tests for asynchronous rules, async refinements, and async validation workflows.

#[cfg(feature = "async")]
mod async_tests {
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };

    use serde_json::json;
    use validator::prelude::*;

    #[tokio::test]
    async fn test_async_string_refine() {
        let call_count = Arc::new(AtomicUsize::new(0));
        let count_clone = Arc::clone(&call_count);

        let email_schema = string().email().refine_async(move |val| {
            let counter = Arc::clone(&count_clone);
            let email = val.to_string();
            Box::pin(async move {
                counter.fetch_add(1, Ordering::SeqCst);
                // Simulate async database uniqueness check
                if email == "taken@example.com" {
                    Err("Email is already registered.".to_string())
                } else {
                    Ok(())
                }
            })
        });

        // Unique email passes
        let result = email_schema
            .validate_async(&json!("fresh@example.com"))
            .await;
        assert_eq!(result.unwrap(), "fresh@example.com");
        assert_eq!(call_count.load(Ordering::SeqCst), 1);

        // Taken email fails
        let err = email_schema
            .validate_async(&json!("taken@example.com"))
            .await
            .unwrap_err();
        assert_eq!(err.issues()[0].rule, "refine_async");
        assert_eq!(err.issues()[0].message, "Email is already registered.");
    }

    #[tokio::test]
    async fn test_async_custom_rule_implementation() {
        struct AsyncBlacklistRule {
            blacklisted_domain: String,
        }

        impl AsyncCustomRule<String> for AsyncBlacklistRule {
            fn validate<'a>(
                &'a self,
                value: &'a String,
                ctx: &'a RuleContext<'a>,
            ) -> futures_util::future::BoxFuture<'a, Result<(), ValidationIssue>> {
                Box::pin(async move {
                    if value.ends_with(&self.blacklisted_domain) {
                        let mut params = RuleParams::new();
                        params.insert("domain", &self.blacklisted_domain);
                        Err(ctx.create_issue(
                            "blacklisted_domain",
                            params,
                            Some("Domain is forbidden."),
                        ))
                    } else {
                        Ok(())
                    }
                })
            }
        }

        let s = string().rule_async(AsyncBlacklistRule {
            blacklisted_domain: "spam.com".to_string(),
        });

        assert!(s.validate_async(&json!("user@clean.org")).await.is_ok());
        let err = s.validate_async(&json!("bot@spam.com")).await.unwrap_err();
        assert_eq!(err.issues()[0].rule, "blacklisted_domain");
        assert_eq!(err.issues()[0].message, "Domain is forbidden.");
    }

    #[tokio::test]
    async fn test_async_object_and_request_validation() {
        let req_schema = request().body(
            object()
                .field(
                    "username",
                    string().refine_async(|name| {
                        let name_str = name.to_string();
                        Box::pin(async move {
                            if name_str == "root" || name_str == "admin" {
                                Err("Reserved username.".to_string())
                            } else {
                                Ok(())
                            }
                        })
                    }),
                )
                .field("email", string().email()),
        );

        let parts_ok = RequestParts {
            params: json!({}),
            query: json!({}),
            headers: json!({}),
            cookies: json!({}),
            body: json!({
                "username": "druto_engineer",
                "email": "engineer@druto.dev"
            }),
        };

        let res = req_schema.validate_async(&parts_ok).await;
        assert!(res.is_ok());

        let parts_reserved = RequestParts {
            params: json!({}),
            query: json!({}),
            headers: json!({}),
            cookies: json!({}),
            body: json!({
                "username": "admin",
                "email": "admin@druto.dev"
            }),
        };

        let err = req_schema
            .validate_async(&parts_reserved)
            .await
            .unwrap_err();
        assert_eq!(err.issues()[0].rule, "refine_async");
        assert_eq!(err.issues()[0].message, "Reserved username.");
    }
}
