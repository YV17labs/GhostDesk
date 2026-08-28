/// What became of a program this session launched.
#[derive(Debug, Clone)]
pub struct ProgramStatus {
    pub pid: u32,
    pub running: bool,
    pub log_file: String,
    pub tail: String,
}
