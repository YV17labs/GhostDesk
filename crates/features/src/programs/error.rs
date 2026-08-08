//! What launching or inspecting a program can go wrong with.
//!
//! The split that matters is not by message but by *whose mistake it is*: the
//! first five variants are the caller asking for something the server will
//! not do, and an agent told so can correct itself and retry. The last three
//! are the server failing, where retrying the same call is pointless. The
//! transport reads that distinction off the variant, which is the whole
//! reason this is an enum and not a string.

use std::path::PathBuf;

/// A refusal or a failure from the programs domain.
#[derive(Debug, thiserror::Error)]
pub enum ProgramsError {
    /// Quoting the caller sent that no shell could parse.
    #[error("Invalid command syntax: {0:?}")]
    Syntax(String),

    #[error("No command provided")]
    Empty,

    /// Arguments would turn the catalogue whitelist into arbitrary command
    /// execution (CWE-78), so they are refused before anything is spawned.
    #[error(
        "Arguments are not allowed — pass only the executable name from \
         app_list(). Got: {0:?}"
    )]
    Arguments(String),

    #[error("{0:?} is not a known GUI app. Call app_list() to see what is available.")]
    Unknown(String),

    /// `app_status` answers only for PIDs this session started, so the tool
    /// cannot be turned into a general-purpose process prober.
    #[error("PID {0} was not launched by this session. Use app_launch() first.")]
    Untracked(u32),

    #[error("Command not found: {} ({source})", program.display())]
    Spawn {
        program: PathBuf,
        source: std::io::Error,
    },

    #[error("the launched process exited before it was tracked")]
    Vanished,

    /// The log file could not be created, staged or renamed.
    #[error("could not set up the log file for a launched program")]
    Log(#[source] std::io::Error),

    /// The desktop could not be asked what it has open.
    #[error("the desktop could not be queried")]
    Desktop(#[source] anyhow::Error),
}
