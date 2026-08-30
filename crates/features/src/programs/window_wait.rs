use super::running_window::RunningWindow;

/// How the wait for a launched program's first window ended.
///
/// Three outcomes rather than `Option`, because the agent's next move differs
/// for each: a window to work with, a process that died (tail its log), or one
/// that is alive without a UI (stop waiting for one).
#[derive(Debug)]
pub enum WindowWait {
    Appeared {
        window: RunningWindow,
        waited_ms: u64,
    },
    ProcessExited {
        waited_ms: u64,
    },
    TimedOut {
        waited_ms: u64,
    },
}
