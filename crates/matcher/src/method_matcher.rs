//! HTTP method-aware composite routing with automatic 405 Method Not Allowed detection and RFC 9110 HEAD fallback.

use std::collections::HashMap;

use crate::{
    error::MatcherError,
    matcher::{Match, Matcher, MatcherConfig},
    method::Method,
};

/// Method-aware router indexing routes by [`Method`] and path pattern.
#[derive(Clone, Debug)]
pub struct MethodMatcher<T> {
    method_matchers: HashMap<Method, Matcher<T>>,
    named_routes: HashMap<String, Method>,
    config: MatcherConfig,
}

impl<T> Default for MethodMatcher<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> MethodMatcher<T> {
    /// Creates a new empty `MethodMatcher`.
    #[must_use]
    pub fn new() -> Self {
        Self {
            method_matchers: HashMap::new(),
            named_routes: HashMap::new(),
            config: MatcherConfig::default(),
        }
    }

    /// Creates a `MethodMatcher` with custom configuration.
    #[must_use]
    pub fn with_config(config: MatcherConfig) -> Self {
        Self {
            method_matchers: HashMap::new(),
            named_routes: HashMap::new(),
            config,
        }
    }

    /// Registers a route for a specific HTTP method.
    pub fn insert(&mut self, method: Method, pattern: &str, payload: T) -> Result<(), MatcherError>
    where
        T: Clone,
    {
        let matcher = self
            .method_matchers
            .entry(method)
            .or_insert_with(|| Matcher::with_config(self.config.clone()));

        matcher.insert(pattern, payload)
    }

    /// Registers a named route for a specific HTTP method for reverse URL generation.
    pub fn insert_named(
        &mut self,
        name: &str,
        method: Method,
        pattern: &str,
        payload: T,
    ) -> Result<(), MatcherError>
    where
        T: Clone,
    {
        let matcher = self
            .method_matchers
            .entry(method.clone())
            .or_insert_with(|| Matcher::with_config(self.config.clone()));

        matcher.insert_named(name, pattern, payload)?;
        self.named_routes.insert(name.to_string(), method);
        Ok(())
    }

    /// Registers a route with parameter constraints for a specific HTTP method.
    pub fn insert_with_constraints(
        &mut self,
        method: Method,
        pattern: &str,
        payload: T,
        constraints: HashMap<String, crate::constraint::Constraint>,
    ) -> Result<(), MatcherError>
    where
        T: Clone,
    {
        let matcher = self
            .method_matchers
            .entry(method)
            .or_insert_with(|| Matcher::with_config(self.config.clone()));

        matcher.insert_with_constraints(pattern, payload, constraints)
    }

    /// Registers a named route with parameter constraints for a specific HTTP method.
    pub fn insert_named_with_constraints(
        &mut self,
        name: &str,
        method: Method,
        pattern: &str,
        payload: T,
        constraints: HashMap<String, crate::constraint::Constraint>,
    ) -> Result<(), MatcherError>
    where
        T: Clone,
    {
        let matcher = self
            .method_matchers
            .entry(method.clone())
            .or_insert_with(|| Matcher::with_config(self.config.clone()));

        matcher.insert_named_with_constraints(name, pattern, payload, constraints)?;
        self.named_routes.insert(name.to_string(), method);
        Ok(())
    }

    /// Registers a route that responds to any HTTP method.
    pub fn insert_any(&mut self, pattern: &str, payload: T) -> Result<(), MatcherError>
    where
        T: Clone,
    {
        self.insert(Method::Any, pattern, payload)
    }

    /// Registers a route for multiple HTTP methods.
    pub fn match_methods(
        &mut self,
        methods: &[Method],
        pattern: &str,
        payload: T,
    ) -> Result<(), MatcherError>
    where
        T: Clone,
    {
        for method in methods {
            self.insert(method.clone(), pattern, payload.clone())?;
        }
        Ok(())
    }

    /// Convenience registration for GET requests.
    pub fn get(&mut self, pattern: &str, payload: T) -> Result<(), MatcherError>
    where
        T: Clone,
    {
        self.insert(Method::Get, pattern, payload)
    }

    /// Convenience registration for POST requests.
    pub fn post(&mut self, pattern: &str, payload: T) -> Result<(), MatcherError>
    where
        T: Clone,
    {
        self.insert(Method::Post, pattern, payload)
    }

    /// Convenience registration for PUT requests.
    pub fn put(&mut self, pattern: &str, payload: T) -> Result<(), MatcherError>
    where
        T: Clone,
    {
        self.insert(Method::Put, pattern, payload)
    }

    /// Convenience registration for DELETE requests.
    pub fn delete(&mut self, pattern: &str, payload: T) -> Result<(), MatcherError>
    where
        T: Clone,
    {
        self.insert(Method::Delete, pattern, payload)
    }

    /// Convenience registration for PATCH requests.
    pub fn patch(&mut self, pattern: &str, payload: T) -> Result<(), MatcherError>
    where
        T: Clone,
    {
        self.insert(Method::Patch, pattern, payload)
    }

    /// Convenience registration for HEAD requests.
    pub fn head(&mut self, pattern: &str, payload: T) -> Result<(), MatcherError>
    where
        T: Clone,
    {
        self.insert(Method::Head, pattern, payload)
    }

    /// Convenience registration for OPTIONS requests.
    pub fn options(&mut self, pattern: &str, payload: T) -> Result<(), MatcherError>
    where
        T: Clone,
    {
        self.insert(Method::Options, pattern, payload)
    }

    /// Convenience registration for RFC 10008 `QUERY` requests.
    pub fn query(&mut self, pattern: &str, payload: T) -> Result<(), MatcherError>
    where
        T: Clone,
    {
        self.insert(Method::Query, pattern, payload)
    }

    /// Convenience registration for PURGE cache invalidation requests.
    pub fn purge(&mut self, pattern: &str, payload: T) -> Result<(), MatcherError>
    where
        T: Clone,
    {
        self.insert(Method::Purge, pattern, payload)
    }

    /// Convenience registration for RFC 3253 / RFC 4791 `REPORT` requests.
    pub fn report(&mut self, pattern: &str, payload: T) -> Result<(), MatcherError>
    where
        T: Clone,
    {
        self.insert(Method::Report, pattern, payload)
    }

    /// Convenience registration for RFC 5323 `SEARCH` requests.
    pub fn search(&mut self, pattern: &str, payload: T) -> Result<(), MatcherError>
    where
        T: Clone,
    {
        self.insert(Method::Search, pattern, payload)
    }

    /// Convenience registration for RFC 4791 `MKCALENDAR` requests.
    pub fn mkcalendar(&mut self, pattern: &str, payload: T) -> Result<(), MatcherError>
    where
        T: Clone,
    {
        self.insert(Method::Mkcalendar, pattern, payload)
    }

    /// Convenience registration for RFC 3744 `ACL` requests.
    pub fn acl(&mut self, pattern: &str, payload: T) -> Result<(), MatcherError>
    where
        T: Clone,
    {
        self.insert(Method::Acl, pattern, payload)
    }

    /// Convenience registration for BAN cache invalidation requests.
    pub fn ban(&mut self, pattern: &str, payload: T) -> Result<(), MatcherError>
    where
        T: Clone,
    {
        self.insert(Method::Ban, pattern, payload)
    }

    /// Looks up a route by HTTP method and URI path.
    ///
    /// Handles RFC 9110 HEAD fallback to GET and returns 405 Method Not Allowed with
    /// the full list of allowed methods if the path matches other verbs.
    pub fn find<'a>(
        &'a self,
        method: &Method,
        uri: &'a str,
    ) -> Result<MethodMatch<'a, T>, MatcherError> {
        // 1. Direct match on requested method
        if let Some(matcher) = self.method_matchers.get(method)
            && let Ok(matched) = matcher.find(uri)
        {
            return Ok(MethodMatch {
                inner: matched,
                matched_method: method.clone(),
            });
        }

        // 2. RFC 9110 HEAD fallback to GET
        if *method == Method::Head
            && let Some(get_matcher) = self.method_matchers.get(&Method::Get)
            && let Ok(matched) = get_matcher.find(uri)
        {
            return Ok(MethodMatch {
                inner: matched,
                matched_method: Method::Get,
            });
        }

        // 3. Fallback to Method::Any wildcard
        if let Some(any_matcher) = self.method_matchers.get(&Method::Any)
            && let Ok(matched) = any_matcher.find(uri)
        {
            return Ok(MethodMatch {
                inner: matched,
                matched_method: Method::Any,
            });
        }

        // 4. 405 Method Not Allowed detection: scan all other registered methods
        let mut allowed = Vec::new();
        for (m, matcher) in &self.method_matchers {
            if m != method && m != &Method::Any && matcher.find(uri).is_ok() {
                allowed.push(m.clone());
            }
        }

        if !allowed.is_empty() {
            // If GET is allowed, HEAD is also implicitly allowed
            if allowed.contains(&Method::Get) && !allowed.contains(&Method::Head) {
                allowed.push(Method::Head);
            }
            allowed.sort_by(|a, b| a.as_str().cmp(b.as_str()));

            let clean_path = uri.split_once('?').map_or(uri, |(p, _)| p);
            return Err(MatcherError::MethodNotAllowed {
                method: method.clone(),
                path: clean_path.to_string(),
                allowed,
            });
        }

        let clean_path = uri.split_once('?').map_or(uri, |(p, _)| p);
        Err(MatcherError::NotFound {
            path: clean_path.to_string(),
        })
    }

    /// Synthesizes a URL path from a named route and parameter key-value pairs.
    pub fn build_path(&self, name: &str, params: &[(&str, &str)]) -> Result<String, MatcherError> {
        let method = self
            .named_routes
            .get(name)
            .ok_or_else(|| MatcherError::RouteNotFound {
                name: name.to_string(),
            })?;

        let matcher =
            self.method_matchers
                .get(method)
                .ok_or_else(|| MatcherError::RouteNotFound {
                    name: name.to_string(),
                })?;

        matcher.build_path(name, params)
    }
}

/// Successful method match result.
#[derive(Debug, Clone)]
pub struct MethodMatch<'a, T> {
    inner: Match<'a, T>,
    matched_method: Method,
}

impl<'a, T> std::ops::Deref for MethodMatch<'a, T> {
    type Target = Match<'a, T>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<'a, T> MethodMatch<'a, T> {
    /// Returns the HTTP method that produced this match.
    #[must_use]
    pub fn matched_method(&self) -> &Method {
        &self.matched_method
    }
}
