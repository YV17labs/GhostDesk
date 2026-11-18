//! What the desk demands of a caller. The guard pool is the one `GhostdeskModule`
//! declares, so booting the module boots the gate the binary serves behind —
//! there is no second list here to keep in step with it.
//!
//! These are the refusals. A tool call that succeeds proves the endpoint works;
//! only a call that is turned away proves it is closed, and the guard chain is
//! the one part of this app whose failure is silent: it serves a desktop.

use nest_rs::http::StatusCode;
use nest_rs::testing::TestApp;
use nest_rs::testing::mcp::{call_tool_statelessly, initialize_request, post_message, result};
use serde_json::json;

use features::auth::AuthConfig;
use ghostdesk::GhostdeskModule;

use crate::PATH;

const TOKEN: &str = "a-secret-only-the-operator-has";

/// Seeded rather than read from the environment: the variables are process-wide
/// and every test in this binary shares them, so a suite that sets one decides
/// the posture of the tests running beside it.
async fn desk(token: Option<&str>) -> TestApp {
    TestApp::builder()
        .module::<GhostdeskModule>()
        .provide(AuthConfig {
            token: token.map(Into::into),
        })
        .build()
        .await
        .expect("the composition root assembles")
}

/// Both refusals in one test on purpose: no credential at all and a credential
/// that fails to verify are the same `401` to the caller, and asserting them
/// together is what catches a gate that only checks a header is *present*.
#[tokio::test]
async fn a_gated_desk_refuses_a_missing_and_a_wrong_credential() {
    let app = desk(Some(TOKEN)).await;

    post_message(app.http(), PATH, None, None, &initialize_request())
        .await
        .assert_status(StatusCode::UNAUTHORIZED);

    post_message(
        app.http(),
        PATH,
        None,
        Some("Bearer not-the-token"),
        &initialize_request(),
    )
    .await
    .assert_status(StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn the_right_credential_reaches_the_endpoint() {
    let app = desk(Some(TOKEN)).await;

    post_message(
        app.http(),
        PATH,
        None,
        Some(&format!("Bearer {TOKEN}")),
        &initialize_request(),
    )
    .await
    .assert_status_is_ok();
}

/// RFC 7235 §2.1 makes the auth scheme case-insensitive. Asserted at the
/// endpoint and not only at the strategy, because this is the spelling a
/// conforming client is free to send and a hand-rolled `strip_prefix` refused.
#[tokio::test]
async fn the_scheme_is_accepted_however_a_client_spells_it() {
    let app = desk(Some(TOKEN)).await;

    post_message(
        app.http(),
        PATH,
        None,
        Some(&format!("bearer {TOKEN}")),
        &initialize_request(),
    )
    .await
    .assert_status_is_ok();
}

/// The dev posture, and the one place the desk is deliberately open. It is
/// reachable only on loopback — `AuthService` refuses to boot a routable bind
/// without a token, which is asserted where the postures are decided.
///
/// Called as a `2026-07-28` client calls, with no session: `app_list` reads the
/// catalogue and touches nothing, so the call proves the gate let the caller
/// through without acting on a desktop.
#[tokio::test]
async fn an_ungated_desk_serves_an_anonymous_caller() {
    let app = desk(None).await;

    let body = call_tool_statelessly(app.http(), PATH, "app_list", None, &[], json!({})).await;

    assert!(
        result(&body)["result"]["structuredContent"]["result"].is_array(),
        "the anonymous call reaches the tool: {body}",
    );
}

/// The other half of the open posture: a session is bound to the subject of the
/// credential that opened it, and an anonymous caller names none, so the
/// handshake that would open one is refused — two anonymous callers would
/// otherwise be served each other's session.
#[tokio::test]
async fn an_anonymous_caller_opens_no_session() {
    let app = desk(None).await;

    post_message(app.http(), PATH, None, None, &initialize_request())
        .await
        .assert_status(StatusCode::FORBIDDEN);
}

/// The regression the global pool invites: a gate installed app-wide also
/// covers the probes an orchestrator reads, and a desk nobody can health-check
/// is a desk that gets restarted in a loop.
#[tokio::test]
async fn the_probes_stay_reachable_on_a_gated_desk() {
    let app = desk(Some(TOKEN)).await;

    for probe in ["/health/live", "/health/ready", "/health/startup"] {
        let status = app.http().get(probe).send().await.0.status();
        assert_ne!(
            status,
            StatusCode::UNAUTHORIZED,
            "{probe} must answer an orchestrator carrying no credential",
        );
    }
}
