//! Asynchronous onion middleware for Cross-Origin Resource Sharing (CORS) enforcement.

use std::sync::Arc;

use dhap::{BoxFuture, Error as DhapError, Middleware, Next};
use router::{HeaderName, HttpContext, Method, Response, ResponseBody, RouterError};

use crate::{
    config::{CorsConfig, CorsConfigBuilder, HeaderPolicy},
    constants,
};

/// CORS enforcement and preflight handling middleware for the Druto pipeline.
#[derive(Clone, Debug)]
pub struct CorsMiddleware {
    config: Arc<CorsConfig>,
}

impl CorsMiddleware {
    /// Creates a new `CorsMiddleware` with the provided configuration.
    #[must_use]
    pub fn new(config: CorsConfig) -> Self {
        Self {
            config: Arc::new(config),
        }
    }

    /// Creates a new `CorsMiddleware` from a configuration builder.
    #[must_use]
    pub fn from_builder(builder: CorsConfigBuilder) -> Self {
        Self::new(builder.build())
    }

    /// Creates a permissive `CorsMiddleware` allowing any origin and method with credentials.
    #[must_use]
    pub fn permissive() -> Self {
        Self::new(
            CorsConfig::builder()
                .allow_any_origin()
                .allow_any_methods()
                .mirror_headers()
                .allow_credentials(true)
                .build(),
        )
    }

    /// Returns a reference to the active CORS configuration.
    #[must_use]
    pub fn config(&self) -> &CorsConfig {
        &self.config
    }

    async fn execute(
        &self,
        ctx: &mut HttpContext,
        next: Next<HttpContext, Response, RouterError>,
    ) -> Result<Response, DhapError<RouterError>> {
        // 1. Bypass if CORS middleware is disabled.
        if !self.config.enabled {
            return next.run(ctx).await;
        }

        // 2. Bypass if request URI path is outside configured path patterns.
        let path = ctx.request.path().to_string();
        if !self.config.matches_request_path(&path) {
            return next.run(ctx).await;
        }

        // 3. Bypass if not a cross-origin request (no Origin header present).
        let origin = match ctx.request.header(constants::ORIGIN) {
            Some(val) if !val.trim().is_empty() => val.trim().to_string(),
            _ => return next.run(ctx).await,
        };

        // 4. Check if request is an OPTIONS preflight request.
        let is_options = *ctx.request.method() == Method::Options;
        let request_method_header = ctx
            .request
            .header(constants::ACCESS_CONTROL_REQUEST_METHOD)
            .map(|val| val.trim().to_string());

        if is_options && let Some(ref req_method) = request_method_header {
            return self.handle_preflight(ctx, &origin, req_method);
        }

        // 5. Handle actual cross-origin request.
        self.handle_actual_request(ctx, &origin, next).await
    }

    fn handle_preflight(
        &self,
        ctx: &mut HttpContext,
        origin: &str,
        requested_method_raw: &str,
    ) -> Result<Response, DhapError<RouterError>> {
        // Evaluate origin policy
        let allow_origin = match self.config.compute_allow_origin(origin, ctx) {
            Some(allowed) => allowed,
            None => {
                // Origin disallowed: fail closed with preflight status and zero CORS headers.
                ctx.response.set_status(self.config.preflight_status);
                ctx.response.set_body(ResponseBody::Empty);
                return Ok(ctx.response.clone());
            }
        };

        // Evaluate requested HTTP method
        let requested_method = Method::from(requested_method_raw);
        if !self.config.is_method_allowed(&requested_method) {
            // Method disallowed: fail closed with preflight status and zero CORS headers.
            ctx.response.set_status(self.config.preflight_status);
            ctx.response.set_body(ResponseBody::Empty);
            return Ok(ctx.response.clone());
        }

        // Evaluate requested headers
        let requested_headers_raw = ctx
            .request
            .header(constants::ACCESS_CONTROL_REQUEST_HEADERS)
            .map(ToString::to_string);
        let requested_headers: Vec<&str> = match requested_headers_raw.as_deref() {
            Some(raw) if !raw.trim().is_empty() => raw.split(',').map(str::trim).collect(),
            _ => Vec::new(),
        };

        if !self.config.are_headers_allowed(&requested_headers) {
            // Headers disallowed: fail closed with preflight status and zero CORS headers.
            ctx.response.set_status(self.config.preflight_status);
            ctx.response.set_body(ResponseBody::Empty);
            return Ok(ctx.response.clone());
        }

        // All checks passed: populate preflight response and short-circuit execution.
        ctx.response.set_status(self.config.preflight_status);
        ctx.response.set_body(ResponseBody::Empty);

        ctx.response
            .set_header(constants::ACCESS_CONTROL_ALLOW_ORIGIN, allow_origin);

        if self.config.credentials {
            ctx.response
                .set_header(constants::ACCESS_CONTROL_ALLOW_CREDENTIALS, "true");
        }

        ctx.response.set_header(
            constants::ACCESS_CONTROL_ALLOW_METHODS,
            self.config.formatted_methods(),
        );

        match &self.config.headers {
            HeaderPolicy::Mirror => {
                if let Some(headers_str) = requested_headers_raw
                    && !headers_str.trim().is_empty()
                {
                    ctx.response
                        .set_header(constants::ACCESS_CONTROL_ALLOW_HEADERS, headers_str.trim());
                }
            }
            HeaderPolicy::List(allowed) => {
                let headers_str = allowed
                    .iter()
                    .map(HeaderName::as_str)
                    .collect::<Vec<_>>()
                    .join(",");
                ctx.response
                    .set_header(constants::ACCESS_CONTROL_ALLOW_HEADERS, headers_str);
            }
        }

        if let Some(max_age) = self.config.max_age {
            ctx.response
                .set_header(constants::ACCESS_CONTROL_MAX_AGE, max_age.to_string());
        }

        if let Some(exposed) = self.config.normalized_expose_headers() {
            ctx.response
                .set_header(constants::ACCESS_CONTROL_EXPOSE_HEADERS, exposed);
        }

        // Append Vary headers to prevent intermediate cache poisoning.
        append_vary(&mut ctx.response, "origin");
        append_vary(&mut ctx.response, "access-control-request-method");
        append_vary(&mut ctx.response, "access-control-request-headers");

        Ok(ctx.response.clone())
    }

    async fn handle_actual_request(
        &self,
        ctx: &mut HttpContext,
        origin: &str,
        next: Next<HttpContext, Response, RouterError>,
    ) -> Result<Response, DhapError<RouterError>> {
        let allow_origin = self.config.compute_allow_origin(origin, ctx);

        // If origin is not allowed, proceed without setting CORS headers so browser blocks response.
        if allow_origin.is_none() {
            return next.run(ctx).await;
        }

        let allow_origin_str = allow_origin.unwrap();

        // Stage CORS headers onto context response
        ctx.response.set_header(
            constants::ACCESS_CONTROL_ALLOW_ORIGIN,
            allow_origin_str.clone(),
        );

        if self.config.credentials {
            ctx.response
                .set_header(constants::ACCESS_CONTROL_ALLOW_CREDENTIALS, "true");
        }

        if let Some(exposed) = self.config.normalized_expose_headers() {
            ctx.response
                .set_header(constants::ACCESS_CONTROL_EXPOSE_HEADERS, exposed);
        }

        if self.should_vary_origin() {
            append_vary(&mut ctx.response, "origin");
        }

        // Advance downstream execution
        let mut result = next.run(ctx).await;

        // Ensure CORS headers remain on outgoing response
        if let Ok(ref mut response) = result {
            response.set_header(
                constants::ACCESS_CONTROL_ALLOW_ORIGIN,
                allow_origin_str.clone(),
            );

            if self.config.credentials {
                response.set_header(constants::ACCESS_CONTROL_ALLOW_CREDENTIALS, "true");
            }

            if let Some(exposed) = self.config.normalized_expose_headers() {
                response.set_header(constants::ACCESS_CONTROL_EXPOSE_HEADERS, exposed);
            }

            if self.should_vary_origin() {
                append_vary(response, "origin");
                append_vary(&mut ctx.response, "origin");
            }
        }

        result
    }

    fn should_vary_origin(&self) -> bool {
        // Vary: Origin is needed whenever origin is not an unconditional '*' without credentials.
        match &self.config.origin {
            crate::config::OriginPolicy::Any => self.config.credentials,
            _ => true,
        }
    }
}

impl Default for CorsMiddleware {
    fn default() -> Self {
        Self::new(CorsConfig::default())
    }
}

impl Middleware<HttpContext, Response, RouterError> for CorsMiddleware {
    fn handle<'a>(
        &'a self,
        ctx: &'a mut HttpContext,
        next: Next<HttpContext, Response, RouterError>,
    ) -> BoxFuture<'a, Result<Response, DhapError<RouterError>>> {
        Box::pin(async move { self.execute(ctx, next).await })
    }
}

fn append_vary(response: &mut Response, vary_token: &str) {
    let lower_token = vary_token.to_ascii_lowercase();
    for existing in response.headers().get_all(constants::VARY) {
        for part in existing.split(',') {
            if part.trim().eq_ignore_ascii_case(&lower_token) {
                return;
            }
        }
    }
    response.append_header(constants::VARY, vary_token);
}
