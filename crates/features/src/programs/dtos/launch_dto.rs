use nest_rs::core::input;

fn default_true() -> bool {
    true
}

/// What `app_launch` accepts.
#[input]
#[derive(Debug)]
pub struct LaunchDto {
    /// A bare executable name from `app_list()`. Arguments are refused.
    pub command: String,

    /// Wait (bounded) for the program to open its first window and include
    /// the settled screen in the result. Turn off only for programs expected
    /// to run without a UI.
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
