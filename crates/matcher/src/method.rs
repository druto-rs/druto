//! Comprehensive HTTP methods supporting RFC 9110, RFC 10008 (QUERY), WebDAV, and custom extensions.

use std::{fmt, str::FromStr};

/// Represents an HTTP request method.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Method {
    /// RFC 9110 `GET` method.
    Get,
    /// RFC 9110 `POST` method.
    Post,
    /// RFC 9110 `PUT` method.
    Put,
    /// RFC 9110 `DELETE` method.
    Delete,
    /// RFC 5789 `PATCH` method.
    Patch,
    /// RFC 9110 `HEAD` method.
    Head,
    /// RFC 9110 `OPTIONS` method.
    Options,
    /// RFC 9110 `CONNECT` method.
    Connect,
    /// RFC 9110 `TRACE` method.
    Trace,
    /// RFC 10008 `QUERY` method (safe and idempotent read request with a payload body).
    Query,
    /// Cache/CDN invalidation `PURGE` method.
    Purge,
    /// WebDAV RFC 4918 `PROPFIND` method.
    Propfind,
    /// WebDAV RFC 4918 `PROPPATCH` method.
    Proppatch,
    /// WebDAV RFC 4918 `MKCOL` method.
    Mkcol,
    /// WebDAV RFC 4918 `COPY` method.
    Copy,
    /// WebDAV RFC 4918 `MOVE` method.
    Move,
    /// WebDAV RFC 4918 `LOCK` method.
    Lock,
    /// WebDAV RFC 4918 `UNLOCK` method.
    Unlock,
    /// RFC 3253 / RFC 4791 `REPORT` method (CalDAV/CardDAV/versioning queries).
    Report,
    /// RFC 4791 `MKCALENDAR` method (CalDAV calendar creation).
    Mkcalendar,
    /// RFC 5323 `SEARCH` method (WebDAV search / DASL query).
    Search,
    /// RFC 3744 `ACL` method (WebDAV access control list modification).
    Acl,
    /// Cache invalidation `BAN` method.
    Ban,
    /// Wildcard accepting any HTTP method.
    Any,
    /// Any arbitrary custom HTTP extension method.
    Custom(String),
}

impl Method {
    /// Returns the standard uppercase string representation of the HTTP method.
    #[must_use]
    pub fn as_str(&self) -> &str {
        match self {
            Self::Get => "GET",
            Self::Post => "POST",
            Self::Put => "PUT",
            Self::Delete => "DELETE",
            Self::Patch => "PATCH",
            Self::Head => "HEAD",
            Self::Options => "OPTIONS",
            Self::Connect => "CONNECT",
            Self::Trace => "TRACE",
            Self::Query => "QUERY",
            Self::Purge => "PURGE",
            Self::Propfind => "PROPFIND",
            Self::Proppatch => "PROPPATCH",
            Self::Mkcol => "MKCOL",
            Self::Copy => "COPY",
            Self::Move => "MOVE",
            Self::Lock => "LOCK",
            Self::Unlock => "UNLOCK",
            Self::Report => "REPORT",
            Self::Mkcalendar => "MKCALENDAR",
            Self::Search => "SEARCH",
            Self::Acl => "ACL",
            Self::Ban => "BAN",
            Self::Any => "*",
            Self::Custom(custom) => custom.as_str(),
        }
    }

    /// Fast zero-copy parsing from raw ASCII byte slices.
    #[must_use]
    pub fn from_bytes(bytes: &[u8]) -> Self {
        if bytes.eq_ignore_ascii_case(b"GET") {
            Self::Get
        } else if bytes.eq_ignore_ascii_case(b"POST") {
            Self::Post
        } else if bytes.eq_ignore_ascii_case(b"PUT") {
            Self::Put
        } else if bytes.eq_ignore_ascii_case(b"DELETE") {
            Self::Delete
        } else if bytes.eq_ignore_ascii_case(b"PATCH") {
            Self::Patch
        } else if bytes.eq_ignore_ascii_case(b"HEAD") {
            Self::Head
        } else if bytes.eq_ignore_ascii_case(b"OPTIONS") {
            Self::Options
        } else if bytes.eq_ignore_ascii_case(b"CONNECT") {
            Self::Connect
        } else if bytes.eq_ignore_ascii_case(b"TRACE") {
            Self::Trace
        } else if bytes.eq_ignore_ascii_case(b"QUERY") {
            Self::Query
        } else if bytes.eq_ignore_ascii_case(b"PURGE") {
            Self::Purge
        } else if bytes.eq_ignore_ascii_case(b"PROPFIND") {
            Self::Propfind
        } else if bytes.eq_ignore_ascii_case(b"PROPPATCH") {
            Self::Proppatch
        } else if bytes.eq_ignore_ascii_case(b"MKCOL") {
            Self::Mkcol
        } else if bytes.eq_ignore_ascii_case(b"COPY") {
            Self::Copy
        } else if bytes.eq_ignore_ascii_case(b"MOVE") {
            Self::Move
        } else if bytes.eq_ignore_ascii_case(b"LOCK") {
            Self::Lock
        } else if bytes.eq_ignore_ascii_case(b"UNLOCK") {
            Self::Unlock
        } else if bytes.eq_ignore_ascii_case(b"REPORT") {
            Self::Report
        } else if bytes.eq_ignore_ascii_case(b"MKCALENDAR") {
            Self::Mkcalendar
        } else if bytes.eq_ignore_ascii_case(b"SEARCH") {
            Self::Search
        } else if bytes.eq_ignore_ascii_case(b"ACL") {
            Self::Acl
        } else if bytes.eq_ignore_ascii_case(b"BAN") {
            Self::Ban
        } else if bytes == b"*" || bytes.eq_ignore_ascii_case(b"ANY") {
            Self::Any
        } else {
            let normalized = String::from_utf8_lossy(bytes).to_ascii_uppercase();
            Self::Custom(normalized)
        }
    }

    /// Returns whether this HTTP request method is safe according to RFC 9110 §9.2.1.
    ///
    /// Safe methods are essentially read-only operations that do not produce side effects
    /// on the server resource state.
    #[must_use]
    pub fn is_safe(&self) -> bool {
        matches!(
            self,
            Self::Get
                | Self::Head
                | Self::Options
                | Self::Trace
                | Self::Query
                | Self::Propfind
                | Self::Report
                | Self::Search
        )
    }

    /// Returns whether this HTTP request method is idempotent according to RFC 9110 §9.2.2.
    ///
    /// Idempotent methods can be called repeatedly with the same intended effect as a single request.
    #[must_use]
    pub fn is_idempotent(&self) -> bool {
        matches!(
            self,
            Self::Get
                | Self::Head
                | Self::Put
                | Self::Delete
                | Self::Options
                | Self::Trace
                | Self::Query
                | Self::Propfind
                | Self::Mkcol
                | Self::Unlock
                | Self::Report
                | Self::Search
                | Self::Acl
                | Self::Purge
                | Self::Ban
        )
    }

    /// Checks if this method matches the given request method.
    ///
    /// Wildcard [`Method::Any`] matches any method.
    #[must_use]
    pub fn matches(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Any, _) | (_, Self::Any) => true,
            _ => self == other,
        }
    }
}

impl FromStr for Method {
    type Err = std::convert::Infallible;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        Ok(Self::from_bytes(input.as_bytes()))
    }
}

impl AsRef<str> for Method {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl From<&str> for Method {
    fn from(input: &str) -> Self {
        Self::from_bytes(input.as_bytes())
    }
}

impl From<String> for Method {
    fn from(input: String) -> Self {
        Self::from(input.as_str())
    }
}

impl fmt::Display for Method {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}
