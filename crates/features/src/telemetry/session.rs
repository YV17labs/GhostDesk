//! What one session accumulates, and the two patterns worth a warning.
//!
//! Everything here is bookkeeping over calls that already happened: no I/O,
//! no logging, no clock reads beyond the ones handed in. The service turns
//! what this produces into log events; keeping the rules separate from the
//! emitting is what makes them testable without a desktop.

use std::collections::hash_map::Entry;
use std::collections::{BTreeMap, HashMap};
use std::time::Instant;

/// Consecutive dead input actions before the streak is worth a line. Two in a
/// row is ordinary — a hover that does nothing, a modifier press. Three is an
/// agent that has lost track of the screen.
const DEAD_STREAK_THRESHOLD: u32 = 3;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    Ok,
    Error,
    /// The client hung up or cancelled before the handler returned. Reported
    /// by the guard's `Drop`, so it survives a future that is simply dropped.
    Cancelled,
}

impl Outcome {
    /// The value that goes in the log field. A `&'static str` rather than
    /// `Debug` so the field is a stable token something can filter on.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Ok => "ok",
            Self::Error => "error",
            Self::Cancelled => "cancelled",
        }
    }
}

/// What an input tool's feedback loop saw, reported by
/// [`note_action`](super::note_action).
///
/// The only per-tool detail the accumulator needs: it is what distinguishes
/// an action that moved the screen from one that did nothing, and the string
/// is the identity a repeated attempt is matched on.
#[derive(Debug, Clone)]
pub(super) struct Observed {
    pub(super) action: String,
    pub(super) screen_changed: bool,
}

/// One finished `tools/call`.
pub(super) struct Call {
    pub(super) seq: u64,
    pub(super) tool: String,
    /// Stable identity of what this call aimed at — tool plus arguments. What
    /// a repeated futile attempt is matched on.
    pub(super) target: String,
    pub(super) server_ms: u64,
    pub(super) gap_ms: Option<u64>,
    pub(super) outcome: Outcome,
    pub(super) result_bytes: usize,
    pub(super) image_bytes: usize,
    pub(super) observed: Option<Observed>,
}

/// A pattern the agent is visibly losing time to.
///
/// Individual failures are already logged where they happen; this is the
/// derived signal — the same failure *repeating* — because that, not the
/// single dead click, is what points at something the product should fix.
#[derive(Debug)]
pub(super) struct Friction {
    pub(super) kind: &'static str,
    pub(super) tool: String,
    /// What was being aimed at, in the same words the agent was given.
    pub(super) target: String,
    pub(super) count: u32,
    /// Server milliseconds burned by the run — the cost of the pattern,
    /// which is what justifies (or does not justify) fixing it.
    pub(super) wasted_ms: u64,
    /// The calls the run spans. A bracket rather than the full list: a run is
    /// unbounded, and an agent stuck for a hundred calls should not produce a
    /// log line carrying a hundred numbers.
    pub(super) first_seq: u64,
    pub(super) last_seq: u64,
}

/// The same coordinates or chord retried after producing no visible change —
/// the agent is stuck on a target it cannot hit.
const DEAD_SPOT: &str = "dead_spot";
/// Several actions in a row all produced no visible change — the agent has
/// lost track of where it is.
const DEAD_STREAK: &str = "dead_streak";

/// The session in one line: what makes two runs comparable.
#[derive(Debug, Default)]
pub(super) struct Summary {
    pub(super) calls: u64,
    pub(super) errors: u64,
    pub(super) cancelled: u64,

    /// Wall-clock span from first call to last.
    pub(super) duration_ms: u64,
    /// Of which, spent inside the server.
    pub(super) server_ms: u64,
    /// Of which, spent waiting on the model between calls. `server_ms` and
    /// `gap_ms` are the two halves of the latency the user feels, and their
    /// ratio is the answer to "where does the time actually go".
    pub(super) gap_ms: u64,

    pub(super) result_bytes: u64,
    pub(super) image_bytes: u64,

    pub(super) input_actions: u64,
    pub(super) dead_actions: u64,
    pub(super) friction: u64,

    /// Calls per tool. A `BTreeMap` so the order is stable across sessions
    /// and two summaries diff cleanly.
    pub(super) by_tool: BTreeMap<String, u64>,
}

/// A run of futile attempts, counted as it grows.
///
/// One shape for both patterns: a run at a single target is a dead spot, a
/// run across any targets is a dead streak. Totals are kept running rather
/// than recomputed, so reporting a long run costs the same as a short one.
struct Run {
    count: u32,
    wasted_ms: u64,
    first_seq: u64,
}

impl Run {
    fn started_by(call: &Call) -> Self {
        Self {
            count: 1,
            wasted_ms: call.server_ms,
            first_seq: call.seq,
        }
    }

    fn extend(&mut self, call: &Call) {
        self.count += 1;
        self.wasted_ms += call.server_ms;
    }

    fn to_friction(&self, kind: &'static str, call: &Call, target: &str) -> Friction {
        Friction {
            kind,
            tool: call.tool.clone(),
            target: target.to_string(),
            count: self.count,
            wasted_ms: self.wasted_ms,
            first_seq: self.first_seq,
            last_seq: call.seq,
        }
    }
}

pub(super) struct SessionState {
    pub(super) started: Instant,
    /// Last MCP traffic of any kind, which is what the idle sweep reads.
    pub(super) last_seen: Instant,
    /// When the previous call returned — the other end of the model-latency
    /// gap.
    last_end: Option<Instant>,
    seq: u64,
    summary: Summary,

    /// The current run of dead input actions, whatever they aimed at.
    streak: Option<Run>,
    /// One run per target that has gone dead, keyed by the action text the
    /// agent itself was shown.
    dead_spots: HashMap<String, Run>,
}

impl SessionState {
    pub(super) fn new(now: Instant) -> Self {
        Self {
            started: now,
            last_seen: now,
            last_end: None,
            seq: 0,
            summary: Summary::default(),
            streak: None,
            dead_spots: HashMap::new(),
        }
    }

    /// Claim the next position in the trajectory.
    pub(super) fn next_seq(&mut self) -> u64 {
        self.seq += 1;
        self.seq
    }

    /// Milliseconds since the previous call returned, or `None` on the first
    /// call of the session.
    pub(super) fn gap_since_last_call(&self, now: Instant) -> Option<u64> {
        self.last_end
            .map(|end| now.saturating_duration_since(end).as_millis() as u64)
    }

    /// Fold one finished call into the totals, and report any friction
    /// pattern it completes.
    pub(super) fn record(&mut self, call: &Call, finished_at: Instant) -> Vec<Friction> {
        self.last_end = Some(finished_at);
        self.last_seen = finished_at;

        self.summary.calls += 1;
        self.summary.server_ms += call.server_ms;
        self.summary.gap_ms += call.gap_ms.unwrap_or(0);
        self.summary.result_bytes += call.result_bytes as u64;
        self.summary.image_bytes += call.image_bytes as u64;
        *self.summary.by_tool.entry(call.tool.clone()).or_insert(0) += 1;
        match call.outcome {
            Outcome::Ok => {}
            Outcome::Error => self.summary.errors += 1,
            Outcome::Cancelled => self.summary.cancelled += 1,
        }

        self.note_friction(call)
    }

    /// Both friction rules, evaluated against one call.
    ///
    /// A call that is not an input action leaves the streak alone rather than
    /// resetting it: a screenshot between two dead clicks is the agent
    /// *looking*, which is exactly the behaviour the streak is meant to
    /// capture, not evidence that it recovered.
    fn note_friction(&mut self, call: &Call) -> Vec<Friction> {
        let Some(observed) = &call.observed else {
            return Vec::new();
        };

        self.summary.input_actions += 1;
        if observed.screen_changed {
            self.streak = None;
            return Vec::new();
        }
        self.summary.dead_actions += 1;

        let mut friction = Vec::new();

        match self.dead_spots.entry(call.target.clone()) {
            Entry::Vacant(slot) => {
                // One dead click is ordinary — the screen may simply not
                // react there. Only the repeat is a pattern.
                slot.insert(Run::started_by(call));
            }
            Entry::Occupied(mut slot) => {
                let spot = slot.get_mut();
                spot.extend(call);
                friction.push(spot.to_friction(DEAD_SPOT, call, &observed.action));
            }
        }

        let streak = match &mut self.streak {
            Some(streak) => {
                streak.extend(call);
                streak
            }
            slot => slot.insert(Run::started_by(call)),
        };
        // Every third dead action rather than only the first: a streak of ten
        // is a different story from a streak of three, and a single line at
        // the threshold would tell them apart only by reading every call.
        if streak.count >= DEAD_STREAK_THRESHOLD
            && streak.count.is_multiple_of(DEAD_STREAK_THRESHOLD)
        {
            friction.push(streak.to_friction(DEAD_STREAK, call, &observed.action));
        }

        self.summary.friction += friction.len() as u64;
        friction
    }

    /// True once the session has produced at least one call — the point at
    /// which it is worth summarising at all.
    pub(super) fn has_calls(&self) -> bool {
        self.seq > 0
    }

    /// Seal the totals. Consumes the state: a summary is emitted once.
    pub(super) fn finish(mut self) -> Summary {
        self.summary.duration_ms = self
            .last_seen
            .saturating_duration_since(self.started)
            .as_millis() as u64;
        self.summary
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const CLICK_1: &str = r#"mouse_click{"x":1,"y":1}"#;
    const CLICK_5: &str = r#"mouse_click{"x":5,"y":5}"#;
    const CLICK_10: &str = r#"mouse_click{"x":10,"y":10}"#;
    const CLICK_759: &str = r#"mouse_click{"x":759,"y":154}"#;

    fn state() -> SessionState {
        SessionState::new(Instant::now())
    }

    fn input_call(
        state: &mut SessionState,
        target: &str,
        action: &str,
        changed: bool,
        server_ms: u64,
    ) -> Call {
        Call {
            seq: state.next_seq(),
            tool: "mouse_click".into(),
            target: target.into(),
            server_ms,
            gap_ms: None,
            outcome: Outcome::Ok,
            result_bytes: 60,
            image_bytes: 0,
            observed: Some(Observed {
                action: action.into(),
                screen_changed: changed,
            }),
        }
    }

    fn plain_call(state: &mut SessionState, tool: &str) -> Call {
        Call {
            seq: state.next_seq(),
            tool: tool.into(),
            target: tool.into(),
            server_ms: 300,
            gap_ms: None,
            outcome: Outcome::Ok,
            result_bytes: 58_000,
            image_bytes: 57_000,
            observed: None,
        }
    }

    fn record(state: &mut SessionState, call: &Call) -> Vec<Friction> {
        state.record(call, Instant::now())
    }

    #[test]
    fn a_landing_action_reports_no_friction() {
        let mut state = state();
        let call = input_call(&mut state, CLICK_10, "Clicked left at (10, 10)", true, 150);
        assert!(record(&mut state, &call).is_empty());
    }

    #[test]
    fn the_same_dead_target_twice_is_a_dead_spot() {
        // The exact shape of the Firefox permission dialog in the session
        // this was built from: one click that did nothing, then the identical
        // click again.
        let mut state = state();
        let first = input_call(
            &mut state,
            CLICK_759,
            "Clicked left at (759, 154)",
            false,
            2000,
        );
        assert!(
            record(&mut state, &first).is_empty(),
            "one dead click is ordinary — the screen may simply not react",
        );

        let second = input_call(
            &mut state,
            CLICK_759,
            "Clicked left at (759, 154)",
            false,
            2069,
        );
        let friction = record(&mut state, &second);
        assert_eq!(friction.len(), 1);
        assert_eq!(friction[0].kind, DEAD_SPOT);
        assert_eq!(friction[0].count, 2);
        assert_eq!((friction[0].first_seq, friction[0].last_seq), (1, 2));
        assert_eq!(
            friction[0].wasted_ms, 4069,
            "the cost reported is the sum of both attempts",
        );
    }

    #[test]
    fn a_dead_spot_survives_the_action_text_being_reworded() {
        // The identity is the tool and its arguments, not the sentence shown
        // to the agent. Without this, a copy edit to input's message would
        // switch dead-spot detection off with nothing failing to say so.
        let mut state = state();
        let first = input_call(
            &mut state,
            CLICK_759,
            "Clicked left at (759, 154)",
            false,
            100,
        );
        record(&mut state, &first);

        let second = input_call(&mut state, CLICK_759, "Left-clicked (759, 154)", false, 100);
        let friction = record(&mut state, &second);
        assert_eq!(friction.len(), 1, "same target, reworded prose, same spot");
        assert_eq!(friction[0].kind, DEAD_SPOT);
    }

    #[test]
    fn two_targets_that_read_alike_are_not_the_same_spot() {
        // The mirror case: identical prose from different arguments must not
        // be collapsed into one target.
        let mut state = state();
        let first = input_call(&mut state, CLICK_1, "Clicked", false, 100);
        record(&mut state, &first);

        let second = input_call(&mut state, CLICK_5, "Clicked", false, 100);
        assert!(
            record(&mut state, &second)
                .iter()
                .all(|f| f.kind != DEAD_SPOT),
            "different coordinates are different targets",
        );
    }

    #[test]
    fn three_different_dead_actions_are_a_dead_streak() {
        let mut state = state();
        let calls = [
            input_call(
                &mut state,
                "key_press{\"keys\":\"return\"}",
                "Pressed return",
                false,
                2000,
            ),
            input_call(
                &mut state,
                "mouse_click{\"x\":798,\"y\":616}",
                "Clicked left at (798, 616)",
                false,
                2043,
            ),
            input_call(
                &mut state,
                "mouse_click{\"x\":1131,\"y\":913}",
                "Clicked left at (1131, 913)",
                false,
                2003,
            ),
        ];
        let mut friction = Vec::new();
        for call in &calls {
            friction.extend(record(&mut state, call));
        }

        assert_eq!(friction.len(), 1, "distinct targets are not dead spots");
        assert_eq!(friction[0].kind, DEAD_STREAK);
        assert_eq!(friction[0].count, 3);
        assert_eq!(friction[0].wasted_ms, 6046);
    }

    #[test]
    fn a_screenshot_between_dead_clicks_does_not_break_the_streak() {
        // Looking at the screen is what an agent does *while* stuck; treating
        // it as recovery would hide every real streak, because the tool
        // instructions demand a screenshot after each action.
        let mut state = state();
        let mut friction = Vec::new();
        for index in 0..3 {
            let call = input_call(
                &mut state,
                &format!("key_press{{{index}}}"),
                &format!("Pressed key {index}"),
                false,
                2000,
            );
            friction.extend(record(&mut state, &call));
            let look = plain_call(&mut state, "screen_shot");
            friction.extend(record(&mut state, &look));
        }
        assert_eq!(friction.len(), 1);
        assert_eq!(friction[0].kind, DEAD_STREAK);
    }

    #[test]
    fn an_action_that_lands_clears_the_streak() {
        let mut state = state();
        for index in 0..2 {
            let call = input_call(
                &mut state,
                &format!("key_press{{{index}}}"),
                &format!("Pressed key {index}"),
                false,
                100,
            );
            assert!(record(&mut state, &call).is_empty());
        }

        let landed = input_call(&mut state, CLICK_5, "Clicked left at (5, 5)", true, 100);
        record(&mut state, &landed);

        let dead = input_call(
            &mut state,
            "key_press{\"keys\":\"x\"}",
            "Pressed something",
            false,
            100,
        );
        assert!(
            record(&mut state, &dead).is_empty(),
            "the streak restarts from one after the screen moves",
        );
    }

    #[test]
    fn a_long_streak_reports_again_as_it_grows() {
        let mut state = state();
        let mut counts = Vec::new();
        for index in 0..6 {
            let call = input_call(
                &mut state,
                &format!("key_press{{{index}}}"),
                &format!("Pressed key {index}"),
                false,
                100,
            );
            counts.extend(record(&mut state, &call).into_iter().map(|f| f.count));
        }
        assert_eq!(counts, [3, 6], "one line at three dead actions, one at six");
    }

    #[test]
    fn the_summary_totals_every_call() {
        let mut state = state();
        let shot = plain_call(&mut state, "screen_shot");
        record(&mut state, &shot);
        let click = input_call(&mut state, CLICK_1, "Clicked left at (1, 1)", false, 2000);
        record(&mut state, &click);

        let summary = state.finish();
        assert_eq!(summary.calls, 2);
        assert_eq!(summary.input_actions, 1);
        assert_eq!(summary.dead_actions, 1);
        assert_eq!(summary.image_bytes, 57_000);
        assert_eq!(summary.by_tool["screen_shot"], 1);
        assert_eq!(summary.by_tool["mouse_click"], 1);
    }

    #[test]
    fn errors_and_cancellations_are_counted_apart() {
        let mut state = state();
        let mut failed = plain_call(&mut state, "app_launch");
        failed.outcome = Outcome::Error;
        record(&mut state, &failed);

        let mut abandoned = plain_call(&mut state, "screen_shot");
        abandoned.outcome = Outcome::Cancelled;
        record(&mut state, &abandoned);

        let summary = state.finish();
        assert_eq!(summary.errors, 1);
        assert_eq!(summary.cancelled, 1);
    }

    #[test]
    fn the_first_call_has_no_gap_to_report() {
        let mut state = state();
        assert!(state.gap_since_last_call(Instant::now()).is_none());

        let call = plain_call(&mut state, "screen_shot");
        record(&mut state, &call);
        assert!(
            state.gap_since_last_call(Instant::now()).is_some(),
            "once a call has returned there is a gap to measure",
        );
    }

    #[test]
    fn a_call_with_no_feedback_is_not_an_input_action() {
        let mut state = state();
        let call = plain_call(&mut state, "clipboard_get");
        assert!(record(&mut state, &call).is_empty());
        assert_eq!(state.finish().input_actions, 0);
    }
}
