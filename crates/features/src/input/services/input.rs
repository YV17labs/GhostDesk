use std::sync::Arc;

use nest_rs::core::{hooks, injectable};
use nest_rs::health::indicators;
use platform::coords;
use platform::input::{Button, InputBackend, ScrollDirection};
use platform::screen::Region;

use super::feedback::FeedbackService;
use crate::input::action::Action;
use crate::input::error::InputError;
use crate::input::feedback::Feedback;

type Result<T> = std::result::Result<T, InputError>;

/// How an act's coordinates became screen pixels.
///
/// The other half of the trail line, and the half no agent ever sees.
/// [`Action`] carries the coordinates the agent *gave*; this carries the
/// pixels they resolved to and the region they were read against, and with
/// all of it on one line the conversion between them is auditable — which is
/// what an audit of a coordinate space actually asks. The pixels alone look perfectly
/// reasonable whatever was asked, so a line without the request cannot show a
/// mis-declared space at all.
#[derive(Default)]
struct Landed {
    /// `None` on the verbs that never had a coordinate — a keypress, a typed
    /// string — which keeps the field off the line rather than putting a zero
    /// on it.
    x: Option<i64>,
    y: Option<i64>,
    to_x: Option<i64>,
    to_y: Option<i64>,
    /// The region the coordinates were read off, as the agent gave it. One
    /// point read off a region and off the whole screen is one request and
    /// two pixels, so without it the line cannot say which conversion ran.
    frame: Option<Region>,
}

impl Landed {
    fn at(x: i64, y: i64, frame: Option<Region>) -> Self {
        Self {
            x: Some(x),
            y: Some(y),
            frame,
            ..Self::default()
        }
    }

    fn between(from: (i64, i64), to: (i64, i64), frame: Option<Region>) -> Self {
        Self {
            x: Some(from.0),
            y: Some(from.1),
            to_x: Some(to.0),
            to_y: Some(to.1),
            frame,
        }
    }
}

#[injectable]
pub struct InputService {
    #[inject]
    svc: Arc<FeedbackService>,
    #[inject]
    backend: Arc<dyn InputBackend>,
}

#[hooks]
impl InputService {
    #[on_application_bootstrap]
    async fn warm_up(&self) {
        let started = std::time::Instant::now();
        match self.backend.warm_up().await {
            Ok(()) => tracing::info!(
                target: "features::input",
                elapsed_ms = started.elapsed().as_millis() as u64,
                "input backend ready",
            ),
            Err(err) => tracing::warn!(
                target: "features::input",
                error = %err,
                elapsed_ms = started.elapsed().as_millis() as u64,
                "input backend unavailable at boot — serving unhealthy until it binds",
            ),
        }
    }
}

#[indicators]
impl InputService {
    #[liveness]
    async fn input_backend(&self) -> anyhow::Result<()> {
        self.backend.ping().await
    }
}

impl InputService {
    /// Record the act, perform it, then watch for its effect.
    ///
    /// The record comes first, and that is the whole shape. `Err` from a
    /// backend does not mean the desktop was left alone — a compositor can
    /// swallow a reply after delivering the event, and a click has already
    /// moved the pointer by the time it presses — so a trail written only once
    /// every step succeeded under-reports what the server did to someone's real
    /// machine. What this line claims is therefore what was *attempted*, which
    /// is the question an audit asks; whether anything came of it is the
    /// verdict line, joined by `action`.
    ///
    /// `perform` takes its own baseline, because where in the sequence the
    /// baseline belongs is the verb's business: a click positions the pointer
    /// first, and that move must not be counted as the effect of the click.
    async fn acted(
        &self,
        action: Action,
        landed: Landed,
        perform: impl AsyncFnOnce() -> Result<Vec<u8>>,
    ) -> Result<Feedback> {
        tracing::info!(
            target: "features::input",
            action = action.kind(),
            button = action.button(),
            x = action.x(),
            y = action.y(),
            to_x = action.to_x(),
            to_y = action.to_y(),
            region_x = landed.frame.map(|frame| frame.x),
            region_y = landed.frame.map(|frame| frame.y),
            region_width = landed.frame.map(|frame| frame.width),
            region_height = landed.frame.map(|frame| frame.height),
            screen_x = landed.x,
            screen_y = landed.y,
            screen_to_x = landed.to_x,
            screen_to_y = landed.to_y,
            direction = action.direction(),
            amount = action.amount(),
            chars = action.chars(),
            keys = action.keys(),
            "input action",
        );
        let before = perform().await?;
        self.svc.observe(action, &before).await
    }

    pub async fn mouse_move(&self, x: i64, y: i64, frame: Option<Region>) -> Result<Feedback> {
        let (at_x, at_y) = coords::to_pixels(frame, x, y);
        self.acted(
            Action::Move { x, y },
            Landed::at(at_x, at_y, frame),
            async || {
                let before = self.svc.capture_before().await?;
                self.backend
                    .move_to(at_x, at_y)
                    .await
                    .map_err(InputError::Backend)?;
                Ok(before)
            },
        )
        .await
    }

    pub async fn mouse_click(
        &self,
        x: i64,
        y: i64,
        frame: Option<Region>,
        button: Button,
    ) -> Result<Feedback> {
        let (at_x, at_y) = coords::to_pixels(frame, x, y);
        self.acted(
            Action::Click { button, x, y },
            Landed::at(at_x, at_y, frame),
            async || {
                self.backend
                    .move_to(at_x, at_y)
                    .await
                    .map_err(InputError::Backend)?;
                let before = self.svc.capture_before().await?;
                self.backend
                    .click(button)
                    .await
                    .map_err(InputError::Backend)?;
                Ok(before)
            },
        )
        .await
    }

    pub async fn mouse_double_click(
        &self,
        x: i64,
        y: i64,
        frame: Option<Region>,
        button: Button,
    ) -> Result<Feedback> {
        let (at_x, at_y) = coords::to_pixels(frame, x, y);
        self.acted(
            Action::DoubleClick { button, x, y },
            Landed::at(at_x, at_y, frame),
            async || {
                self.backend
                    .move_to(at_x, at_y)
                    .await
                    .map_err(InputError::Backend)?;
                let before = self.svc.capture_before().await?;
                for _ in 0..2 {
                    self.backend
                        .click(button)
                        .await
                        .map_err(InputError::Backend)?;
                }
                Ok(before)
            },
        )
        .await
    }

    pub async fn mouse_drag(
        &self,
        from: (i64, i64),
        to: (i64, i64),
        frame: Option<Region>,
        button: Button,
    ) -> Result<Feedback> {
        let at_from = coords::to_pixels(frame, from.0, from.1);
        let at_to = coords::to_pixels(frame, to.0, to.1);
        let action = Action::Drag {
            button,
            x: from.0,
            y: from.1,
            to_x: to.0,
            to_y: to.1,
        };
        self.acted(action, Landed::between(at_from, at_to, frame), async || {
            let before = self.svc.capture_before().await?;
            self.backend
                .drag(at_from, at_to, button)
                .await
                .map_err(InputError::Backend)?;
            Ok(before)
        })
        .await
    }

    pub async fn mouse_scroll(
        &self,
        x: i64,
        y: i64,
        frame: Option<Region>,
        direction: ScrollDirection,
        amount: u32,
    ) -> Result<Feedback> {
        let (at_x, at_y) = coords::to_pixels(frame, x, y);
        let action = Action::Scroll {
            direction,
            amount,
            x,
            y,
        };
        self.acted(action, Landed::at(at_x, at_y, frame), async || {
            self.backend
                .move_to(at_x, at_y)
                .await
                .map_err(InputError::Backend)?;
            let before = self.svc.capture_before().await?;
            self.backend
                .scroll(direction, amount)
                .await
                .map_err(InputError::Backend)?;
            Ok(before)
        })
        .await
    }

    pub async fn key_type(&self, text: &str) -> Result<Feedback> {
        let action = Action::Type {
            chars: text.chars().count(),
        };
        self.acted(action, Landed::default(), async || {
            let before = self.svc.capture_before().await?;
            self.backend
                .type_text(text)
                .await
                .map_err(InputError::Backend)?;
            Ok(before)
        })
        .await
    }

    pub async fn key_press(&self, keys: &str) -> Result<Feedback> {
        // Resolved before the act is claimed: a chord this desktop cannot spell
        // never reaches it, so there is nothing to have attempted.
        let chord = self
            .backend
            .resolve_chord(keys)
            .map_err(InputError::Chord)?;
        let action = Action::Key {
            keys: keys.to_owned(),
        };
        self.acted(action, Landed::default(), async || {
            let before = self.svc.capture_before().await?;
            self.backend
                .press_chord(chord)
                .await
                .map_err(InputError::Backend)?;
            Ok(before)
        })
        .await
    }
}

#[cfg(test)]
mod tests {
    use nest_rs::testing::LogCapture;
    use platform::screen::ScreenBackend;

    use super::*;
    use crate::input::services::feedback::Watch;
    use crate::testing::{FailingWatch, RecordingInput, RestlessScreen};

    /// Short enough that a suite driving every verb for real costs
    /// milliseconds rather than the seconds a desktop is given.
    const WATCH: Watch = Watch {
        window: std::time::Duration::from_millis(60),
        poll: std::time::Duration::from_millis(2),
    };

    fn service(backend: Arc<RecordingInput>, screen: Arc<dyn ScreenBackend>) -> InputService {
        InputService {
            svc: Arc::new(FeedbackService::watching(screen, WATCH)),
            backend,
        }
    }

    /// A desktop that reacts, so the watch ends on the change rather than on
    /// its deadline — these tests are about what reached the backend and what
    /// reached the trail, not about how long a quiet screen is watched.
    fn on_a_lively_desktop() -> (Arc<RecordingInput>, InputService) {
        let backend = Arc::new(RecordingInput::default());
        let screen = Arc::new(RestlessScreen::default());
        (Arc::clone(&backend), service(backend, screen))
    }

    /// The lower half of the screen, asked for in the agent's own space.
    const LOWER: Region = Region {
        x: 0,
        y: 500,
        width: 1000,
        height: 500,
    };

    fn act(logs: &LogCapture) -> nest_rs::testing::CapturedEvent {
        logs.expect_one("features::input", "input action")
    }

    #[tokio::test]
    async fn a_click_moves_before_it_presses() {
        let (backend, input) = on_a_lively_desktop();
        input
            .mouse_click(612, 335, None, Button::Left)
            .await
            .expect("clicked");
        assert_eq!(
            backend.calls(),
            vec!["move_to(612,335)".to_string(), "click(left)".to_string()],
            "a press at the old position clicks whatever was there instead",
        );
    }

    #[tokio::test]
    async fn a_double_click_presses_twice_after_one_move() {
        let (backend, input) = on_a_lively_desktop();
        input
            .mouse_double_click(10, 20, None, Button::Right)
            .await
            .expect("clicked");
        assert_eq!(
            backend.calls(),
            vec![
                "move_to(10,20)".to_string(),
                "click(right)".to_string(),
                "click(right)".to_string(),
            ],
        );
    }

    #[tokio::test]
    async fn a_point_read_off_a_region_is_clicked_where_that_region_sits() {
        let (backend, input) = on_a_lively_desktop();
        // The agent captured the lower half and read the middle of it.
        coords::with_model_space(1000, async {
            input
                .mouse_click(500, 500, Some(LOWER), Button::Left)
                .await
                .expect("clicked");
        })
        .await;

        assert_eq!(
            backend.calls().first().map(String::as_str),
            Some("move_to(640,768)"),
            "the middle of the lower half — not the middle of the screen, \
             which is where the same point went before the frame travelled \
             with it",
        );
    }

    #[tokio::test]
    async fn the_trail_carries_the_request_beside_the_pixels_it_became() {
        let logs = LogCapture::install();
        let (_backend, input) = on_a_lively_desktop();

        coords::with_model_space(1000, async {
            input
                .mouse_click(500, 500, None, Button::Left)
                .await
                .expect("clicked");
        })
        .await;

        let act = act(&logs);
        assert_eq!(act.field("x").as_deref(), Some("500"), "what was asked");
        assert_eq!(
            act.field("screen_x").as_deref(),
            Some("640"),
            "what it became — both, so the conversion itself is auditable",
        );
        assert_eq!(act.field("screen_y").as_deref(), Some("512"));
        assert_eq!(
            act.field("region_x"),
            None,
            "read off the whole screen, so no region is on the line",
        );
    }

    #[tokio::test]
    async fn the_trail_names_the_region_a_point_was_read_against() {
        // One request, two pixels, depending on the frame it was read off —
        // so a line that drops the frame cannot be checked against the pixel
        // beside it.
        let logs = LogCapture::install();
        let (_backend, input) = on_a_lively_desktop();
        coords::with_model_space(1000, async {
            input
                .mouse_click(500, 500, Some(LOWER), Button::Left)
                .await
                .expect("clicked");
        })
        .await;

        let act = act(&logs);
        assert_eq!(act.field("y").as_deref(), Some("500"), "what was asked");
        assert_eq!(act.field("region_y").as_deref(), Some("500"));
        assert_eq!(act.field("region_height").as_deref(), Some("500"));
        assert_eq!(
            act.field("screen_y").as_deref(),
            Some("768"),
            "what it became, which only the region on the same line explains",
        );
    }

    /// The obligation every pointer verb carries, asserted on every one of
    /// them rather than on the one that happened to be written first.
    ///
    /// The request and the pixels it resolved to are two pairs of the same
    /// type, and nothing but their names keeps them apart — so a verb that
    /// hands the pixels where the request belongs compiles, answers the agent
    /// in a space it does not speak, and logs a line that reads perfectly
    /// well. That is the defect this whole path exists to close, and one verb
    /// proving itself proves nothing about the other four.
    #[tokio::test]
    async fn every_pointer_verb_keeps_the_request_apart_from_the_pixels() {
        let logs = LogCapture::install();
        let (backend, input) = on_a_lively_desktop();

        // (500, 500) read off the lower half of a 1280x1024 screen is the
        // pixel (640, 768) — and is (640, 512) read off the whole screen, so
        // a verb that loses the frame is visible in the number.
        coords::with_model_space(1000, async {
            input
                .mouse_move(500, 500, Some(LOWER))
                .await
                .expect("moved");
            input
                .mouse_click(500, 500, Some(LOWER), Button::Left)
                .await
                .expect("clicked");
            input
                .mouse_double_click(500, 500, Some(LOWER), Button::Left)
                .await
                .expect("double-clicked");
            input
                .mouse_drag((500, 500), (600, 700), Some(LOWER), Button::Left)
                .await
                .expect("dragged");
            input
                .mouse_scroll(500, 500, Some(LOWER), ScrollDirection::Down, 3)
                .await
                .expect("scrolled");
        })
        .await;

        let acts = logs.find("features::input", "input action");
        assert_eq!(acts.len(), 5, "one line per pointer verb");

        for act in &acts {
            let verb = act.field("action").unwrap_or_default();
            assert_eq!(
                act.field("x").as_deref(),
                Some("500"),
                "{verb} must log what the agent asked",
            );
            assert_eq!(
                act.field("screen_x").as_deref(),
                Some("640"),
                "{verb} must log the pixel it resolved to",
            );
            assert_eq!(act.field("screen_y").as_deref(), Some("768"), "{verb}");
            assert_eq!(
                act.field("region_y").as_deref(),
                Some("500"),
                "{verb} must name the frame that explains the pair",
            );
        }

        assert!(
            !backend.calls().iter().any(|call| call.contains("500,500")),
            "a verb handed the desktop the request instead of the pixels: {:?}",
            backend.calls(),
        );
    }

    #[tokio::test]
    async fn the_agent_is_answered_in_the_space_it_asked_in() {
        use crate::input::dtos::FeedbackDto;

        let (_backend, input) = on_a_lively_desktop();

        let feedback = coords::with_model_space(1000, async {
            input
                .mouse_click(236, 608, None, Button::Left)
                .await
                .expect("clicked")
        })
        .await;

        assert_eq!(
            FeedbackDto::from(feedback).action,
            "Clicked left at (236, 608)",
            "answering `(302, 623)` to an agent that speaks 0-1000 hands it a \
             number it cannot correct from — it reads as a click somewhere it \
             never aimed",
        );
    }

    #[tokio::test]
    async fn a_mis_spelled_chord_fails_before_anything_is_pressed() {
        let logs = LogCapture::install();
        let (backend, input) = on_a_lively_desktop();

        let err = input.key_press("ctrl+nonsense").await.unwrap_err();
        assert!(matches!(err, InputError::Chord(_)), "got: {err}");
        assert!(
            backend.calls().is_empty(),
            "the refusal costs no baseline and touches no desktop: {:?}",
            backend.calls(),
        );
        logs.expect_none("features::input", "input action");
    }

    #[tokio::test]
    async fn the_act_is_spelled_in_fields_never_in_a_sentence() {
        let logs = LogCapture::install();
        let (_backend, input) = on_a_lively_desktop();
        input
            .mouse_click(612, 335, None, Button::Left)
            .await
            .expect("clicked");

        let act = act(&logs);
        assert_eq!(act.field("action").as_deref(), Some("click"));
        assert_eq!(act.field("button").as_deref(), Some("left"));
        assert_eq!(act.field("x").as_deref(), Some("612"));
        assert_eq!(act.field("y").as_deref(), Some("335"));
    }

    #[tokio::test]
    async fn a_field_the_act_never_had_is_absent_rather_than_empty() {
        let logs = LogCapture::install();
        let (_backend, input) = on_a_lively_desktop();
        input.key_press("ctrl+c").await.expect("pressed");

        let act = act(&logs);
        assert_eq!(act.field("keys").as_deref(), Some("ctrl+c"));
        assert_eq!(
            act.field("x"),
            None,
            "a keypress never had a coordinate, so the field is absent",
        );
        assert_eq!(act.field("button"), None);
    }

    #[tokio::test]
    async fn typed_text_is_counted_never_quoted() {
        let logs = LogCapture::install();
        let (_backend, input) = on_a_lively_desktop();
        input.key_type("hunter2!").await.expect("typed");

        let act = act(&logs);
        assert_eq!(act.field("chars").as_deref(), Some("8"));
        assert!(
            !format!("{:?}", logs.events()).contains("hunter2"),
            "a trail that leaks what it audits is worse than none",
        );
    }

    /// The press landed and the watch then failed. Recorded from the verdict —
    /// as it once was — this act left no trace at all.
    #[tokio::test]
    async fn an_act_whose_effect_cannot_be_observed_is_still_recorded() {
        let logs = LogCapture::install();
        let backend = Arc::new(RecordingInput::default());
        let input = service(Arc::clone(&backend), Arc::new(FailingWatch::default()));

        let err = input.key_type("abc").await.unwrap_err();
        assert!(matches!(err, InputError::Feedback(_)), "got: {err}");
        assert_eq!(backend.calls(), vec!["type_text(3 chars)".to_string()]);
        assert_eq!(act(&logs).field("chars").as_deref(), Some("3"));
    }

    /// The pointer moved on someone's real machine and the press then failed.
    /// The act begins at the *first* call that reaches the desktop, which is
    /// why the line is filed before any of them rather than after all of them.
    #[tokio::test]
    async fn an_act_that_fails_partway_is_still_recorded() {
        let logs = LogCapture::install();
        let backend = Arc::new(RecordingInput::default());
        let input = service(
            Arc::clone(&backend),
            Arc::new(FailingWatch::from_the_first_capture()),
        );

        let err = input
            .mouse_click(4, 5, None, Button::Left)
            .await
            .unwrap_err();
        assert!(matches!(err, InputError::Feedback(_)), "got: {err}");
        assert_eq!(
            backend.calls(),
            vec!["move_to(4,5)".to_string()],
            "the baseline failed between the move and the press",
        );

        let act = act(&logs);
        assert_eq!(act.field("action").as_deref(), Some("click"));
        assert_eq!(act.field("x").as_deref(), Some("4"));
    }

    #[tokio::test]
    async fn every_verb_that_reaches_the_desktop_leaves_a_line() {
        // One funnel, so a verb added without its trail is a test failure
        // rather than a silent gap in the record.
        let logs = LogCapture::install();
        let (_backend, input) = on_a_lively_desktop();

        input.mouse_move(1, 2, None).await.expect("moved");
        input
            .mouse_click(1, 2, None, Button::Left)
            .await
            .expect("clicked");
        input
            .mouse_double_click(1, 2, None, Button::Left)
            .await
            .expect("clicked");
        input
            .mouse_drag((1, 2), (3, 4), None, Button::Left)
            .await
            .expect("dragged");
        input
            .mouse_scroll(1, 2, None, ScrollDirection::Down, 3)
            .await
            .expect("scrolled");
        input.key_type("hi").await.expect("typed");
        input.key_press("tab").await.expect("pressed");

        let acted: Vec<String> = logs
            .find("features::input", "input action")
            .into_iter()
            .filter_map(|event| event.field("action"))
            .collect();
        assert_eq!(
            acted,
            [
                "move",
                "click",
                "double_click",
                "drag",
                "scroll",
                "type",
                "key",
            ],
            "one line per act, in order, no more and no fewer",
        );
    }
}
