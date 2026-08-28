//! The app's own share of the endpoint: the identity, the session brief,
//! the icons, and the per-call binding. Each one describes or spans the
//! whole surface, which no single feature can see.

mod context;
mod guard;
mod icons;
mod instructions;
mod module;

pub use guard::CallTrail;
pub use icons::icons;
pub use instructions::instructions;
pub use module::McpModule;
