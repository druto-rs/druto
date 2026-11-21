//! In-memory zero-port test harness and fluent client for Druto applications.

use std::sync::Arc;

use bytes::Bytes;
use poth::Method;
use router::{CompiledRouter, HttpContext, StatusCode};
use serde::{Serialize, de::DeserializeOwned};

use crate::{error::ServerError, server::IntoCompiledRouter};

/// Fluent HTTP test response wrapper offering inspection and assertion utilities.
pub struct TestResponse {
    status: StatusCode,
    headers: router::HeaderMap,
    body: Bytes,
}

impl TestResponse {
    /// Returns the HTTP status code of the response.
    #[must_use]
    pub fn status(&self) -> StatusCode {
        self.status
    }

    /// Returns an immutable reference to the response headers.
    #[must_use]
    pub fn headers(&self) -> &router::HeaderMap {
        &self.headers
    }

    /// Returns the response body as raw bytes.
    #[must_use]
    pub fn body(&self) -> &Bytes {
        &self.body
    }

    /// Returns the response body decoded as a UTF-8 string.
    pub fn text(&self) -> Result<String, std::string::FromUtf8Error> {
        String::from_utf8(self.body.to_vec())
    }

    /// Deserializes the response body JSON payload into the target type.
    pub fn json<T: DeserializeOwned>(&self) -> Result<T, serde_json::Error> {
        serde_json::from_slice(&self.body)
    }

    /// Asserts that the response status code equals `expected`.
    pub fn assert_status(&self, expected: StatusCode) {
        assert_eq!(
            self.status, expected,
            "Expected status {expected}, got {}",
            self.status
        );
    }

    /// Asserts that the response status is 200 OK.
    pub fn assert_ok(&self) {
        self.assert_status(StatusCode::OK);
    }

    /// Asserts that the response text matches `expected`.
    pub fn assert_text(&self, expected: &str) {
        let text = self.text().expect("Failed to decode response as UTF-8");
        assert_eq!(text, expected);
    }
}

/// Fluent request builder for in-memory [`TestServer`] dispatches.
pub struct TestRequestBuilder {
    router: Arc<CompiledRouter>,
    method: Method,
    path: String,
    query: Option<String>,
    headers: router::HeaderMap,
    body: Bytes,
}

impl TestRequestBuilder {
    fn new(router: Arc<CompiledRouter>, method: Method, path: String) -> Self {
        Self {
            router,
            method,
            path,
            query: None,
            headers: router::HeaderMap::new(),
            body: Bytes::new(),
        }
    }

    /// Adds a request header name and value.
    #[must_use]
    pub fn header(
        mut self,
        name: impl Into<router::HeaderName>,
        value: impl Into<router::HeaderValue>,
    ) -> Self {
        self.headers.insert(name.into(), value.into());
        self
    }

    /// Sets query string parameters on the request.
    #[must_use]
    pub fn query(mut self, query: impl Into<String>) -> Self {
        self.query = Some(query.into());
        self
    }

    /// Sets raw payload bytes on the request.
    #[must_use]
    pub fn body(mut self, bytes: impl Into<Bytes>) -> Self {
        self.body = bytes.into();
        self
    }

    /// Sets text string payload and adds `Content-Type: text/plain` if omitted.
    #[must_use]
    pub fn text(mut self, text: impl Into<String>) -> Self {
        let text_content = text.into();
        if !self.headers.contains_key("content-type") {
            self = self.header("content-type", "text/plain; charset=utf-8");
        }
        self.body = Bytes::from(text_content.into_bytes());
        self
    }

    /// Serializes `payload` as JSON and adds `Content-Type: application/json` if omitted.
    pub fn json<T: Serialize>(mut self, payload: &T) -> Result<Self, serde_json::Error> {
        let json_bytes = serde_json::to_vec(payload)?;
        if !self.headers.contains_key("content-type") {
            self = self.header("content-type", "application/json");
        }
        self.body = Bytes::from(json_bytes);
        Ok(self)
    }

    /// Dispatches the request through the in-memory router pipeline.
    pub async fn send(self) -> Result<TestResponse, ServerError> {
        let mut req_builder = router::Request::builder()
            .method(self.method)
            .path(self.path)
            .body(self.body);

        if let Some(query_str) = self.query {
            req_builder = req_builder.query(query_str);
        }

        for (name, values) in self.headers.iter() {
            for val in values {
                req_builder = req_builder.header(name.as_str(), val.as_str());
            }
        }

        let req = req_builder.build();
        let mut ctx = HttpContext::new(req);

        self.router.dispatch(&mut ctx).await?;

        Ok(TestResponse {
            status: ctx.response.status(),
            headers: ctx.response.headers().clone(),
            body: ctx.response.body().as_bytes(),
        })
    }
}

/// In-memory zero-port testing server executing the Druto request pipeline without network binding.
#[derive(Clone)]
pub struct TestServer {
    router: Arc<CompiledRouter>,
}

impl TestServer {
    /// Creates a new `TestServer` wrapping the given router table.
    pub fn new(router: impl IntoCompiledRouter) -> Result<Self, ServerError> {
        let compiled = router.into_compiled()?;
        Ok(Self { router: compiled })
    }

    /// Initiates a `GET` request builder against `path`.
    #[must_use]
    pub fn get(&self, path: impl Into<String>) -> TestRequestBuilder {
        self.request(Method::Get, path)
    }

    /// Initiates a `POST` request builder against `path`.
    #[must_use]
    pub fn post(&self, path: impl Into<String>) -> TestRequestBuilder {
        self.request(Method::Post, path)
    }

    /// Initiates a `PUT` request builder against `path`.
    #[must_use]
    pub fn put(&self, path: impl Into<String>) -> TestRequestBuilder {
        self.request(Method::Put, path)
    }

    /// Initiates a `DELETE` request builder against `path`.
    #[must_use]
    pub fn delete(&self, path: impl Into<String>) -> TestRequestBuilder {
        self.request(Method::Delete, path)
    }

    /// Initiates a `PATCH` request builder against `path`.
    #[must_use]
    pub fn patch(&self, path: impl Into<String>) -> TestRequestBuilder {
        self.request(Method::Patch, path)
    }

    /// Initiates a request builder with the specified method and path.
    #[must_use]
    pub fn request(&self, method: Method, path: impl Into<String>) -> TestRequestBuilder {
        TestRequestBuilder::new(self.router.clone(), method, path.into())
    }
}
