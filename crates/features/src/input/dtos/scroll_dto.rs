use nest_rs::core::input;

use super::ScrollDirectionDto;

fn default_amount() -> u32 {
    3
}

/// What `mouse_scroll` accepts.
#[input]
#[derive(Debug)]
pub struct ScrollDto {
    pub x: i64,
    pub y: i64,
    #[serde(default)]
    pub direction: ScrollDirectionDto,
    /// Wheel notches, clamped to 1-5 per call — chain calls for long pages.
    #[serde(default = "default_amount")]
    pub amount: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn direction_names_are_lowercase_and_the_notch_count_has_a_default() {
        let scroll: ScrollDto = serde_json::from_str(r#"{"x":1,"y":2,"direction":"up"}"#).unwrap();
        assert_eq!(scroll.direction, ScrollDirectionDto::Up);
        assert_eq!(scroll.amount, 3, "the default notch count");
    }
}
