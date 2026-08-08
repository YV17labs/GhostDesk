use nest_rs::core::input;

/// Trailing log lines `app_status` returns when the caller does not say.
///
/// A published default belongs to the wire contract, not to the domain:
/// `ProgramsService::status` is handed a line count and has no opinion on
/// what the absence of one should mean.
fn default_tail() -> usize {
    50
}

/// What `app_status` accepts.
#[input]
#[derive(Debug)]
pub struct StatusDto {
    /// A process ID returned by `app_launch()`.
    pub pid: u32,
    /// Trailing log lines to return.
    #[serde(default = "default_tail")]
    pub lines: usize,
}
