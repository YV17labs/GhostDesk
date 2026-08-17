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
    /// Wheel notches. Bounded to 1-5 so one call cannot fly past content the
    /// agent never sees; long pages are scrolled by chaining calls, each with
    /// its own screenshot.
    #[serde(default = "default_amount")]
    #[validate(range(min = 1, max = 5))]
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

    /// The band the tool advertises is refused by the pipe, not clamped in
    /// silence: a model that asked for 20 notches has to be told it got 5.
    #[test]
    fn a_notch_count_outside_the_band_is_refused() {
        use nest_rs::core::validator::Validate;

        let of = |amount: u32| ScrollDto {
            x: 0,
            y: 0,
            direction: ScrollDirectionDto::Up,
            amount,
        };

        assert!(of(0).validate().is_err());
        assert!(of(20).validate().is_err());
        assert!(of(1).validate().is_ok());
        assert!(of(5).validate().is_ok());
    }
}
