//! Whose mistake a failure is, and the single answer that follows from it.
//!
//! Each module's `error.rs` already decides this per variant; what lives here
//! is the one rendering of that decision at the MCP edge. Crate-root rather
//! than per-adapter for the reason the framework's own `Opaque` is not
//! per-host: choosing between a message the model reads verbatim and one it
//! must never read is a security posture, and a posture copied into each
//! adapter is a posture that will be copied wrong the day a domain is added in
//! a hurry — silently, since a copy that forgets the refusal branch still
//! compiles and still answers.

use std::fmt::Display;

use nest_rs::mcp::{McpError, Opaque};

/// Which side a failure blames — the one thing an adapter cannot read off a
/// message.
pub(crate) trait Blame {
    /// True when the caller can act on this and call again.
    fn blames_the_caller(&self) -> bool;
}

/// Render a domain failure as the answer its blame earns.
pub(crate) trait Answered<T> {
    fn answered(self) -> Result<T, McpError>;
}

impl<T, E: Blame + Display> Answered<T> for Result<T, E> {
    /// A refusal the caller can act on reaches the model verbatim, so it can
    /// correct itself and call again; everything else leaves through the
    /// framework's `opaque`, which keeps the real error for the operator and
    /// hands the model a constant.
    fn answered(self) -> Result<T, McpError> {
        match self {
            Err(err) if err.blames_the_caller() => {
                Err(McpError::invalid_params(err.to_string(), None))
            }
            other => other.opaque(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, thiserror::Error)]
    #[error("{0}")]
    struct Faulted(&'static str, bool);

    impl Blame for Faulted {
        fn blames_the_caller(&self) -> bool {
            self.1
        }
    }

    #[test]
    fn a_caller_error_reaches_the_model_verbatim() {
        let err = Err::<(), _>(Faulted("pass an executable from app_list()", true))
            .answered()
            .unwrap_err();
        assert!(err.message.contains("app_list()"), "{}", err.message);
    }

    #[test]
    fn a_server_error_leaves_as_the_shared_opaque_message() {
        let err = Err::<(), _>(Faulted("/run/user/1000/wayland-1 refused", false))
            .answered()
            .unwrap_err();
        assert_eq!(err.message, nest_rs::core::OPAQUE_CLIENT_MESSAGE);
        assert!(
            !err.message.contains("wayland"),
            "no host detail reaches the model: {}",
            err.message,
        );
    }
}
