use nest_rs::core::input;

use super::ButtonDto;

#[input]
#[derive(Debug)]
pub struct ClickDto {
    pub x: i64,
    pub y: i64,
    #[serde(default)]
    pub button: ButtonDto,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn button_names_are_lowercase_on_the_wire() {
        let click: ClickDto = serde_json::from_str(r#"{"x":1,"y":2,"button":"right"}"#).unwrap();
        assert_eq!(click.button, ButtonDto::Right);
    }

    #[test]
    fn a_click_without_a_button_defaults_to_left() {
        let click: ClickDto = serde_json::from_str(r#"{"x":10,"y":20}"#).unwrap();
        assert_eq!(click.button, ButtonDto::Left);
    }
}
