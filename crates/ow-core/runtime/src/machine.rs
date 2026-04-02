use crate::state::ContainerPhase;
use ow_core_traits::{OwError, Result};

pub fn is_valid_transition(from: &ContainerPhase, to: &ContainerPhase) -> bool {
    matches!(
        (phase_tag(from), phase_tag(to)),
        ("created", "starting")
        | ("starting", "running")
        | ("starting", "failed")
        | ("running", "stopping")
        | ("running", "paused")
        | ("running", "frozen")
        | ("paused", "running")
        | ("paused", "stopping")
        | ("paused", "frozen")
        | ("stopping", "stopped")
        | ("stopped", "removing")
        | ("failed", "removing")
        | ("removing", "removed")
    )
}

pub fn try_transition(from: &ContainerPhase, to: ContainerPhase) -> Result<ContainerPhase> {
    if is_valid_transition(from, &to) {
        Ok(to)
    } else {
        Err(OwError::InvalidTransition {
            from: format!("{:?}", from),
            to: format!("{:?}", to),
        })
    }
}

fn phase_tag(phase: &ContainerPhase) -> &'static str {
    use ContainerPhase::*;
    match phase {
        Created => "created",
        Starting => "starting",
        Running => "running",
        Paused => "paused",
        Stopping => "stopping",
        Stopped { .. } => "stopped",
        Failed { .. } => "failed",
        Frozen { .. } => "frozen",
        Removing => "removing",
        Removed => "removed",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::ContainerPhase;

    #[test]
    fn created_to_starting_is_valid() {
        assert!(is_valid_transition(&ContainerPhase::Created, &ContainerPhase::Starting));
    }

    #[test]
    fn starting_to_running_is_valid() {
        assert!(is_valid_transition(&ContainerPhase::Starting, &ContainerPhase::Running));
    }

    #[test]
    fn starting_to_failed_is_valid() {
        let failed = ContainerPhase::Failed { error: "test".into() };
        assert!(is_valid_transition(&ContainerPhase::Starting, &failed));
    }

    #[test]
    fn running_to_stopping_is_valid() {
        assert!(is_valid_transition(&ContainerPhase::Running, &ContainerPhase::Stopping));
    }

    #[test]
    fn running_to_frozen_is_valid() {
        let frozen = ContainerPhase::Frozen { reason: "emergency".into() };
        assert!(is_valid_transition(&ContainerPhase::Running, &frozen));
    }

    #[test]
    fn stopped_to_removing_is_valid() {
        let stopped = ContainerPhase::Stopped { exit_code: 0 };
        assert!(is_valid_transition(&stopped, &ContainerPhase::Removing));
    }

    #[test]
    fn created_to_running_is_invalid() {
        assert!(!is_valid_transition(&ContainerPhase::Created, &ContainerPhase::Running));
    }

    #[test]
    fn running_to_created_is_invalid() {
        assert!(!is_valid_transition(&ContainerPhase::Running, &ContainerPhase::Created));
    }

    #[test]
    fn removed_to_anything_is_invalid() {
        assert!(!is_valid_transition(&ContainerPhase::Removed, &ContainerPhase::Created));
        assert!(!is_valid_transition(&ContainerPhase::Removed, &ContainerPhase::Starting));
    }
}
