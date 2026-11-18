//! Core `Matcher<T>` path tree container, route registration, and reverse path synthesis.

use std::collections::HashMap;

use crate::{
    constraint::Constraint,
    error::MatcherError,
    params::{Params, QueryParams},
    parser::RouteParser,
    segment::Segment,
    tree::{RouteEntry, RouteNode},
};

/// Configuration options for the route matcher.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MatcherConfig {
    /// When true, `/path` and `/path/` match the same route (default: `true`).
    pub flexible_trailing_slash: bool,
    /// When true, static path segments are case-sensitive (default: `true`).
    pub case_sensitive: bool,
}

impl Default for MatcherConfig {
    fn default() -> Self {
        Self {
            flexible_trailing_slash: true,
            case_sensitive: true,
        }
    }
}

/// High-performance URL pattern matcher and parameter extraction engine.
#[derive(Clone, Debug)]
pub struct Matcher<T> {
    root: RouteNode<T>,
    named_routes: HashMap<String, (String, Vec<Segment>)>,
    config: MatcherConfig,
}

impl<T> Default for Matcher<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> Matcher<T> {
    /// Creates an empty `Matcher` with default configuration.
    #[must_use]
    pub fn new() -> Self {
        Self {
            root: RouteNode::new(),
            named_routes: HashMap::new(),
            config: MatcherConfig::default(),
        }
    }

    /// Creates a `Matcher` with custom configuration.
    #[must_use]
    pub fn with_config(config: MatcherConfig) -> Self {
        Self {
            root: RouteNode::new(),
            named_routes: HashMap::new(),
            config,
        }
    }

    /// Inserts a route pattern with an associated payload.
    pub fn insert(&mut self, pattern: &str, payload: T) -> Result<(), MatcherError>
    where
        T: Clone,
    {
        self.insert_named_internal(None, pattern, payload, HashMap::new())
    }

    /// Inserts a named route pattern with an associated payload for reverse URL generation.
    pub fn insert_named(
        &mut self,
        name: &str,
        pattern: &str,
        payload: T,
    ) -> Result<(), MatcherError>
    where
        T: Clone,
    {
        self.insert_named_internal(Some(name.to_string()), pattern, payload, HashMap::new())
    }

    /// Internal insertion method applying extra programmatic constraints.
    fn insert_named_internal(
        &mut self,
        name: Option<String>,
        pattern: &str,
        payload: T,
        extra_constraints: HashMap<String, Constraint>,
    ) -> Result<(), MatcherError>
    where
        T: Clone,
    {
        let mut segments = RouteParser::parse(pattern)?;

        // Apply programmatic constraints if provided
        for segment in &mut segments {
            if let Segment::Param {
                name: param_name,
                constraint,
                ..
            } = segment
                && let Some(extra) = extra_constraints.get(param_name)
            {
                *constraint = Some(extra.clone());
            }
        }

        let entry = RouteEntry {
            payload,
            pattern: pattern.to_string(),
            name: name.clone(),
            param_names: Vec::new(),
        };

        self.root.insert_segments(&segments, entry)?;

        if let Some(route_name) = name {
            self.named_routes
                .insert(route_name, (pattern.to_string(), segments));
        }

        Ok(())
    }

    /// Creates a fluent route builder for chaining constraints before registration.
    pub fn route(&mut self, pattern: &str) -> RouteBuilder<'_, T> {
        RouteBuilder {
            matcher: self,
            pattern: pattern.to_string(),
            name: None,
            constraints: HashMap::new(),
        }
    }

    /// Returns a reference to the matcher configuration.
    #[must_use]
    pub fn config(&self) -> &MatcherConfig {
        &self.config
    }

    /// Finds a matching route for the given URI path.
    ///
    /// Automatically isolates query strings (`?query`) and extracts borrowed parameter slices.
    pub fn find<'a>(&'a self, uri: &'a str) -> Result<Match<'a, T>, MatcherError> {
        let (raw_path, raw_query) = match uri.split_once('?') {
            Some((path, query)) => (path, Some(query)),
            None => (uri, None),
        };

        let trimmed_path = raw_path.trim();

        // Split into non-empty segments (collapses redundant slashes: `//` -> `/`)
        let segments: Vec<&'a str> = trimmed_path
            .split('/')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .collect();

        let mut params = Params::new();

        if let Some(entry) = self.root.lookup(
            trimmed_path,
            &segments,
            &mut params,
            self.config.case_sensitive,
        ) {
            // If trailing slashes are strictly enforced, verify trailing slash match
            if !self.config.flexible_trailing_slash {
                let path_has_trailing = trimmed_path.len() > 1 && trimmed_path.ends_with('/');
                let pattern_has_trailing = entry.pattern.len() > 1 && entry.pattern.ends_with('/');

                if path_has_trailing != pattern_has_trailing {
                    return Err(MatcherError::NotFound {
                        path: raw_path.to_string(),
                    });
                }
            }

            return Ok(Match {
                payload: &entry.payload,
                params,
                path: raw_path,
                raw_query,
                pattern: &entry.pattern,
                name: entry.name.as_deref(),
            });
        }

        Err(MatcherError::NotFound {
            path: raw_path.to_string(),
        })
    }

    /// Synthesizes a URL path from a named route and parameter key-value pairs.
    pub fn build_path(&self, name: &str, params: &[(&str, &str)]) -> Result<String, MatcherError> {
        let (pattern, segments) =
            self.named_routes
                .get(name)
                .ok_or_else(|| MatcherError::RouteNotFound {
                    name: name.to_string(),
                })?;

        let param_map: HashMap<&str, &str> = params.iter().copied().collect();
        let mut path_result = String::with_capacity(32);

        if segments.is_empty() {
            return Ok("/".to_string());
        }

        for segment in segments {
            match segment {
                Segment::Static(literal) => {
                    path_result.push('/');
                    path_result.push_str(literal);
                }
                Segment::Param {
                    name: param_name,
                    optional,
                    constraint,
                } => {
                    if let Some(&val) = param_map.get(param_name.as_str()) {
                        if let Some(rule) = constraint
                            && !rule.matches(val)
                        {
                            return Err(MatcherError::ConstraintFailed {
                                param: param_name.clone(),
                                value: val.to_string(),
                                constraint: rule.description(),
                            });
                        }
                        path_result.push('/');
                        path_result.push_str(val);
                    } else if *optional {
                        // Optional parameters can be omitted
                        continue;
                    } else {
                        return Err(MatcherError::MissingParameter {
                            name: param_name.clone(),
                            route: pattern.clone(),
                        });
                    }
                }
                Segment::Wildcard { name: wild_name } => {
                    if let Some(&val) = param_map.get(wild_name.as_str()) {
                        path_result.push('/');
                        path_result.push_str(val.trim_start_matches('/'));
                    } else {
                        return Err(MatcherError::MissingParameter {
                            name: wild_name.clone(),
                            route: pattern.clone(),
                        });
                    }
                }
            }
        }

        if path_result.is_empty() {
            path_result.push('/');
        }

        Ok(path_result)
    }
}

/// Successful match result containing the route payload, borrowed parameters, and query view.
#[derive(Debug, Clone)]
pub struct Match<'a, T> {
    /// Matched route payload.
    pub payload: &'a T,
    /// Extracted route parameters.
    pub params: Params<'a>,
    /// Matched URI path without query string.
    pub path: &'a str,
    /// Raw unparsed query string slice if present.
    pub raw_query: Option<&'a str>,
    /// Route pattern that produced this match.
    pub pattern: &'a str,
    /// Optional route name.
    pub name: Option<&'a str>,
}

impl<'a, T> Match<'a, T> {
    /// Returns the parsed query parameters view if a query string was present.
    #[must_use]
    pub fn query_params(&self) -> Option<QueryParams<'a>> {
        self.raw_query.map(QueryParams::new)
    }
}

/// Fluent builder for registering routes with programmatic constraints and names.
pub struct RouteBuilder<'a, T> {
    matcher: &'a mut Matcher<T>,
    pattern: String,
    name: Option<String>,
    constraints: HashMap<String, Constraint>,
}

impl<'a, T> RouteBuilder<'a, T>
where
    T: Clone,
{
    /// Names the route for reverse URL generation.
    #[must_use]
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Attaches a constraint to a parameter.
    #[must_use]
    pub fn where_constraint(mut self, param: impl Into<String>, constraint: Constraint) -> Self {
        self.constraints.insert(param.into(), constraint);
        self
    }

    /// Constrains a parameter to numeric digits only (`^[0-9]+$`).
    #[must_use]
    pub fn where_num(self, param: impl Into<String>) -> Self {
        self.where_constraint(param, Constraint::num())
    }

    /// Constrains a parameter to valid UUID strings.
    #[must_use]
    pub fn where_uuid(self, param: impl Into<String>) -> Self {
        self.where_constraint(param, Constraint::uuid())
    }

    /// Constrains a parameter to valid URL slug strings.
    #[must_use]
    pub fn where_slug(self, param: impl Into<String>) -> Self {
        self.where_constraint(param, Constraint::slug())
    }

    /// Constrains a parameter to alphabetic characters (`^[a-zA-Z]+$`).
    #[must_use]
    pub fn where_alpha(self, param: impl Into<String>) -> Self {
        self.where_constraint(param, Constraint::alpha())
    }

    /// Constrains a parameter to alphanumeric characters (`^[a-zA-Z0-9]+$`).
    #[must_use]
    pub fn where_alphanumeric(self, param: impl Into<String>) -> Self {
        self.where_constraint(param, Constraint::alphanumeric())
    }

    /// Constrains a parameter with a regular expression pattern.
    #[cfg(feature = "regex")]
    pub fn where_regex(
        self,
        param: impl Into<String>,
        pattern: &str,
    ) -> Result<Self, MatcherError> {
        let constraint = Constraint::regex(pattern)?;
        Ok(self.where_constraint(param, constraint))
    }

    /// Constrains a parameter with a custom predicate function.
    pub fn where_custom<F>(
        self,
        param: impl Into<String>,
        name: impl Into<String>,
        predicate: F,
    ) -> Self
    where
        F: Fn(&str) -> bool + Send + Sync + 'static,
    {
        let constraint = Constraint::custom(name, predicate);
        self.where_constraint(param, constraint)
    }

    /// Completes route registration by associating a payload.
    pub fn handler(self, payload: T) -> Result<(), MatcherError> {
        self.matcher
            .insert_named_internal(self.name, &self.pattern, payload, self.constraints)
    }
}
