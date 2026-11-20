//! Asynchronous onion middleware for request body decoding and file upload ingestion.

use std::sync::Arc;

use khonj::encoding::decode_component;
use pipeline::{BoxFuture, Middleware, Next, PipelineError};
use router::{HeaderName, HttpContext, Response, RouterError};

use crate::{
    config::BodyParserConfig,
    form_data::{FormFields, UploadedFiles},
    limits::PayloadLimits,
    multipart::{MultipartParser, extract_boundary},
    parsers::{form::parse_form_value, json::parse_json_value, text::parse_text},
};

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
    ) -> BoxFuture<'a, Result<Response, PipelineError<RouterError>>> {
        Box::pin(async move {
            // 1. Bypass if the HTTP method is not permitted for body parsing (e.g. GET, HEAD)
            if !self.config.is_method_allowed(ctx.request.intended_method()) {
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

            // 4. Decode according to MIME type
            if content_type.starts_with("multipart/form-data") {
                // Check if route is marked for manual streaming bypass
                if self.config.should_process_manually(ctx.request.path()) {
                    return next.run(ctx).await;
                }

                let boundary = extract_boundary(&content_type)
                    .map_err(|error| PipelineError::Execution(RouterError::from(error)))?;

                let mut parser = MultipartParser::new(boundary, *self.config.limits());
                if let Some(temp_dir) = self.config.temp_dir() {
                    parser = parser.temp_dir(temp_dir);
                }

                let parsed = parser
                    .parse_slice(ctx.request.body())
                    .map_err(|error| PipelineError::Execution(RouterError::from(error)))?;

                let structured_tree = parsed
                    .structured_fields()
                    .map_err(|error| PipelineError::Execution(RouterError::from(error)))?;

                let raw_pairs: Vec<(String, String)> = parsed
                    .text_fields()
                    .map(|(k, v)| (k.to_string(), v.to_string()))
                    .collect();

                let form_fields = FormFields::new(raw_pairs, structured_tree);
                let uploaded_files = UploadedFiles::new(parsed.into_files());

                ctx.extensions.insert(form_fields);
                ctx.extensions.insert(uploaded_files);
            } else if content_type.starts_with("application/json") {
                parse_json_value(ctx.request.body(), self.config.limits())
                    .map_err(|error| PipelineError::Execution(RouterError::from(error)))?;
            } else if content_type.starts_with("application/x-www-form-urlencoded") {
                let tree = parse_form_value(ctx.request.body(), self.config.limits())
                    .map_err(|error| PipelineError::Execution(RouterError::from(error)))?;

                let raw_query = std::str::from_utf8(ctx.request.body()).unwrap_or("");
                let raw_pairs: Vec<(String, String)> = raw_query
                    .split('&')
                    .filter(|pair| !pair.is_empty())
                    .filter_map(|pair| {
                        pair.split_once('=')
                            .map(|(k, v)| (decode_component(k), decode_component(v)))
                    })
                    .collect();

                let form_fields = FormFields::new(raw_pairs, tree);
                ctx.extensions.insert(form_fields);
            } else if content_type.starts_with("text/") {
                parse_text(ctx.request.body(), self.config.limits())
                    .map_err(|error| PipelineError::Execution(RouterError::from(error)))?;
            }

            next.run(ctx).await
        })
    }
}
