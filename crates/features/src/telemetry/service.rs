//! The seam that turns tool calls into log events.
//!
//! It writes nothing of its own — every fact leaves through `tracing`, the
//! subscriber the framework already installs, so it lands wherever the
//! operator's logs land and an OpenTelemetry layer can export it unchanged.
//!
//! What it adds is the part a log line cannot compute for itself: a call's
//! position in the session, the wait before it, and the running totals that
//! only mean something once the session is over.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, Instant};

use nest_rs::core::{hooks, injectable};
use nest_rs::schedule::scheduled;
use serde_json::{Map, Value};

use super::config::TelemetryConfig;
use super::session::{Call, Friction, Observed, Outcome, State};

/// The fallback session name for a transport that does not carry one.
///
/// Grouping everything under one name is right rather than inventing an id
/// per call: without a session header there is exactly one conversation, and
/// splitting it would make the trajectory unreconstructable.
const UNSESSIONED: &str = "unsessioned";

const TARGET: &str = "features::telemetry";

tokio::task_local! {
    /// The session id, installed for every MCP operation by the endpoint's
    /// per-call context. Task-local rather than global for the same reason
    /// the model space is: two clients' operations interleave.
    static SESSION: Arc<str>;

    /// The call in flight, installed by [`CallGuard::scope`]. This is what
    /// lets [`note_action`] reach the right call from inside a service with
    /// no argument threading.
    static CALL: Arc<CallScope>;
}

struct CallScope {
    seq: u64,
    observed: Mutex<Option<Observed>>,
}

/// Report what an input action did, from wherever observed it.
///
/// The verdict is logged where it is produced; this hands the same two facts
/// to the session accumulator, which is the only thing that can see a *second*
/// identical failure and call it a pattern.
///
/// A no-op outside a tool call, so a service may report unconditionally
/// without caring whether it was reached from MCP or from a test.
pub fn note_action(action: &str, screen_changed: bool) {
    let _ = CALL.try_with(|scope| {
        *lock(&scope.observed) = Some(Observed {
            action: action.to_string(),
            screen_changed,
        });
    });
}

/// How a call ended, as the MCP host measured it.
pub struct CallOutcome {
    pub outcome: Outcome,
    /// The message the client was given, when it was refused. Failures that
    /// go through `failed` are already logged where they happen; an
    /// `invalid_params` refusal is not, and "which arguments do agents get
    /// wrong" is worth being able to ask.
    pub error: Option<String>,
    /// Bytes of the result as the client receives it.
    pub result_bytes: usize,
    /// The base64 image share of `result_bytes`.
    pub image_bytes: usize,
}

impl CallOutcome {
    /// A call that returned nothing measurable — refused, cancelled, or
    /// answered with a payload that has not been produced yet.
    pub fn empty(outcome: Outcome) -> Self {
        Self {
            outcome,
            error: None,
            result_bytes: 0,
            image_bytes: 0,
        }
    }

    pub fn failed(error: impl std::fmt::Display) -> Self {
        Self {
            error: Some(error.to_string()),
            ..Self::empty(Outcome::Error)
        }
    }
}

/// The live sessions, keyed by the MCP session id.
type Sessions = HashMap<Arc<str>, State>;

/// A shared handle on the live sessions.
///
/// A newtype rather than a bare `Arc<…>` field because `#[injectable]`
/// rejects those on sight — an un-injected `Arc` is nearly always a
/// dependency someone forgot to wire, and would be silently defaulted. This
/// one is the service's own interior, not a dependency, and the wrapper is
/// how that intent is stated to the container. Guards hold a clone of it, so
/// a cancelled call can still report from its `Drop`.
#[derive(Clone, Default)]
struct Core(Arc<Mutex<Sessions>>);

#[injectable]
pub struct TelemetryService {
    #[inject]
    config: Arc<TelemetryConfig>,
    state: Core,
}

#[hooks]
impl TelemetryService {
    /// Summarise every live session before the process goes away.
    ///
    /// Without this a server restarted mid-session loses that session's
    /// totals entirely — and a restart is exactly when someone is about to go
    /// looking for them.
    #[on_application_shutdown]
    async fn close(&self) {
        self.summarise(Duration::ZERO);
    }
}

#[scheduled]
impl TelemetryService {
    /// Close and log sessions that have gone quiet.
    ///
    /// MCP has no goodbye — a client simply stops calling — so silence is the
    /// only signal a session is over. The cadence is deliberately shorter
    /// than the threshold it enforces: the sweep is a scan over a handful of
    /// entries, and a summary is more useful the sooner it lands after the
    /// run it describes.
    #[every("30s")]
    async fn summarise_quiet_sessions(&self) -> anyhow::Result<()> {
        self.summarise(Duration::from_secs(self.config.session_idle_secs));
        Ok(())
    }
}

impl TelemetryService {
    /// Install the session for the operation about to run, and mark it live.
    ///
    /// Wraps the whole operation rather than being a bare setter: the id has
    /// to be readable from inside rmcp's dispatch, which is a different task
    /// from the one that read the header.
    pub async fn with_session<F: std::future::Future>(&self, session: &str, inner: F) -> F::Output {
        let session: Arc<str> = Arc::from(if session.is_empty() {
            UNSESSIONED
        } else {
            session
        });

        // Traffic of any kind keeps a session alive — a client reading a
        // resource between two tool calls is still someone at the desk, and
        // the idle sweep must not summarise it out from under them.
        if let Some(state) = lock(&self.state.0).get_mut(&session) {
            state.last_seen = Instant::now();
        }

        SESSION.scope(session, inner).await
    }

    /// Open a guard around one tool call.
    ///
    /// The returned guard is inert when no session is installed, so the
    /// caller never branches.
    pub fn begin(&self, tool: &str, args: Option<&Map<String, Value>>) -> CallGuard {
        let Ok(session) = SESSION.try_with(Arc::clone) else {
            return CallGuard(None);
        };

        let started = Instant::now();
        let mut sessions = lock(&self.state.0);
        let state = sessions
            .entry(Arc::clone(&session))
            .or_insert_with(|| State::new(started));

        let seq = state.next_seq();
        let gap_ms = state.gap_since_last_call(started);
        drop(sessions);

        CallGuard(Some(Active {
            state: self.state.clone(),
            scope: Arc::new(CallScope {
                seq,
                observed: Mutex::new(None),
            }),
            session,
            tool: tool.to_string(),
            target: target_of(tool, args),
            started,
            gap_ms,
        }))
    }

    /// Close and log every session silent for at least `idle_after`.
    ///
    /// `Duration::ZERO` takes every session, which is what shutdown wants.
    /// Extraction and logging are separate steps on purpose: the lock is
    /// released before the first line is formatted.
    fn summarise(&self, idle_after: Duration) -> usize {
        let now = Instant::now();
        let closing: Vec<(Arc<str>, State)> = lock(&self.state.0)
            .extract_if(|_, state| {
                state.has_calls() && now.saturating_duration_since(state.last_seen) >= idle_after
            })
            .collect();

        let closed = closing.len();
        for (session, state) in closing {
            let summary = state.finish();
            // `info`, unlike the per-call `debug`: one line per session is
            // cheap enough to keep on in production, and it is the line that
            // makes two runs comparable.
            tracing::info!(
                target: TARGET,
                session = %session,
                calls = summary.calls,
                errors = summary.errors,
                cancelled = summary.cancelled,
                duration_ms = summary.duration_ms,
                server_ms = summary.server_ms,
                gap_ms = summary.gap_ms,
                result_bytes = summary.result_bytes,
                image_bytes = summary.image_bytes,
                input_actions = summary.input_actions,
                dead_actions = summary.dead_actions,
                friction = summary.friction,
                by_tool = ?summary.by_tool,
                "session summary",
            );
        }

        closed
    }
}

/// One tool call's lifetime.
///
/// `None` outside a session — every method is then a branch and a return.
pub struct CallGuard(Option<Active>);

struct Active {
    state: Core,
    scope: Arc<CallScope>,
    session: Arc<str>,
    tool: String,
    target: String,
    started: Instant,
    gap_ms: Option<u64>,
}

impl CallGuard {
    /// This call's position in the session, or `0` when it is not being
    /// tracked. It is what ties the events emitted underneath a call to the
    /// call's own line.
    pub fn seq(&self) -> u64 {
        self.0.as_ref().map_or(0, |active| active.scope.seq)
    }

    /// Run the dispatch with this call installed as the ambient one.
    pub async fn scope<F: std::future::Future>(&self, inner: F) -> F::Output {
        match &self.0 {
            Some(active) => CALL.scope(Arc::clone(&active.scope), inner).await,
            None => inner.await,
        }
    }

    /// Log the call. Consuming `self` is what makes "finished" and
    /// "cancelled" mutually exclusive without a flag: after this, `Drop` has
    /// nothing left to report.
    pub fn finish(mut self, outcome: CallOutcome) {
        if let Some(active) = self.0.take() {
            active.emit(outcome);
        }
    }
}

impl Drop for CallGuard {
    /// A guard reaching here still holding its call means the dispatch was
    /// dropped before it returned — the client cancelled, or the connection
    /// went away. That is a fact about the product (something took long
    /// enough that someone gave up) and it is invisible in every other log,
    /// so it is reported like any other outcome.
    fn drop(&mut self) {
        if let Some(active) = self.0.take() {
            active.emit(CallOutcome::empty(Outcome::Cancelled));
        }
    }
}

impl Active {
    fn emit(self, outcome: CallOutcome) {
        let finished_at = Instant::now();
        let call = Call {
            seq: self.scope.seq,
            tool: self.tool,
            target: self.target,
            server_ms: finished_at
                .saturating_duration_since(self.started)
                .as_millis() as u64,
            gap_ms: self.gap_ms,
            outcome: outcome.outcome,
            result_bytes: outcome.result_bytes,
            image_bytes: outcome.image_bytes,
            observed: lock(&self.scope.observed).take(),
        };

        // Self-contained rather than leaning on the enclosing span: this is
        // the line an analysis groups by, and a `Drop` on a cancelled future
        // is not guaranteed to run with that span entered.
        tracing::debug!(
            target: TARGET,
            session = %self.session,
            seq = call.seq,
            tool = %call.tool,
            server_ms = call.server_ms,
            gap_ms = call.gap_ms,
            outcome = call.outcome.as_str(),
            error = outcome.error.as_deref(),
            result_bytes = call.result_bytes,
            image_bytes = call.image_bytes,
            screen_changed = call.observed.as_ref().map(|seen| seen.screen_changed),
            "tool call",
        );

        // The session may already have been swept — a call that outlives the
        // idle threshold is exactly the kind worth catching — in which case
        // the line above still stands, just without totals behind it.
        let friction: Vec<Friction> = lock(&self.state.0)
            .get_mut(&self.session)
            .map(|state| state.record(&call, finished_at))
            .unwrap_or_default();

        for pattern in friction {
            tracing::warn!(
                target: TARGET,
                session = %self.session,
                kind = pattern.kind,
                tool = %pattern.tool,
                target_action = %pattern.target,
                count = pattern.count,
                wasted_ms = pattern.wasted_ms,
                first_seq = pattern.first_seq,
                last_seq = pattern.last_seq,
                "the agent is repeating an action that changes nothing",
            );
        }
    }
}

/// The stable identity of what a call aimed at.
///
/// Built from the tool name and the wire arguments, never from the
/// agent-facing prose. The prose is display text owned by another module and
/// free to be reworded; keying pattern detection on it would let a copy edit
/// silently switch the detection off, with nothing failing to say so. The
/// arguments are the published contract and cannot move underneath us.
///
/// Free text is reduced to a character count: this key is held for the life
/// of the session, and a map that outlives the call must never be the thing
/// holding a password. Two different texts of the same length therefore share
/// an identity — which is the right grouping anyway, since "typing is not
/// landing" is one pattern, not one per phrase.
fn target_of(tool: &str, args: Option<&Map<String, Value>>) -> String {
    let mut args = args.cloned().unwrap_or_default();
    if let Some(Value::String(text)) = args.remove("text") {
        args.insert("text_chars".into(), Value::from(text.chars().count()));
    }
    // `serde_json::Map` is a `BTreeMap`, so the rendering is key-sorted and
    // two calls with the same arguments always produce the same identity.
    format!("{tool}{}", Value::Object(args))
}

/// Take a lock, ignoring poisoning.
///
/// A panic while holding one of these leaves a counter half-updated, nothing
/// more. Refusing to record anything afterwards — which is what `unwrap`
/// would do — turns a cosmetic inconsistency into a permanent loss.
fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn service(session_idle_secs: u64) -> TelemetryService {
        TelemetryService {
            config: Arc::new(TelemetryConfig { session_idle_secs }),
            state: Core::default(),
        }
    }

    fn ok() -> CallOutcome {
        CallOutcome {
            outcome: Outcome::Ok,
            error: None,
            result_bytes: 60,
            image_bytes: 0,
        }
    }

    #[tokio::test]
    async fn calls_outside_a_session_are_not_tracked() {
        // Every entry point installs a session, but a unit test or a future
        // transport might not — and that must cost nothing rather than panic.
        let telemetry = service(120);
        let call = telemetry.begin("screen_shot", None);
        assert_eq!(call.seq(), 0);
        assert_eq!(call.scope(async { 42 }).await, 42);
        call.finish(ok());

        assert!(lock(&telemetry.state.0).is_empty());
    }

    #[tokio::test]
    async fn a_session_numbers_its_calls_and_measures_the_wait_between_them() {
        let telemetry = service(120);
        telemetry
            .with_session("sess-1", async {
                let first = telemetry.begin("screen_shot", None);
                assert_eq!(first.seq(), 1);
                first.finish(ok());

                let second = telemetry.begin("mouse_click", None);
                assert_eq!(second.seq(), 2);
                assert!(
                    second.0.as_ref().unwrap().gap_ms.is_some(),
                    "every call after the first measures the wait before it",
                );
                second.finish(ok());
            })
            .await;

        assert_eq!(lock(&telemetry.state.0).len(), 1);
    }

    #[tokio::test]
    async fn what_a_service_observed_reaches_the_accumulator() {
        let telemetry = service(120);
        telemetry
            .with_session("sess-2", async {
                for _ in 0..2 {
                    let call = telemetry.begin("mouse_click", None);
                    call.scope(async {
                        note_action("Clicked left at (200, 830)", false);
                    })
                    .await;
                    call.finish(ok());
                }
            })
            .await;

        let mut sessions = lock(&telemetry.state.0);
        let summary = sessions.remove("sess-2").unwrap().finish();
        assert_eq!(summary.input_actions, 2);
        assert_eq!(summary.dead_actions, 2);
        assert_eq!(
            summary.friction, 1,
            "the second identical dead click is the pattern",
        );
    }

    #[tokio::test]
    async fn noting_an_action_outside_a_call_is_a_no_op() {
        note_action("Clicked left at (1, 1)", true);
    }

    #[tokio::test]
    async fn a_quiet_session_is_summarised_and_a_busy_one_is_left_alone() {
        let telemetry = service(0);
        telemetry
            .with_session("sess-3", async {
                telemetry.begin("screen_shot", None).finish(ok());
            })
            .await;

        assert_eq!(telemetry.summarise(Duration::ZERO), 1);
        assert!(
            lock(&telemetry.state.0).is_empty(),
            "a summarised session is forgotten, not summarised twice",
        );
        assert_eq!(telemetry.summarise(Duration::ZERO), 0);
    }

    #[test]
    fn typed_text_never_becomes_part_of_a_target() {
        // The identity outlives the call; it must never be the thing holding
        // a password.
        let mut args = Map::new();
        args.insert("text".into(), json!("hunter2 correct horse"));

        let target = target_of("key_type", Some(&args));
        assert!(!target.contains("hunter2"), "the content is dropped");
        assert!(target.contains("text_chars"), "its length identifies it");
    }

    #[test]
    fn the_same_arguments_always_produce_the_same_target() {
        // Key order on the wire is the client's business. If the identity
        // depended on it, a repeated action could read as a fresh target and
        // the pattern would never be seen.
        let one: Map<String, Value> = serde_json::from_str(r#"{"x":200,"y":830}"#).unwrap();
        let two: Map<String, Value> = serde_json::from_str(r#"{"y":830,"x":200}"#).unwrap();
        assert_eq!(
            target_of("mouse_click", Some(&one)),
            target_of("mouse_click", Some(&two)),
        );
    }

    #[test]
    fn different_arguments_and_different_tools_are_different_targets() {
        let here: Map<String, Value> = serde_json::from_str(r#"{"x":200,"y":830}"#).unwrap();
        let there: Map<String, Value> = serde_json::from_str(r#"{"x":201,"y":830}"#).unwrap();
        assert_ne!(
            target_of("mouse_click", Some(&here)),
            target_of("mouse_click", Some(&there)),
        );
        assert_ne!(
            target_of("mouse_click", Some(&here)),
            target_of("mouse_double_click", Some(&here)),
        );
    }

    #[tokio::test]
    async fn a_dispatch_dropped_before_it_returns_is_counted_as_cancelled() {
        let telemetry = service(120);
        telemetry
            .with_session("sess-cancel", async {
                // Exactly what rmcp does on a cancellation: the future holding
                // the guard is dropped and no `finish` is ever reached.
                let call = telemetry.begin("screen_shot", None);
                drop(call);
            })
            .await;

        let summary = lock(&telemetry.state.0)
            .remove("sess-cancel")
            .expect("the session was created by the call")
            .finish();
        assert_eq!(summary.cancelled, 1);
        assert_eq!(summary.calls, 1);
    }

    #[tokio::test]
    async fn a_transport_without_a_session_still_groups_its_calls() {
        let telemetry = service(120);
        telemetry
            .with_session("", async {
                telemetry.begin("screen_shot", None).finish(ok());
            })
            .await;

        assert!(
            lock(&telemetry.state.0).contains_key(UNSESSIONED),
            "a sessionless transport is one conversation, not none",
        );
    }

    #[tokio::test]
    async fn a_session_with_no_calls_is_never_summarised() {
        // A client that connects, lists the tools and leaves has no
        // trajectory to report.
        let telemetry = service(0);
        telemetry.with_session("sess-4", async {}).await;
        assert_eq!(telemetry.summarise(Duration::ZERO), 0);
    }
}
