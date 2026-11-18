//! Segment-based radix trie indexing and deterministic route lookup.

use std::collections::HashMap;

use crate::{constraint::Constraint, error::MatcherError, params::Params, segment::Segment};

/// Internal route target stored at leaf nodes.
#[derive(Clone, Debug)]
pub struct RouteEntry<T> {
    /// Associated handler or payload.
    pub payload: T,
    /// Original route pattern.
    pub pattern: String,
    /// Optional name for reverse routing.
    pub name: Option<String>,
    /// Route parameter names for this endpoint.
    pub param_names: Vec<String>,
}

/// A node within the route trie.
#[derive(Clone, Debug)]
pub struct RouteNode<T> {
    /// Exact static segment children, e.g. `users`, `api`.
    static_children: HashMap<String, RouteNode<T>>,
    /// Constrained dynamic children sorted by constraint specificity.
    constrained_children: Vec<ConstrainedChild<T>>,
    /// Unconstrained dynamic parameter child, e.g. `:id`.
    param_child: Option<ParamChild<T>>,
    /// Wildcard catch-all child, e.g. `*path`.
    wildcard_child: Option<WildcardChild<T>>,
    /// Stored route payload if this node is an endpoint.
    entry: Option<RouteEntry<T>>,
}

#[derive(Clone, Debug)]
struct ConstrainedChild<T> {
    name: String,
    constraint: Constraint,
    node: RouteNode<T>,
}

#[derive(Clone, Debug)]
struct ParamChild<T> {
    name: String,
    node: Box<RouteNode<T>>,
}

#[derive(Clone, Debug)]
struct WildcardChild<T> {
    name: String,
    entry: RouteEntry<T>,
}

impl<T> Default for RouteNode<T> {
    fn default() -> Self {
        Self {
            static_children: HashMap::new(),
            constrained_children: Vec::new(),
            param_child: None,
            wildcard_child: None,
            entry: None,
        }
    }
}

impl<T> RouteNode<T> {
    /// Creates an empty trie node.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Inserts a compiled sequence of segments into the trie.
    pub fn insert_segments(
        &mut self,
        segments: &[Segment],
        entry: RouteEntry<T>,
    ) -> Result<(), MatcherError>
    where
        T: Clone,
    {
        let mut active_param_names = Vec::new();
        self.insert_recursive(segments, 0, entry, &mut active_param_names)
    }

    fn insert_recursive(
        &mut self,
        segments: &[Segment],
        index: usize,
        mut entry: RouteEntry<T>,
        active_param_names: &mut Vec<String>,
    ) -> Result<(), MatcherError>
    where
        T: Clone,
    {
        if index >= segments.len() {
            if let Some(existing) = &self.entry {
                return Err(MatcherError::ConflictingRoute {
                    existing: existing.pattern.clone(),
                    new_pattern: entry.pattern.clone(),
                    reason: "a route with identical path specificity is already registered"
                        .to_string(),
                });
            }
            entry.param_names = active_param_names.clone();
            self.entry = Some(entry);
            return Ok(());
        }

        let segment = &segments[index];

        match segment {
            Segment::Static(literal) => {
                let child = self.static_children.entry(literal.clone()).or_default();
                child.insert_recursive(segments, index + 1, entry, active_param_names)
            }
            Segment::Param {
                name,
                optional,
                constraint,
            } => {
                // If optional, register an endpoint at this exact level (without this param)
                if *optional {
                    let mut optional_entry = entry.clone();
                    optional_entry.param_names = active_param_names.clone();
                    self.insert_recursive(
                        &segments[index + 1..],
                        0,
                        optional_entry,
                        active_param_names,
                    )?;
                }

                active_param_names.push(name.clone());

                let result = if let Some(rule) = constraint {
                    if let Some(pos) = self
                        .constrained_children
                        .iter()
                        .position(|c| c.constraint == *rule)
                    {
                        self.constrained_children[pos].node.insert_recursive(
                            segments,
                            index + 1,
                            entry,
                            active_param_names,
                        )
                    } else {
                        let mut new_node = RouteNode::new();
                        new_node.insert_recursive(
                            segments,
                            index + 1,
                            entry,
                            active_param_names,
                        )?;

                        let child = ConstrainedChild {
                            name: name.clone(),
                            constraint: rule.clone(),
                            node: new_node,
                        };

                        let insert_idx = self.constrained_children.partition_point(|existing| {
                            existing.constraint.precedence_rank()
                                >= child.constraint.precedence_rank()
                        });

                        self.constrained_children.insert(insert_idx, child);
                        Ok(())
                    }
                } else if let Some(ref mut param) = self.param_child {
                    param
                        .node
                        .insert_recursive(segments, index + 1, entry, active_param_names)
                } else {
                    let mut new_node = RouteNode::new();
                    new_node.insert_recursive(segments, index + 1, entry, active_param_names)?;
                    self.param_child = Some(ParamChild {
                        name: name.clone(),
                        node: Box::new(new_node),
                    });
                    Ok(())
                };

                active_param_names.pop();
                result
            }
            Segment::Wildcard { name } => {
                if let Some(existing) = &self.wildcard_child {
                    return Err(MatcherError::ConflictingRoute {
                        existing: existing.entry.pattern.clone(),
                        new_pattern: entry.pattern.clone(),
                        reason: "wildcard route already exists at this node".to_string(),
                    });
                }
                active_param_names.push(name.clone());
                entry.param_names = active_param_names.clone();
                active_param_names.pop();

                self.wildcard_child = Some(WildcardChild {
                    name: name.clone(),
                    entry,
                });
                Ok(())
            }
        }
    }

    /// Matches a sequence of path segments against the trie according to strict precedence:
    /// Static > Constrained Param > Dynamic Param > Wildcard.
    pub fn lookup<'a>(
        &'a self,
        raw_path: &'a str,
        segments: &[&'a str],
        params: &mut Params<'a>,
        case_sensitive: bool,
    ) -> Option<&'a RouteEntry<T>> {
        let entry = self.lookup_recursive(raw_path, segments, 0, params, case_sensitive)?;
        params.apply_param_names(&entry.param_names);
        Some(entry)
    }

    fn lookup_recursive<'a>(
        &'a self,
        raw_path: &'a str,
        segments: &[&'a str],
        index: usize,
        params: &mut Params<'a>,
        case_sensitive: bool,
    ) -> Option<&'a RouteEntry<T>> {
        let saved_params_len = params.len();

        // Terminal node check
        if index >= segments.len() {
            if let Some(entry) = &self.entry {
                return Some(entry);
            }
            if let Some(wildcard) = &self.wildcard_child {
                params.push(&wildcard.name, "");
                return Some(&wildcard.entry);
            }
            return None;
        }

        let segment = segments[index];

        // 1. Static match (highest priority)
        if case_sensitive {
            if let Some(child) = self.static_children.get(segment) {
                if let Some(result) =
                    child.lookup_recursive(raw_path, segments, index + 1, params, case_sensitive)
                {
                    return Some(result);
                }
                params.truncate(saved_params_len);
            }
        } else {
            for (literal, child) in &self.static_children {
                if literal.eq_ignore_ascii_case(segment) {
                    if let Some(result) = child.lookup_recursive(
                        raw_path,
                        segments,
                        index + 1,
                        params,
                        case_sensitive,
                    ) {
                        return Some(result);
                    }
                    params.truncate(saved_params_len);
                }
            }
        }

        // 2. Constrained dynamic parameters (in order of constraint specificity)
        for constrained in &self.constrained_children {
            if constrained.constraint.matches(segment) {
                params.push(&constrained.name, segment);
                if let Some(result) = constrained.node.lookup_recursive(
                    raw_path,
                    segments,
                    index + 1,
                    params,
                    case_sensitive,
                ) {
                    return Some(result);
                }
                params.truncate(saved_params_len);
            }
        }

        // 3. Unconstrained dynamic parameter
        if let Some(param) = &self.param_child {
            params.push(&param.name, segment);
            if let Some(result) =
                param
                    .node
                    .lookup_recursive(raw_path, segments, index + 1, params, case_sensitive)
            {
                return Some(result);
            }
            params.truncate(saved_params_len);
        }

        // 4. Wildcard catch-all (matches the current segment and all remaining segments)
        if let Some(wildcard) = &self.wildcard_child {
            let remainder = &segments[index..];
            let first_seg = remainder[0];
            let last_seg = remainder[remainder.len() - 1];

            let start_offset = (first_seg.as_ptr() as usize) - (raw_path.as_ptr() as usize);
            let end_offset =
                (last_seg.as_ptr() as usize + last_seg.len()) - (raw_path.as_ptr() as usize);

            let wildcard_slice = raw_path.get(start_offset..end_offset).unwrap_or(first_seg);
            params.push(&wildcard.name, wildcard_slice);
            return Some(&wildcard.entry);
        }

        None
    }
}
