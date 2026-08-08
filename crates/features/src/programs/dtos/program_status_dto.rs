use schemars::JsonSchema;
use serde::Serialize;

use crate::programs::service::ProgramStatus;

/// What `app_status` answers with.
#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct ProgramStatusDto {
    pub pid: u32,
    pub running: bool,
    pub log_file: String,
    pub tail: String,
}

impl From<ProgramStatus> for ProgramStatusDto {
    fn from(status: ProgramStatus) -> Self {
        Self {
            pid: status.pid,
            running: status.running,
            log_file: status.log_file,
            tail: status.tail,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_published_status_shape_is_what_clients_already_read() {
        let status = ProgramStatusDto::from(ProgramStatus {
            pid: 2737,
            running: false,
            log_file: "/tmp/ghostdesk/proc-2737.log".into(),
            tail: "segfault".into(),
        });
        assert_eq!(
            serde_json::to_value(&status).unwrap(),
            serde_json::json!({
                "pid": 2737,
                "running": false,
                "log_file": "/tmp/ghostdesk/proc-2737.log",
                "tail": "segfault",
            }),
        );
    }
}
