use nest_rs::core::input;

fn default_tail() -> usize {
    50
}

#[input]
#[derive(Debug)]
pub struct StatusDto {
    pub pid: u32,
    #[serde(default = "default_tail")]
    pub lines: usize,
}
