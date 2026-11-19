//! Topic matching and wildcard pattern evaluation for dot-delimited namespaces.

/// Checks whether a dot-delimited topic matches a pattern containing single-segment (`*`)
/// or multi-segment recursive (`**`) wildcards.
///
/// # Rules
///
/// 1. Segments are delimited strictly by the dot (`.`) character.
/// 2. Literal segments must match verbatim (case-sensitive).
/// 3. A single asterisk (`*`) matches exactly one segment at that position.
/// 4. A double asterisk (`**`) matches zero or more segments recursively.
/// 5. Global pattern `*` matches any single-segment topic, while `**` matches all topics.
pub fn matches_topic(pattern: &str, topic: &str) -> bool {
    if pattern == "**" {
        return true;
    }
    if pattern == topic {
        return true;
    }

    let pattern_segments: Vec<&str> = pattern.split('.').collect();
    let topic_segments: Vec<&str> = topic.split('.').collect();

    match_segments(&pattern_segments, &topic_segments)
}

fn match_segments(pattern_segments: &[&str], topic_segments: &[&str]) -> bool {
    let mut p_idx = 0;
    let mut t_idx = 0;

    while p_idx < pattern_segments.len() && t_idx < topic_segments.len() {
        let pattern_seg = pattern_segments[p_idx];

        if pattern_seg == "**" {
            if p_idx + 1 == pattern_segments.len() {
                // Trailing recursive wildcard matches all remaining topic segments.
                return true;
            }

            // Attempt to match the remainder of pattern against all suffixes of topic segments.
            for next_t in t_idx..=topic_segments.len() {
                if match_segments(&pattern_segments[p_idx + 1..], &topic_segments[next_t..]) {
                    return true;
                }
            }
            return false;
        }

        if pattern_seg == "*" || pattern_seg == topic_segments[t_idx] {
            p_idx += 1;
            t_idx += 1;
        } else {
            return false;
        }
    }

    // Skip any trailing "**" segments in the pattern.
    while p_idx < pattern_segments.len() && pattern_segments[p_idx] == "**" {
        p_idx += 1;
    }

    p_idx == pattern_segments.len() && t_idx == topic_segments.len()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exact_matches() {
        assert!(matches_topic("order.created", "order.created"));
        assert!(!matches_topic("order.created", "order.cancelled"));
        assert!(!matches_topic("order.created", "order"));
        assert!(!matches_topic("order", "order.created"));
    }

    #[test]
    fn test_single_segment_wildcard() {
        assert!(matches_topic("order.*", "order.created"));
        assert!(matches_topic("order.*", "order.cancelled"));
        assert!(!matches_topic("order.*", "order.item.created"));
        assert!(!matches_topic("order.*", "order"));

        assert!(matches_topic("*.created", "order.created"));
        assert!(matches_topic("*.created", "user.created"));
        assert!(!matches_topic("*.created", "order.item.created"));

        assert!(matches_topic("order.*.created", "order.item.created"));
        assert!(!matches_topic(
            "order.*.created",
            "order.item.variant.created"
        ));
    }

    #[test]
    fn test_recursive_multi_segment_wildcard() {
        assert!(matches_topic("order.**", "order.created"));
        assert!(matches_topic("order.**", "order.item.created"));
        assert!(matches_topic("order.**", "order.item.variant.created"));
        assert!(matches_topic("order.**", "order"));
        assert!(!matches_topic("order.**", "user.created"));

        assert!(matches_topic("**.created", "created"));
        assert!(matches_topic("**.created", "order.created"));
        assert!(matches_topic("**.created", "order.item.created"));
        assert!(!matches_topic("**.created", "order.item.updated"));

        assert!(matches_topic("order.**.created", "order.created"));
        assert!(matches_topic("order.**.created", "order.item.created"));
        assert!(matches_topic(
            "order.**.created",
            "order.item.variant.created"
        ));
    }

    #[test]
    fn test_global_wildcards() {
        assert!(matches_topic("**", "order.created"));
        assert!(matches_topic("**", "anything"));
        assert!(matches_topic("*", "order"));
        assert!(!matches_topic("*", "order.created"));
    }
}
