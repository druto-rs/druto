//! Asynchronous HTTP server engine, socket listener, and runtime-agnostic connection lifecycle manager.

use std::{future::Future, net::ToSocketAddrs, path::PathBuf, pin::Pin, sync::Arc, time::Duration};

use futures_io::{AsyncRead, AsyncWrite};
use router::{CompiledRouter, HttpContext, Router};
use sebok::{RequestBody, ResponseBody, ServerConfig, ServerConfigBuilder};

use crate::error::ServerError;

/// Conversion trait transforming router builders and compiled route tables into an executable [`CompiledRouter`].
pub trait IntoCompiledRouter {
    /// Consumes or borrows the router and returns the frozen [`CompiledRouter`].
    fn into_compiled(self) -> Result<Arc<CompiledRouter>, ServerError>;
}

impl IntoCompiledRouter for Router {
    fn into_compiled(self) -> Result<Arc<CompiledRouter>, ServerError> {
        let compiled = self.compile()?;
        Ok(Arc::new(compiled))
    }
}

impl IntoCompiledRouter for CompiledRouter {
    fn into_compiled(self) -> Result<Arc<CompiledRouter>, ServerError> {
        Ok(Arc::new(self))
    }
}

impl IntoCompiledRouter for Arc<CompiledRouter> {
    fn into_compiled(self) -> Result<Arc<CompiledRouter>, ServerError> {
        Ok(self)
    }
}

/// Dispatches an incoming HTTP/1.1 request through the compiled router pipeline.
pub async fn dispatch_http_request(
    compiled: Arc<CompiledRouter>,
    http_req: http::Request<RequestBody>,
) -> Result<http::Response<ResponseBody>, sebok::SebokError> {
    let (parts, body) = http_req.into_parts();
    let method = poth::Method::from(parts.method.as_str());
    let path = parts.uri.path().to_string();
    let query = parts.uri.query().map(ToString::to_string);

    let body_bytes = body.into_bytes().await?;

    let mut req_builder = router::Request::builder()
        .method(method)
        .path(path)
        .body(body_bytes);

    if let Some(query_str) = query {
        req_builder = req_builder.query(query_str);
    }

    for (name, val) in &parts.headers {
        if let Ok(val_str) = val.to_str() {
            req_builder = req_builder.header(name.as_str(), val_str);
        }
    }

    let req = req_builder.build();
    let mut ctx = HttpContext::new(req);

    compiled
        .dispatch(&mut ctx)
        .await
        .map_err(|err| sebok::SebokError::Protocol(err.to_string()))?;

    let status =
        http::StatusCode::from_u16(ctx.response.status().as_u16()).unwrap_or(http::StatusCode::OK);

    let mut resp_builder = http::Response::builder().status(status);
    for (name, values) in ctx.response.headers().iter() {
        for val in values {
            resp_builder = resp_builder.header(name.as_str(), val.as_str());
        }
    }

    let resp_body = ResponseBody::Bytes(ctx.response.body().as_bytes());
    resp_builder
        .body(resp_body)
        .map_err(|err| sebok::SebokError::Protocol(err.to_string()))
}

/// Universal HTTP server and socket listener.
pub struct Server {
    config_builder: ServerConfigBuilder,
    tcp_addr: Option<String>,
    #[cfg(unix)]
    unix_path: Option<PathBuf>,
    tcp_nodelay: bool,
    shutdown_signal: Option<Pin<Box<dyn Future<Output = ()> + Send>>>,
}

impl Server {
    /// Creates a server configured to bind to the given TCP socket address.
    #[must_use]
    pub fn bind(addr: impl Into<String>) -> Self {
        Self {
            config_builder: ServerConfig::builder(),
            tcp_addr: Some(addr.into()),
            #[cfg(unix)]
            unix_path: None,
            tcp_nodelay: true,
            shutdown_signal: None,
        }
    }

    /// Creates a server configured from the `HOST` and `PORT` environment variables.
    ///
    /// Defaults to `127.0.0.1:5555` if neither environment variable is defined.
    #[must_use]
    pub fn from_env() -> Self {
        let host = std::env::var("HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
        let port = std::env::var("PORT").unwrap_or_else(|_| "5555".to_string());
        Self::bind(format!("{host}:{port}"))
    }

    /// Returns the configured TCP socket address string if set.
    #[must_use]
    pub fn tcp_addr(&self) -> Option<&str> {
        self.tcp_addr.as_deref()
    }

    /// Creates a server configured to bind to a Unix Domain Socket path.
    #[cfg(unix)]
    #[must_use]
    pub fn bind_unix(path: impl Into<PathBuf>) -> Self {
        Self {
            config_builder: ServerConfig::builder(),
            tcp_addr: None,
            unix_path: Some(path.into()),
            tcp_nodelay: false,
            shutdown_signal: None,
        }
    }

    /// Configures whether persistent Keep-Alive connections are permitted.
    #[must_use]
    pub fn keep_alive(mut self, enabled: bool) -> Self {
        self.config_builder = self.config_builder.keep_alive(enabled);
        self
    }

    /// Sets the idle timeout duration for persistent Keep-Alive connections.
    #[must_use]
    pub fn keep_alive_timeout(mut self, timeout: Option<Duration>) -> Self {
        self.config_builder = self.config_builder.keep_alive_timeout(timeout);
        self
    }

    /// Sets the maximum permitted size in bytes for the HTTP request header section.
    #[must_use]
    pub fn max_header_size(mut self, size: usize) -> Self {
        self.config_builder = self.config_builder.max_header_size(size);
        self
    }

    /// Sets the maximum permitted number of request header fields.
    #[must_use]
    pub fn max_headers_count(mut self, count: usize) -> Self {
        self.config_builder = self.config_builder.max_headers_count(count);
        self
    }

    /// Configures whether `Expect: 100-continue` handshakes are automatically processed.
    #[must_use]
    pub fn expect_continue(mut self, enabled: bool) -> Self {
        self.config_builder = self.config_builder.expect_continue(enabled);
        self
    }

    /// Configures the TCP `TCP_NODELAY` socket option.
    #[must_use]
    pub fn tcp_nodelay(mut self, enabled: bool) -> Self {
        self.tcp_nodelay = enabled;
        self
    }

    /// Configures a future that triggers graceful server shutdown when completed.
    #[must_use]
    pub fn with_graceful_shutdown<F>(mut self, signal: F) -> Self
    where
        F: Future<Output = ()> + Send + 'static,
    {
        self.shutdown_signal = Some(Box::pin(signal));
        self
    }

    /// Serves an individual asynchronous stream connection against the provided router table.
    pub async fn serve_stream<S>(
        stream: S,
        router: impl IntoCompiledRouter,
    ) -> Result<(), ServerError>
    where
        S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
    {
        Self::serve_stream_with_config(stream, router, ServerConfig::default()).await
    }

    /// Serves an individual asynchronous stream connection with custom configuration settings.
    pub async fn serve_stream_with_config<S>(
        stream: S,
        router: impl IntoCompiledRouter,
        config: ServerConfig,
    ) -> Result<(), ServerError>
    where
        S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
    {
        let compiled = router.into_compiled()?;
        sebok::serve_connection(
            stream,
            move |http_req| {
                let table = Arc::clone(&compiled);
                dispatch_http_request(table, http_req)
            },
            config,
        )
        .await?;

        Ok(())
    }

    /// Binds the configured listener and serves incoming connections until shutdown.
    pub async fn serve(self, router: impl IntoCompiledRouter) -> Result<(), ServerError> {
        let compiled = router.into_compiled()?;
        let config = self.config_builder.build();
        let nodelay = self.tcp_nodelay;
        let mut shutdown_fut = self.shutdown_signal;

        if let Some(ref addr_str) = self.tcp_addr {
            let mut addrs = addr_str
                .to_socket_addrs()
                .map_err(|err| ServerError::AddressParse(err.to_string()))?;
            let addr = addrs.next().ok_or_else(|| {
                ServerError::AddressParse("No socket address resolved".to_string())
            })?;

            let listener = async_io::Async::<std::net::TcpListener>::bind(addr)?;

            loop {
                let accept_fut = listener.accept();
                tokio::pin!(accept_fut);

                let (stream, _) = if let Some(ref mut shutdown) = shutdown_fut {
                    tokio::select! {
                        accept_res = &mut accept_fut => accept_res?,
                        _ = shutdown.as_mut() => {
                            break;
                        }
                    }
                } else {
                    accept_fut.await?
                };

                if nodelay {
                    let _ = stream.get_ref().set_nodelay(true);
                }

                let route_table = Arc::clone(&compiled);
                let conn_config = config.clone();

                tokio::spawn(async move {
                    let _ = sebok::serve_connection(
                        stream,
                        move |http_req| {
                            let table = Arc::clone(&route_table);
                            dispatch_http_request(table, http_req)
                        },
                        conn_config,
                    )
                    .await;
                });
            }

            return Ok(());
        }

        #[cfg(unix)]
        if let Some(ref path) = self.unix_path {
            let listener = async_io::Async::<std::os::unix::net::UnixListener>::bind(path)?;

            loop {
                let accept_fut = listener.accept();
                tokio::pin!(accept_fut);

                let (stream, _) = if let Some(ref mut shutdown) = shutdown_fut {
                    tokio::select! {
                        accept_res = &mut accept_fut => accept_res?,
                        _ = shutdown.as_mut() => {
                            break;
                        }
                    }
                } else {
                    accept_fut.await?
                };

                let route_table = Arc::clone(&compiled);
                let conn_config = config.clone();

                tokio::spawn(async move {
                    let _ = sebok::serve_connection(
                        stream,
                        move |http_req| {
                            let table = Arc::clone(&route_table);
                            dispatch_http_request(table, http_req)
                        },
                        conn_config,
                    )
                    .await;
                });
            }

            return Ok(());
        }

        Err(ServerError::AddressParse(
            "No binding address configured".to_string(),
        ))
    }
}
