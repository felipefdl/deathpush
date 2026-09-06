pub mod apply;
pub mod policy;
pub mod registry;
pub mod types;

pub use registry::SessionRegistry;

/// One UI session, one per window. Allocated by `Core::open_session`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct SessionId(pub u64);
