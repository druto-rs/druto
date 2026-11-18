//! Parsing and compilation of route pattern strings into structured segment descriptors.

use std::collections::HashSet;

use crate::{constraint::Constraint, error::MatcherError, segment::Segment};

/// Parser converting route pattern strings into a vector of compiled [`Segment`]s.
pub struct RouteParser;

impl RouteParser {
    /// Parses a path pattern string into a sequence of [`Segment`]s.
    ///
    /// Normalizes redundant slashes and verifies pattern integrity.
    pub fn parse(pattern: &str) -> Result<Vec<Segment>, MatcherError> {
        let trimmed = pattern.trim();
        if trimmed.is_empty() || trimmed == "/" {
            return Ok(Vec::new());
        }

        let mut segments = Vec::new();
        let mut seen_params = HashSet::new();
        let mut has_optional = false;

        let raw_tokens = Self::split_path_segments(trimmed);
        let token_count = raw_tokens.len();

        for (idx, token) in raw_tokens.into_iter().enumerate() {
            let is_last = idx + 1 == token_count;
            let segment = Self::parse_token(token, pattern)?;

            match &segment {
                Segment::Wildcard { .. } => {
                    if !is_last {
                        return Err(MatcherError::InvalidPattern {
                            pattern: pattern.to_string(),
                            reason: "wildcard segment must be the final segment in a pattern"
                                .to_string(),
                        });
                    }
                }
                Segment::Param {
                    name,
                    optional,
                    constraint: _,
                } => {
                    if !seen_params.insert(name.clone()) {
                        return Err(MatcherError::InvalidPattern {
                            pattern: pattern.to_string(),
                            reason: format!("duplicate parameter name `{name}` in route pattern"),
                        });
                    }

                    if has_optional && !optional {
                        return Err(MatcherError::InvalidPattern {
                            pattern: pattern.to_string(),
                            reason: "required parameter cannot follow an optional parameter"
                                .to_string(),
                        });
                    }

                    if *optional {
                        has_optional = true;
                    }
                }
                Segment::Static(_) => {
                    if has_optional {
                        return Err(MatcherError::InvalidPattern {
                            pattern: pattern.to_string(),
                            reason: "static segment cannot follow an optional parameter"
                                .to_string(),
                        });
                    }
                }
            }

            segments.push(segment);
        }

        Ok(segments)
    }

    /// Splits path into tokens while preserving slashes inside parentheses and braces.
    fn split_path_segments(path: &str) -> Vec<&str> {
        let mut segments = Vec::new();
        let mut depth_paren = 0usize;
        let mut depth_brace = 0usize;
        let mut start = 0;

        for (idx, ch) in path.char_indices() {
            match ch {
                '(' => depth_paren += 1,
                ')' => depth_paren = depth_paren.saturating_sub(1),
                '{' => depth_brace += 1,
                '}' => depth_brace = depth_brace.saturating_sub(1),
                '/' if depth_paren == 0 && depth_brace == 0 => {
                    let seg = path[start..idx].trim();
                    if !seg.is_empty() {
                        segments.push(seg);
                    }
                    start = idx + 1;
                }
                _ => {}
            }
        }

        let tail = path[start..].trim();
        if !tail.is_empty() {
            segments.push(tail);
        }

        segments
    }

    /// Parses a single path segment token into a [`Segment`].
    fn parse_token(token: &str, full_pattern: &str) -> Result<Segment, MatcherError> {
        // Catch-all wildcard checks: `*path`, `**`, or `{path*}`
        if token == "**" {
            return Ok(Segment::wildcard("path"));
        }

        if let Some(wild_name) = token.strip_prefix('*') {
            let name = if wild_name.is_empty() {
                "wildcard"
            } else {
                wild_name
            };
            return Ok(Segment::wildcard(name));
        }

        if token.starts_with('{') && token.ends_with("*}") {
            let inner = &token[1..token.len() - 2];
            let name = if inner.is_empty() { "wildcard" } else { inner };
            return Ok(Segment::wildcard(name));
        }

        // Colon-style parameter: `:id`, `:id?`, `:id(num)`, `:id(uuid)?`, etc.
        if let Some(param_content) = token.strip_prefix(':') {
            return Self::parse_colon_param(param_content, token, full_pattern);
        }

        // Brace-style parameter: `{id}`, `{id?}`, `{id:num}`, `{id:[0-9]+}`
        if token.starts_with('{') && token.ends_with('}') {
            let inner = &token[1..token.len() - 1];
            return Self::parse_brace_param(inner, token, full_pattern);
        }

        // Unmatched braces or parentheses validation
        if token.contains('{') || token.contains('}') || token.contains('(') || token.contains(')')
        {
            return Err(MatcherError::InvalidPattern {
                pattern: full_pattern.to_string(),
                reason: format!("malformed segment syntax in `{token}`"),
            });
        }

        Ok(Segment::static_segment(token))
    }

    /// Parses colon parameter expressions like `:id`, `:id?`, `:id(num)`, `:id(num)?`.
    fn parse_colon_param(
        content: &str,
        raw_token: &str,
        pattern: &str,
    ) -> Result<Segment, MatcherError> {
        if content.is_empty() {
            return Err(MatcherError::InvalidPattern {
                pattern: pattern.to_string(),
                reason: "parameter name cannot be empty".to_string(),
            });
        }

        let (without_opt, optional) = if let Some(stripped) = content.strip_suffix('?') {
            (stripped, true)
        } else {
            (content, false)
        };

        // Check for inline constraint `:name(constraint)`
        if let Some(paren_start) = without_opt.find('(') {
            if !without_opt.ends_with(')') {
                return Err(MatcherError::InvalidPattern {
                    pattern: pattern.to_string(),
                    reason: format!("unclosed constraint parenthesis in `{raw_token}`"),
                });
            }

            let name = &without_opt[..paren_start];
            let constraint_str = &without_opt[paren_start + 1..without_opt.len() - 1];

            if name.is_empty() {
                return Err(MatcherError::InvalidPattern {
                    pattern: pattern.to_string(),
                    reason: format!("missing parameter name before constraint in `{raw_token}`"),
                });
            }

            let constraint = Self::parse_constraint_string(constraint_str, pattern)?;
            return Ok(Segment::constrained_param(name, optional, constraint));
        }

        if without_opt.is_empty() {
            return Err(MatcherError::InvalidPattern {
                pattern: pattern.to_string(),
                reason: "parameter name cannot be empty".to_string(),
            });
        }

        Ok(Segment::Param {
            name: without_opt.to_string(),
            optional,
            constraint: None,
        })
    }

    /// Parses brace parameter expressions like `{id}`, `{id?}`, `{id:num}`, `{id:[0-9]+}`.
    fn parse_brace_param(
        inner: &str,
        raw_token: &str,
        pattern: &str,
    ) -> Result<Segment, MatcherError> {
        if inner.is_empty() {
            return Err(MatcherError::InvalidPattern {
                pattern: pattern.to_string(),
                reason: "empty parameter braces `{}`".to_string(),
            });
        }

        let (without_opt, optional) = if let Some(stripped) = inner.strip_suffix('?') {
            (stripped, true)
        } else {
            (inner, false)
        };

        if let Some((name, constraint_str)) = without_opt.split_once(':') {
            if name.is_empty() {
                return Err(MatcherError::InvalidPattern {
                    pattern: pattern.to_string(),
                    reason: format!("missing parameter name before `:` in `{raw_token}`"),
                });
            }

            let constraint = Self::parse_constraint_string(constraint_str, pattern)?;
            return Ok(Segment::constrained_param(name, optional, constraint));
        }

        Ok(Segment::Param {
            name: without_opt.to_string(),
            optional,
            constraint: None,
        })
    }

    /// Parses shorthand or regex constraint strings.
    fn parse_constraint_string(
        constraint_str: &str,
        pattern: &str,
    ) -> Result<Constraint, MatcherError> {
        let trimmed = constraint_str.trim();

        match trimmed {
            "num" | "int" | "integer" | "number" => Ok(Constraint::num()),
            "uuid" => Ok(Constraint::uuid()),
            "slug" => Ok(Constraint::slug()),
            "alpha" => Ok(Constraint::alpha()),
            "alphanumeric" | "alphanum" => Ok(Constraint::alphanumeric()),
            other => {
                #[cfg(feature = "regex")]
                {
                    // Clean optional surrounding regex delimiters `/.../`
                    let regex_source =
                        if other.starts_with('/') && other.ends_with('/') && other.len() >= 2 {
                            &other[1..other.len() - 1]
                        } else {
                            other
                        };

                    Constraint::regex(regex_source).map_err(|err| MatcherError::InvalidPattern {
                        pattern: pattern.to_string(),
                        reason: err.to_string(),
                    })
                }
                #[cfg(not(feature = "regex"))]
                {
                    Err(MatcherError::InvalidPattern {
                        pattern: pattern.to_string(),
                        reason: format!(
                            "unrecognized constraint `{other}`; enable `regex` feature for custom patterns"
                        ),
                    })
                }
            }
        }
    }
}
