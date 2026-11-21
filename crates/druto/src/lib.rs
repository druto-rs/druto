//! Top-level umbrella facade for the Druto web framework.
//!
//! Provides the primary application builder, routing engine, runtime-agnostic HTTP/1.1 server,
//! in-memory test harness, and unified prelude.

pub mod error;
pub mod server;
pub mod test_server;

pub use barta;
pub use borno;
pub use dhap;
pub use error::ServerError;
pub use gopon;
pub use jachai;
pub use khonj;
pub use kouta;
pub use mishron;
pub use poribesh;
pub use poth;
pub use router;
pub use sebok;
pub use server::{IntoCompiledRouter, Server, dispatch_http_request};
pub use test_server::{TestRequestBuilder, TestResponse, TestServer};

/// Essential types and traits for building Druto web applications.
pub mod prelude {
    pub use router::prelude::*;
    pub use sebok::{RequestBody, ResponseBody, ServerConfig};

    pub use crate::{
        ServerError,
        server::{IntoCompiledRouter, Server},
        test_server::{TestRequestBuilder, TestResponse, TestServer},
    };
}
