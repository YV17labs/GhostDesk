use std::time::Duration;

use nest_rs::config::{Bound, DurationBounds, Floor, config};

/// Read from `IDLE__TIMEOUT_SECS`. `0` there, and `None` in code, disarm the
/// watchdog: the field is an `Option`, so off is a value rather than a number
/// that happens to mean it.
const TIMEOUT: DurationBounds = DurationBounds::secs(
    Floor::Units(Bound::new(
        1,
        "a desktop idle for under a second is one still in use",
    )),
    Bound::new(
        604_800,
        "past a week a desktop left open is abandoned, not idle",
    ),
);

#[config(namespace = "idle")]
#[derive(Clone, Debug)]
pub struct IdleConfig {
    #[config(bounds = TIMEOUT)]
    pub timeout: Option<Duration>,
}

impl Default for IdleConfig {
    fn default() -> Self {
        Self {
            timeout: Some(Duration::from_secs(1800)),
        }
    }
}
