//! DeathPush core: git, sessions, watcher, terminal state. No UI dependency.

pub mod content_hash;
pub mod error;
pub mod events;
pub mod git;
pub mod pty;
pub mod session;
pub mod shell_env;
pub mod terminal;
pub mod types;
pub mod util;

pub use error::{Error, Result};
pub use events::{CoreEvent, EventHub};
pub use session::SessionId;
