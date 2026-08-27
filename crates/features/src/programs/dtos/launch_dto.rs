use nest_rs::core::input;

fn default_true() -> bool {
    true
}

#[input]
#[derive(Debug)]
pub struct LaunchDto {
    pub command: String,

    #[serde(default = "default_true")]
    pub wait_for_window: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_launch_waits_for_its_window_by_default() {
        let params: LaunchDto = serde_json::from_str(r#"{"command":"firefox"}"#).unwrap();
        assert!(params.wait_for_window);
    }
}
