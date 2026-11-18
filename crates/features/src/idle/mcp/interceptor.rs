use std::sync::Arc;

use nest_rs::core::operation_log::Unit;
use nest_rs::core::{Interceptor, Layer, Next, Outcome, UnitContext, async_trait, injectable};
use nest_rs::mcp;

use crate::idle::IdleService;

/// A tool call is what uses the desk, so each one resets the idle clock.
///
/// At the handler stage, so it runs around a declared tool the guards
/// admitted and around nothing else: a listing or a handshake is a client
/// looking at the desk, which keeps no desktop alive, and a refused call never
/// used it.
#[injectable]
pub struct IdleInterceptor {
    #[inject]
    svc: Arc<IdleService>,
}

impl Layer for IdleInterceptor {
    fn serves(&self, unit: Unit) -> bool {
        unit == mcp::unit::OPERATION
    }
}

#[async_trait]
impl Interceptor for IdleInterceptor {
    async fn intercept(&self, cx: &mut UnitContext<'_>, next: Next<'_>) -> Outcome {
        self.svc.mark_activity();
        next.run(cx).await
    }
}
