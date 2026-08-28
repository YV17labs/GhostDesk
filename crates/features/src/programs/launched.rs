/// A process this session started.
#[derive(Debug, Clone)]
pub struct Launched {
    pub pid: u32,
    pub log_file: String,
    pub action: String,
}
