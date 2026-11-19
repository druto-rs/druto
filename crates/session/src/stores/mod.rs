//! Built-in session storage drivers.

pub mod cookie;
pub mod file;
pub mod memory;

pub use cookie::{CookieSessionStore, MAX_COOKIE_PAYLOAD_SIZE, SESSION_COOKIE_PURPOSE};
pub use file::FileSessionStore;
pub use memory::MemorySessionStore;
