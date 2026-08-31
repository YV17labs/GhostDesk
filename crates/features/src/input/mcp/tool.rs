use std::sync::Arc;

use nest_rs::mcp::{Json, McpError, Parameters, Valid, mcp, tools};

use crate::blame::Answered;
use crate::input::dtos::{ClickDto, DragDto, FeedbackDto, MoveDto, PressDto, ScrollDto, TypeDto};
use crate::input::services::InputService;

#[mcp]
#[derive(Clone)]
pub struct InputTool {
    #[inject]
    svc: Arc<InputService>,
}

#[tools]
impl InputTool {
    #[tool(
        description = "Move the cursor to (x, y) without pressing any button.\n\n\
            Use this to trigger hover-only UI reactions: dropdown menus that \
            appear on mouse-over, tooltips, CSS :hover states, or any element \
            that reveals itself when the pointer enters its bounds. Take a \
            screen_shot() first to get the coordinates.\n\n\
            A screen_changed of false is expected for elements that have no \
            hover effect — it is not an error. If the menu or tooltip you \
            wanted does not appear, the element probably needs a click \
            instead: fall back to mouse_click.",
        annotations(destructive_hint = false, open_world_hint = false)
    )]
    #[public]
    async fn mouse_move(
        &self,
        Parameters(params): Parameters<MoveDto>,
    ) -> Result<Json<FeedbackDto>, McpError> {
        Ok(Json(FeedbackDto::from(
            self.svc.mouse_move(params.x, params.y).await.answered()?,
        )))
    }

    #[tool(
        description = "Click once at screen coordinates (pixels from the last \
            screen_shot()).\n\n\
            A screen_changed of false means the click had no visible effect \
            anywhere on screen. Do not retry the same coordinates — the target \
            probably moved (page scrolled, dialog opened) or was never where \
            you thought. Take a new screen_shot() and recompute.",
        annotations(destructive_hint = true, open_world_hint = false)
    )]
    #[public]
    async fn mouse_click(
        &self,
        Parameters(params): Parameters<ClickDto>,
    ) -> Result<Json<FeedbackDto>, McpError> {
        Ok(Json(FeedbackDto::from(
            self.svc
                .mouse_click(params.x, params.y, params.button.into())
                .await
                .answered()?,
        )))
    }

    #[tool(
        description = "Double-click at screen coordinates. Standard use cases: \
            open a file or folder in a file manager, select an entire word in \
            editable text.",
        annotations(destructive_hint = true, open_world_hint = false)
    )]
    #[public]
    async fn mouse_double_click(
        &self,
        Parameters(params): Parameters<ClickDto>,
    ) -> Result<Json<FeedbackDto>, McpError> {
        Ok(Json(FeedbackDto::from(
            self.svc
                .mouse_double_click(params.x, params.y, params.button.into())
                .await
                .answered()?,
        )))
    }

    #[tool(
        description = "Drag from one point to another while holding a button. \
            Standard use cases: select a range of text, move a window or item, \
            resize from a corner, draw in a canvas.\n\n\
            For selecting text, mouse_click(start) plus key_press(\"shift+end\") \
            (or any shift+navigation) is often more reliable than a \
            pixel-precise drag.",
        annotations(destructive_hint = true, open_world_hint = false)
    )]
    #[public]
    async fn mouse_drag(
        &self,
        Parameters(params): Parameters<DragDto>,
    ) -> Result<Json<FeedbackDto>, McpError> {
        Ok(Json(FeedbackDto::from(
            self.svc
                .mouse_drag(
                    (params.from_x, params.from_y),
                    (params.to_x, params.to_y),
                    params.button.into(),
                )
                .await
                .answered()?,
        )))
    }

    #[tool(
        description = "Scroll the region under (x, y). direction is \
            up/down/left/right, amount is the number of wheel notches (clamped \
            to 1-5 per call — chain multiple calls for long pages, with a \
            screenshot between each).\n\n\
            A screen_changed of false typically means the page is already at \
            the scroll boundary — there is nothing more to reveal in that \
            direction.",
        annotations(destructive_hint = false, open_world_hint = false)
    )]
    #[public]
    async fn mouse_scroll(
        &self,
        Parameters(Valid(params)): Parameters<Valid<ScrollDto>>,
    ) -> Result<Json<FeedbackDto>, McpError> {
        Ok(Json(FeedbackDto::from(
            self.svc
                .mouse_scroll(params.x, params.y, params.direction.into(), params.amount)
                .await
                .answered()?,
        )))
    }

    #[tool(
        description = "Type text at the current keyboard focus. Handles \
            Unicode, newlines and tabs. Layout-independent — a French AZERTY \
            host produces the same output as US QWERTY.\n\n\
            For more than a sentence or two, prefer clipboard_set(text) plus \
            the paste shortcut: it is instant, immune to autocomplete and \
            autocorrect, and does not race with the app's own key handlers. \
            The server instructions name the modifier this desktop uses — it \
            is not the same on every OS.\n\n\
            A screen_changed of false almost always means the field did not \
            have focus. Click into it first and retry.",
        annotations(destructive_hint = true, open_world_hint = false)
    )]
    #[public]
    async fn key_type(
        &self,
        Parameters(params): Parameters<TypeDto>,
    ) -> Result<Json<FeedbackDto>, McpError> {
        Ok(Json(FeedbackDto::from(
            self.svc.key_type(&params.text).await.answered()?,
        )))
    }

    #[tool(
        description = "Press a key or a chord (modifiers plus key), using + as \
            separator.\n\n\
            Modifier tokens: ctrl/control, alt/option, shift, \
            super/meta/win/cmd/command. Which of them carries the standard \
            shortcuts differs by OS — the server instructions say which, and \
            sending the wrong one types a stray character instead of \
            failing.\n\n\
            Non-printable tokens: return/enter, escape/esc, backspace, delete, \
            tab, space, home/end, pageup/pagedown, left/right/up/down, \
            f1..f12. Every one of them is accepted on every desktop this \
            server runs on.\n\n\
            A screen_changed of false usually means the keystroke went to a \
            window or field that did not care about it — check focus with a \
            screenshot.",
        annotations(destructive_hint = true, open_world_hint = false)
    )]
    #[public]
    async fn key_press(
        &self,
        Parameters(params): Parameters<PressDto>,
    ) -> Result<Json<FeedbackDto>, McpError> {
        Ok(Json(FeedbackDto::from(
            self.svc.key_press(&params.keys).await.answered()?,
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::error::InputError;

    #[test]
    fn a_backend_failure_leaves_as_the_shared_opaque_message() {
        let err = Err::<(), _>(InputError::Backend(anyhow::anyhow!(
            "zwp_virtual_keyboard_v1 is missing on this compositor"
        )))
        .answered()
        .unwrap_err();

        assert_eq!(err.message, nest_rs::core::OPAQUE_CLIENT_MESSAGE);
    }

    #[test]
    fn a_mis_spelled_chord_is_returned_to_the_caller_verbatim() {
        let err = Err::<(), _>(InputError::Chord(anyhow::anyhow!("unknown key: ctrl+zz")))
            .answered()
            .unwrap_err();

        assert!(err.message.contains("ctrl+zz"), "{}", err.message);
    }

    #[test]
    fn the_published_chord_grammar_is_the_one_the_backends_serve() {
        // The description is where the grammar reaches the agent, and
        // `platform::chord` is where every backend is held to it. A token
        // served but never described is one no agent will send; a token
        // described but not served is one every agent will.
        let described = InputTool::tool_router()
            .list_all()
            .into_iter()
            .find(|tool| tool.name == "key_press")
            .and_then(|tool| tool.description)
            .expect("key_press describes itself")
            .to_string();

        // Whole words, not substrings: `esc` sits inside `escape`, `up` inside
        // `pageup` and `cmd` inside `command`, so containment would pass three
        // of these without the description ever naming them.
        let words: Vec<&str> = described
            .split(|c: char| !c.is_ascii_alphanumeric())
            .collect();

        for token in platform::chord::MODIFIERS
            .iter()
            .chain(platform::chord::KEYS)
        {
            // The function keys are named by their endpoints — `f1..f12` —
            // so those two are checked and the ten between them are covered
            // by shape, which is what keeps an `f13` from needing an edit
            // here.
            if !matches!(*token, "f1" | "f12")
                && token
                    .strip_prefix('f')
                    .is_some_and(|number| number.parse::<u8>().is_ok())
            {
                continue;
            }

            // `page_up`/`page_down` reach the reader as their hyphen-free
            // twins, which `KEYS` publishes too — so the description is asked
            // for the token with its underscore dropped.
            let published_as = token.replace('_', "");
            assert!(
                words.contains(&&*published_as),
                "{token:?} is served but never published: {described}",
            );
        }
    }

    #[test]
    fn every_tool_closes_its_world() {
        for tool in InputTool::tool_router().list_all() {
            assert_eq!(
                tool.annotations.and_then(|hints| hints.open_world_hint),
                Some(false),
                "{} declares a closed world",
                tool.name,
            );
        }
    }
}
