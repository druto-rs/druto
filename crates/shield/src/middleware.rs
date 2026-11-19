//! Onion middleware implementations for unified shield defense and modular security guards.

use std::sync::Arc;

use pipeline::{BoxFuture, Middleware, Next, PipelineError};
use router::{HttpContext, Response, RouterError};

use crate::{
    csp::{CspConfig, CspConfigBuilder, CspNonce},
    csrf::{CsrfConfig, CsrfConfigBuilder, CsrfFailureMode},
    error::ShieldError,
    headers::{SecurityHeadersConfig, SecurityHeadersConfigBuilder, SecurityPreset},
};

/// Unified configuration bundling security headers, CSP, and CSRF protection.
#[derive(Clone, Debug)]
pub struct ShieldConfig {
    /// Optional security response headers configuration.
    pub headers: Option<SecurityHeadersConfig>,
    /// Optional Content Security Policy configuration.
    pub csp: Option<CspConfig>,
    /// Optional Cross-Site Request Forgery configuration.
    pub csrf: Option<CsrfConfig>,
}

impl Default for ShieldConfig {
    fn default() -> Self {
        Self {
            headers: Some(SecurityHeadersConfig::default()),
            csp: Some(CspConfig::default()),
            csrf: Some(CsrfConfig::default()),
        }
    }
}

impl ShieldConfig {
    /// Creates a new unified shield configuration builder.
    #[must_use]
    pub fn builder() -> ShieldConfigBuilder {
        ShieldConfigBuilder::new()
    }
}

/// Fluent builder for [`ShieldConfig`].
#[derive(Clone, Debug, Default)]
pub struct ShieldConfigBuilder {
    headers: Option<SecurityHeadersConfig>,
    csp: Option<CspConfig>,
    csrf: Option<CsrfConfig>,
}

impl ShieldConfigBuilder {
    /// Creates a builder initialized with default security guards.
    #[must_use]
    pub fn new() -> Self {
        Self {
            headers: Some(SecurityHeadersConfig::default()),
            csp: Some(CspConfig::default()),
            csrf: Some(CsrfConfig::default()),
        }
    }

    /// Sets the base security headers preset.
    #[must_use]
    pub fn preset(mut self, preset: SecurityPreset) -> Self {
        self.headers = Some(SecurityHeadersConfig::from_preset(preset));
        self
    }

    /// Configures the security headers guard.
    #[must_use]
    pub fn headers(mut self, config: impl Into<Option<SecurityHeadersConfig>>) -> Self {
        self.headers = config.into();
        self
    }

    /// Configures the Content Security Policy guard.
    #[must_use]
    pub fn csp(mut self, config: impl Into<Option<CspConfig>>) -> Self {
        self.csp = config.into();
        self
    }

    /// Configures the CSRF protection guard.
    #[must_use]
    pub fn csrf(mut self, config: impl Into<Option<CsrfConfig>>) -> Self {
        self.csrf = config.into();
        self
    }

    /// Finalizes and returns the [`ShieldConfig`].
    #[must_use]
    pub fn build(self) -> ShieldConfig {
        ShieldConfig {
            headers: self.headers,
            csp: self.csp,
            csrf: self.csrf,
        }
    }
}

/// Unified security middleware coordinating security headers, CSP, and CSRF protection.
#[derive(Clone, Debug)]
pub struct ShieldMiddleware {
    config: Arc<ShieldConfig>,
}

impl Default for ShieldMiddleware {
    fn default() -> Self {
        Self::new(ShieldConfig::default())
    }
}

impl ShieldMiddleware {
    /// Creates a new `ShieldMiddleware` with the provided configuration.
    #[must_use]
    pub fn new(config: ShieldConfig) -> Self {
        Self {
            config: Arc::new(config),
        }
    }

    /// Returns a reference to the active shield configuration.
    #[must_use]
    pub fn config(&self) -> &ShieldConfig {
        &self.config
    }
}

impl Middleware<HttpContext, Response, RouterError> for ShieldMiddleware {
    fn handle<'a>(
        &'a self,
        ctx: &'a mut HttpContext,
        next: Next<HttpContext, Response, RouterError>,
    ) -> BoxFuture<'a, Result<Response, PipelineError<RouterError>>> {
        Box::pin(async move {
            // 1. Handle CSRF verification and token lifecycle.
            let mut csrf_token_for_cookie = None;
            if let Some(csrf) = self.config.csrf.as_ref().filter(|c| c.enabled) {
                let is_safe = ctx.request.method().is_safe();
                let is_exempt = is_safe || csrf.is_exempt(ctx);

                let secret = csrf
                    .resolve_or_init_secret(ctx)
                    .map_err(|err| PipelineError::Execution(RouterError::from(err)))?;

                if !is_exempt {
                    let maybe_token = csrf.extract_token(ctx);
                    let is_valid = match maybe_token {
                        Some(ref token) => csrf.verify_token(&secret, token),
                        None => false,
                    };

                    if !is_valid {
                        return match csrf.failure_mode {
                            CsrfFailureMode::Response403 => Ok(csrf.forbidden_response()),
                            CsrfFailureMode::Error => Err(PipelineError::Execution(
                                RouterError::from(ShieldError::InvalidCsrfToken),
                            )),
                        };
                    }
                }

                // Generate fresh per-request token and store in extensions.
                let fresh_token = csrf
                    .generate_token(&secret)
                    .map_err(|err| PipelineError::Execution(RouterError::from(err)))?;
                ctx.extensions.insert(fresh_token.clone());
                csrf.attach_xsrf_cookie(ctx, &fresh_token);
                csrf_token_for_cookie = Some((csrf.clone(), fresh_token));
            }

            // 2. Handle CSP per-request nonce generation.
            let mut active_nonce = None;
            if self
                .config
                .csp
                .as_ref()
                .is_some_and(|c| c.enabled && c.generate_nonce)
            {
                let nonce = CspNonce::generate()
                    .map_err(|err| PipelineError::Execution(RouterError::from(err)))?;
                ctx.extensions.insert(nonce.clone());
                active_nonce = Some(nonce);
            }

            // 3. Delegate to downstream pipeline handlers.
            let mut response = next.run(ctx).await?;

            // 4. Ensure XSRF-TOKEN cookie is set on outgoing response.
            if let Some((csrf_cfg, token)) = &csrf_token_for_cookie {
                let missing_cookie = !response
                    .headers()
                    .contains_key(router::HeaderName::SET_COOKIE.as_str());
                if csrf_cfg.enable_xsrf_cookie && missing_cookie {
                    csrf_cfg.attach_xsrf_cookie(ctx, token);
                }
            }

            // 5. Apply Content Security Policy non-destructively.
            if let Some(csp) = &self.config.csp {
                csp.apply(&mut response, active_nonce.as_ref());
            }

            // 6. Apply Security Headers non-destructively.
            if let Some(headers) = &self.config.headers {
                headers.apply(&mut response);
            }

            Ok(response)
        })
    }
}

/// Standalone security response headers middleware.
#[derive(Clone, Debug)]
pub struct SecurityHeadersMiddleware {
    config: Arc<SecurityHeadersConfig>,
}

impl Default for SecurityHeadersMiddleware {
    fn default() -> Self {
        Self::new(SecurityHeadersConfig::default())
    }
}

impl SecurityHeadersMiddleware {
    /// Creates a new `SecurityHeadersMiddleware` with the provided configuration.
    #[must_use]
    pub fn new(config: SecurityHeadersConfig) -> Self {
        Self {
            config: Arc::new(config),
        }
    }

    /// Creates a new `SecurityHeadersMiddleware` from a configuration builder.
    #[must_use]
    pub fn from_builder(builder: SecurityHeadersConfigBuilder) -> Self {
        Self::new(builder.build())
    }

    /// Returns a reference to the active security headers configuration.
    #[must_use]
    pub fn config(&self) -> &SecurityHeadersConfig {
        &self.config
    }
}

impl Middleware<HttpContext, Response, RouterError> for SecurityHeadersMiddleware {
    fn handle<'a>(
        &'a self,
        ctx: &'a mut HttpContext,
        next: Next<HttpContext, Response, RouterError>,
    ) -> BoxFuture<'a, Result<Response, PipelineError<RouterError>>> {
        Box::pin(async move {
            let mut response = next.run(ctx).await?;
            self.config.apply(&mut response);
            Ok(response)
        })
    }
}

/// Standalone Content Security Policy middleware.
#[derive(Clone, Debug)]
pub struct CspMiddleware {
    config: Arc<CspConfig>,
}

impl Default for CspMiddleware {
    fn default() -> Self {
        Self::new(CspConfig::default())
    }
}

impl CspMiddleware {
    /// Creates a new `CspMiddleware` with the provided configuration.
    #[must_use]
    pub fn new(config: CspConfig) -> Self {
        Self {
            config: Arc::new(config),
        }
    }

    /// Creates a new `CspMiddleware` from a configuration builder.
    #[must_use]
    pub fn from_builder(builder: CspConfigBuilder) -> Self {
        Self::new(builder.build())
    }

    /// Returns a reference to the active CSP configuration.
    #[must_use]
    pub fn config(&self) -> &CspConfig {
        &self.config
    }
}

impl Middleware<HttpContext, Response, RouterError> for CspMiddleware {
    fn handle<'a>(
        &'a self,
        ctx: &'a mut HttpContext,
        next: Next<HttpContext, Response, RouterError>,
    ) -> BoxFuture<'a, Result<Response, PipelineError<RouterError>>> {
        Box::pin(async move {
            if !self.config.enabled {
                return next.run(ctx).await;
            }

            let mut active_nonce = None;
            if self.config.generate_nonce {
                let nonce = CspNonce::generate()
                    .map_err(|err| PipelineError::Execution(RouterError::from(err)))?;
                ctx.extensions.insert(nonce.clone());
                active_nonce = Some(nonce);
            }

            let mut response = next.run(ctx).await?;
            self.config.apply(&mut response, active_nonce.as_ref());
            Ok(response)
        })
    }
}

/// Standalone Cross-Site Request Forgery (CSRF) guard middleware.
#[derive(Clone, Debug)]
pub struct CsrfMiddleware {
    config: Arc<CsrfConfig>,
}

impl Default for CsrfMiddleware {
    fn default() -> Self {
        Self::new(CsrfConfig::default())
    }
}

impl CsrfMiddleware {
    /// Creates a new `CsrfMiddleware` with the provided configuration.
    #[must_use]
    pub fn new(config: CsrfConfig) -> Self {
        Self {
            config: Arc::new(config),
        }
    }

    /// Creates a new `CsrfMiddleware` from a configuration builder.
    #[must_use]
    pub fn from_builder(builder: CsrfConfigBuilder) -> Self {
        Self::new(builder.build())
    }

    /// Returns a reference to the active CSRF configuration.
    #[must_use]
    pub fn config(&self) -> &CsrfConfig {
        &self.config
    }
}

impl Middleware<HttpContext, Response, RouterError> for CsrfMiddleware {
    fn handle<'a>(
        &'a self,
        ctx: &'a mut HttpContext,
        next: Next<HttpContext, Response, RouterError>,
    ) -> BoxFuture<'a, Result<Response, PipelineError<RouterError>>> {
        Box::pin(async move {
            if !self.config.enabled {
                return next.run(ctx).await;
            }

            let is_safe = ctx.request.method().is_safe();
            let is_exempt = is_safe || self.config.is_exempt(ctx);

            let secret = self
                .config
                .resolve_or_init_secret(ctx)
                .map_err(|err| PipelineError::Execution(RouterError::from(err)))?;

            if !is_exempt {
                let maybe_token = self.config.extract_token(ctx);
                let is_valid = match maybe_token {
                    Some(ref token) => self.config.verify_token(&secret, token),
                    None => false,
                };

                if !is_valid {
                    return match self.config.failure_mode {
                        CsrfFailureMode::Response403 => Ok(self.config.forbidden_response()),
                        CsrfFailureMode::Error => Err(PipelineError::Execution(RouterError::from(
                            ShieldError::InvalidCsrfToken,
                        ))),
                    };
                }
            }

            // Generate per-request token and store in extensions.
            let fresh_token = self
                .config
                .generate_token(&secret)
                .map_err(|err| PipelineError::Execution(RouterError::from(err)))?;
            ctx.extensions.insert(fresh_token.clone());
            self.config.attach_xsrf_cookie(ctx, &fresh_token);

            next.run(ctx).await
        })
    }
}
