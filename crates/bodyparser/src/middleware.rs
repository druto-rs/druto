//! Asynchronous onion middleware for request body decoding and file upload ingestion.

use std::sync::Arc;

use bojha::{BodyParserConfig, PayloadLimits};
use dhap::{BoxFuture, Error as DhapError, Middleware, Next};
use router::{HeaderName, HttpContext, Response, RouterError};

/// Middleware automatically parsing incoming request bodies according to their Content-Type.
#[derive(Clone, Debug)]
pub struct BodyParserMiddleware {
    config: Arc<BodyParserConfig>,
}

impl BodyParserMiddleware {
    /// Constructs middleware with custom configuration.
    #[must_use]
    pub fn new(config: BodyParserConfig) -> Self {
        Self {
            config: Arc::new(config),
        }
    }

    /// Constructs middleware with default configuration.
    #[must_use]
    pub fn default_config() -> Self {
        Self::new(BodyParserConfig::default())
    }

    /// Constructs middleware with custom quota limits.
    #[must_use]
    pub fn with_limits(limits: PayloadLimits) -> Self {
        let config = BodyParserConfig::builder().limits(limits).build();
        Self::new(config)
    }

    /// Returns the configuration associated with this middleware.
    #[must_use]
    pub fn config(&self) -> &BodyParserConfig {
        &self.config
    }
}

impl Middleware<HttpContext, Response, RouterError> for BodyParserMiddleware {
    fn handle<'a>(
        &'a self,
        ctx: &'a mut HttpContext,
        next: Next<HttpContext, Response, RouterError>,
    ) -> BoxFuture<'a, Result<Response, DhapError<RouterError>>> {
        Box::pin(async move {
            // 1. Bypass if the HTTP method is not permitted for body parsing (e.g. GET, HEAD)
            if !self
                .config
                .is_method_allowed(ctx.request.intended_method().as_str())
            {
                return next.run(ctx).await;
            }

            // 2. Bypass if request body is empty
            if ctx.request.body().is_empty() {
                return next.run(ctx).await;
            }

            // 3. Inspect Content-Type header
            let content_type = match ctx.request.header(HeaderName::CONTENT_TYPE) {
                Some(ct) => ct.to_string(),
                None => return next.run(ctx).await,
            };

            // 4. Check if route is marked for manual streaming bypass
            if content_type.starts_with("multipart/form-data")
                && self.config.should_process_manually(ctx.request.path())
            {
                return next.run(ctx).await;
            }

            // 5. Decode according to Content-Type via bojha
            let parsed = bojha::parse(Some(&content_type), ctx.request.body(), &self.config)
                .map_err(|error| DhapError::Execution(RouterError::from(error)))?;

            match parsed {
                bojha::ParsedBody::Multipart { fields, files } => {
                    ctx.extensions.insert(fields);
                    ctx.extensions.insert(files);
                }
                bojha::ParsedBody::Form(fields) => {
                    ctx.extensions.insert(fields);
                }
                _ => {}
            }

            next.run(ctx).await
        })
    }
}
