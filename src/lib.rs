//! Top-level umbrella facade for the Druto web framework.
//!
//! Provides the primary application builder, routing engine, runtime-agnostic HTTP/1.1 server,
//! in-memory test harness, and modular feature-flagged facade submodules.

pub mod cli;
pub mod error;
pub mod server;
pub mod test_server;

// Core HTTP and routing primitives (always available)
#[cfg(feature = "events")]
pub use barta as events;
// Modular facade submodules
#[cfg(feature = "bodyparser")]
pub use bodyparser;
#[cfg(feature = "string")]
pub use borno as string;
#[cfg(feature = "cors")]
pub use cors;
pub use dhap;
pub use error::ServerError;
#[cfg(feature = "crypto")]
pub use gopon as crypto;
pub use ignitor;
#[cfg(feature = "validation")]
pub use jachai as validation;
pub use khonj;
pub use kouta;
#[cfg(feature = "limiter")]
pub use limiter;
#[cfg(feature = "hash")]
pub use mishron as hash;
#[cfg(feature = "env")]
pub use poribesh as env;
pub use poth;
#[cfg(feature = "redis")]
pub use redis;
pub use router;
pub use sebok;
pub use server::{IntoCompiledRouter, Server, dispatch_http_request};
#[cfg(feature = "session")]
pub use session;
#[cfg(feature = "shield")]
pub use shield;
pub use test_server::{TestRequestBuilder, TestResponse, TestServer};

/// Essential types and traits for building Druto web applications.
pub mod prelude {
    #[cfg(feature = "events")]
    pub use barta::{Dispatcher, Event};
    #[cfg(feature = "bodyparser")]
    pub use bodyparser::{BodyParserConfig, BodyParserMiddleware};
    #[cfg(feature = "string")]
    pub use borno::StrExt;
    #[cfg(feature = "cors")]
    pub use cors::{CorsConfig, CorsConfigBuilder, CorsMiddleware, OriginPolicy};
    #[cfg(feature = "crypto")]
    pub use gopon::{Encrypter, HmacSigner, Key, KeyRing, UrlSigner};
    pub use ignitor::{Ignitor, start};
    #[cfg(feature = "limiter")]
    pub use limiter::{LimiterConfig, LimiterConfigBuilder, LimiterMiddleware};
    #[cfg(feature = "hash")]
    pub use mishron::{HashManager, Hasher};
    #[cfg(feature = "env")]
    pub use poribesh::Env;
    #[cfg(feature = "redis")]
    pub use redis::{RedisConfig, RedisConnection, RedisManager};
    pub use router::prelude::*;
    pub use sebok::{RequestBody, ResponseBody, ServerConfig};
    #[cfg(feature = "session")]
    pub use session::{Session, SessionConfig, SessionManager, SessionMiddleware};
    #[cfg(feature = "shield")]
    pub use shield::{CspConfig, CsrfConfig, SecurityPreset, ShieldConfig, ShieldMiddleware};

    pub use crate::{
        ServerError,
        server::{IntoCompiledRouter, Server},
        test_server::{TestRequestBuilder, TestResponse, TestServer},
    };
}
