//! Wire DTOs for the tool surface.
//!
//! `#[input]` carries `Serialize`/`Deserialize`/`Validate`/`JsonSchema`, so
//! the schema a client reads and the validation a call goes through both come
//! from this one declaration. The enums derive directly — a `Validate` on a
//! plain choice would have nothing to check.

use nest_rs::core::input;
use platform::desktop::DesktopApp;
use platform::input::{Button, ScrollDirection};
use platform::screen::{ImageFormat, Region};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Which pointer button to use.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum ButtonArg {
    #[default]
    Left,
    Middle,
    Right,
}

impl From<ButtonArg> for Button {
    fn from(value: ButtonArg) -> Self {
        match value {
            ButtonArg::Left => Self::Left,
            ButtonArg::Middle => Self::Middle,
            ButtonArg::Right => Self::Right,
        }
    }
}

/// Which way to scroll.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum ScrollArg {
    Up,
    #[default]
    Down,
    Left,
    Right,
}

impl From<ScrollArg> for ScrollDirection {
    fn from(value: ScrollArg) -> Self {
        match value {
            ScrollArg::Up => Self::Up,
            ScrollArg::Down => Self::Down,
            ScrollArg::Left => Self::Left,
            ScrollArg::Right => Self::Right,
        }
    }
}

/// Encoding for a returned capture.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum FormatArg {
    #[default]
    Webp,
    Png,
}

impl From<FormatArg> for ImageFormat {
    fn from(value: FormatArg) -> Self {
        match value {
            FormatArg::Webp => Self::Webp,
            FormatArg::Png => Self::Png,
        }
    }
}

/// An area of the screen to capture.
#[input]
#[derive(Debug, Clone, Copy)]
pub struct RegionArg {
    pub x: i64,
    pub y: i64,
    pub width: i64,
    pub height: i64,
}

impl From<RegionArg> for Region {
    fn from(value: RegionArg) -> Self {
        Self {
            x: value.x,
            y: value.y,
            width: value.width,
            height: value.height,
        }
    }
}

fn default_true() -> bool {
    true
}

fn default_quality() -> u8 {
    platform::screen::DEFAULT_WEBP_QUALITY
}

fn default_amount() -> u32 {
    3
}

fn default_tail() -> usize {
    crate::apps::DEFAULT_TAIL
}

#[input]
#[derive(Debug)]
pub struct ScreenShotParams {
    /// Area to capture. Omit to capture the full screen — almost always the
    /// right choice.
    #[serde(default)]
    pub region: Option<RegionArg>,

    /// `webp` (default, small payload, visually stable at UI resolutions) or
    /// `png` (lossless, larger).
    #[serde(default)]
    pub format: FormatArg,

    /// Wait up to 2.5 s for two consecutive frames to be identical before
    /// returning — catches pages still animating after a click or a
    /// navigation. Set false only to observe a genuinely animating UI.
    #[serde(default = "default_true")]
    pub stabilize: bool,

    /// WebP encoder quality. The default is visually indistinguishable from
    /// 80 on typical UI content but roughly half the bytes; raise it to read
    /// fine pixels (small fonts in a PDF, design mockups, photos). Ignored
    /// for PNG.
    #[serde(default = "default_quality")]
    #[validate(range(min = 1, max = 100))]
    pub quality: u8,
}

#[input]
#[derive(Debug)]
pub struct MoveParams {
    pub x: i64,
    pub y: i64,
}

#[input]
#[derive(Debug)]
pub struct ClickParams {
    pub x: i64,
    pub y: i64,
    #[serde(default)]
    pub button: ButtonArg,
}

#[input]
#[derive(Debug)]
pub struct DragParams {
    pub from_x: i64,
    pub from_y: i64,
    pub to_x: i64,
    pub to_y: i64,
    #[serde(default)]
    pub button: ButtonArg,
}

#[input]
#[derive(Debug)]
pub struct ScrollParams {
    pub x: i64,
    pub y: i64,
    #[serde(default)]
    pub direction: ScrollArg,
    /// Wheel notches, clamped to 1-5 per call — chain calls for long pages.
    #[serde(default = "default_amount")]
    pub amount: u32,
}

#[input]
#[derive(Debug)]
pub struct TypeParams {
    pub text: String,
}

#[input]
#[derive(Debug)]
pub struct PressParams {
    /// A key or a chord, `+`-separated: `ctrl+shift+tab`, `alt+tab`, `f5`.
    pub keys: String,
}

#[input]
#[derive(Debug)]
pub struct LaunchParams {
    /// A bare executable name from `app_list()`. Arguments are refused.
    pub command: String,
}

#[input]
#[derive(Debug)]
pub struct StatusParams {
    /// A process ID returned by `app_launch()`.
    pub pid: u32,
    /// Trailing log lines to return.
    #[serde(default = "default_tail")]
    pub lines: usize,
}

#[input]
#[derive(Debug)]
pub struct ClipboardSetParams {
    pub text: String,
}

// --- outputs ------------------------------------------------------------
//
// Returning `Json<T>` is what makes rmcp derive each tool's `outputSchema`,
// so these types are the published shape of every structured result. They
// are plain `Serialize + JsonSchema`: nothing deserializes a response, and
// nothing validates one on the way out.

/// One installed GUI application.
///
/// A DTO rather than `platform::desktop::DesktopApp` directly: renaming a
/// field in the `.desktop` parser would otherwise silently change the MCP
/// contract, with nothing in this module to notice.
#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct AppEntry {
    /// Human-readable application name.
    pub name: String,
    /// The string to pass to `app_launch`.
    pub exec: String,
}

impl From<DesktopApp> for AppEntry {
    fn from(app: DesktopApp) -> Self {
        Self {
            name: app.name,
            exec: app.exec,
        }
    }
}

/// A list result.
///
/// `structuredContent` is typed as a JSON *object* by the spec, so a tool
/// whose natural result is a list needs a field to hang it on. `result` is
/// the name the Python original used, kept so existing prompts and clients
/// see the same shape.
#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct Listed<T> {
    pub result: Vec<T>,
}

impl<T> Listed<T> {
    pub fn new(result: impl IntoIterator<Item = T>) -> Self {
        Self {
            result: result.into_iter().collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn omitted_screenshot_fields_fall_back_to_the_agent_facing_defaults() {
        let params: ScreenShotParams = serde_json::from_str("{}").unwrap();
        assert!(params.region.is_none());
        assert_eq!(params.format, FormatArg::Webp);
        assert!(params.stabilize);
        assert_eq!(params.quality, 50);
    }

    #[test]
    fn button_and_direction_names_are_lowercase_on_the_wire() {
        let click: ClickParams = serde_json::from_str(r#"{"x":1,"y":2,"button":"right"}"#).unwrap();
        assert_eq!(click.button, ButtonArg::Right);

        let scroll: ScrollParams =
            serde_json::from_str(r#"{"x":1,"y":2,"direction":"up"}"#).unwrap();
        assert_eq!(scroll.direction, ScrollArg::Up);
        assert_eq!(scroll.amount, 3, "the default notch count");
    }

    #[test]
    fn a_click_without_a_button_defaults_to_left() {
        let click: ClickParams = serde_json::from_str(r#"{"x":10,"y":20}"#).unwrap();
        assert_eq!(click.button, ButtonArg::Left);
    }

    #[test]
    fn out_of_range_quality_fails_validation() {
        use nest_rs::core::validator::Validate;

        let params: ScreenShotParams = serde_json::from_str(r#"{"quality":0}"#).unwrap();
        assert!(params.validate().is_err());

        let params: ScreenShotParams = serde_json::from_str(r#"{"quality":100}"#).unwrap();
        assert!(params.validate().is_ok());
    }
}
