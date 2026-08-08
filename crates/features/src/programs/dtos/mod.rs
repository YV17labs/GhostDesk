//! What a client sends and reads for the four program tools.
//!
//! Every output shape here is a DTO over a type that already exists in
//! [`service`](super::service), and that duplication is the point: a field
//! renamed in the domain is a refactor, while the same rename on the wire is
//! a breaking change for every client. Keeping the two apart is what makes
//! the compiler ask which one you meant.

mod launch_dto;
mod launched_dto;
mod listed_dto;
mod program_dto;
mod program_status_dto;
mod status_dto;
mod window_dto;

pub use launch_dto::LaunchDto;
pub use launched_dto::LaunchedDto;
pub use listed_dto::ListedDto;
pub use program_dto::ProgramDto;
pub use program_status_dto::ProgramStatusDto;
pub use status_dto::StatusDto;
pub use window_dto::WindowDto;
