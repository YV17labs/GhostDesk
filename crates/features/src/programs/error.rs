use std::path::PathBuf;

use crate::blame::Blame;

#[derive(Debug, thiserror::Error)]
pub enum ProgramsError {
    #[error("Invalid command syntax: {0:?}")]
    Syntax(String),

    #[error("No command provided")]
    Empty,

    #[error(
        "Arguments are not allowed — pass only the executable name from \
         app_list(). Got: {0:?}"
    )]
    Arguments(String),

    #[error("{0:?} is not a known GUI app. Call app_list() to see what is available.")]
    Unknown(String),

    #[error("PID {0} was not launched by this session. Use app_launch() first.")]
    Untracked(u32),

    #[error("{0:?} is in the catalogue but is not installed. Call app_list() again.")]
    Missing(String),

    #[error("could not start {} ({source})", program.display())]
    Spawn {
        program: PathBuf,
        source: std::io::Error,
    },

    #[error("the launched process exited before it was tracked")]
    Vanished,

    #[error("could not set up the log file for a launched program")]
    Log(#[source] std::io::Error),

    #[error("the desktop could not be queried")]
    Desktop(#[source] anyhow::Error),
}

impl Blame for ProgramsError {
    fn blames_the_caller(&self) -> bool {
        match self {
            // Each of these names what to send instead, so the model can
            // correct itself and call again. The rest are the host's problem,
            // and their messages carry paths the model must never read.
            Self::Syntax(_)
            | Self::Empty
            | Self::Arguments(_)
            | Self::Unknown(_)
            | Self::Untracked(_)
            | Self::Missing(_) => true,
            Self::Spawn { .. } | Self::Vanished | Self::Log(_) | Self::Desktop(_) => false,
        }
    }
}
