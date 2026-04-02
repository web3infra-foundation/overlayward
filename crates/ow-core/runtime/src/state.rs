use std::time::SystemTime;

#[derive(Debug, Clone, PartialEq)]
pub enum ContainerPhase {
    Created,
    Starting,
    Running,
    Paused,
    Stopping,
    Stopped { exit_code: i32 },
    Failed { error: String },
    Frozen { reason: String },
    Removing,
    Removed,
}

#[derive(Debug, Clone, PartialEq)]
pub enum StartingSubstate {
    PullingImage,
    MountingRootfs,
    CreatingIsolation,
    ConfiguringCgroup,
    SettingUpNetwork,
    ApplyingSecurity,
    StartingProcess,
}

impl StartingSubstate {
    pub fn ordered() -> Vec<Self> {
        vec![
            Self::PullingImage,
            Self::MountingRootfs,
            Self::CreatingIsolation,
            Self::ConfiguringCgroup,
            Self::SettingUpNetwork,
            Self::ApplyingSecurity,
            Self::StartingProcess,
        ]
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum StoppingSubstate {
    SignalingProcess,
    DrainTimeout,
    ForceKilling,
    UnmountingFilesystem,
    TearingDownNetwork,
    CleaningUpIsolation,
}

impl StoppingSubstate {
    pub fn ordered() -> Vec<Self> {
        vec![
            Self::SignalingProcess,
            Self::DrainTimeout,
            Self::ForceKilling,
            Self::UnmountingFilesystem,
            Self::TearingDownNetwork,
            Self::CleaningUpIsolation,
        ]
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Substate {
    Starting(StartingSubstate),
    Stopping(StoppingSubstate),
}

#[derive(Debug, Clone)]
pub struct ContainerState {
    pub phase: ContainerPhase,
    pub substate: Option<Substate>,
    pub updated_at: SystemTime,
}

impl ContainerState {
    pub fn new() -> Self {
        Self {
            phase: ContainerPhase::Created,
            substate: None,
            updated_at: SystemTime::now(),
        }
    }

    pub fn transition(&mut self, phase: ContainerPhase) {
        self.phase = phase;
        self.substate = None;
        self.updated_at = SystemTime::now();
    }

    pub fn set_substate(&mut self, substate: Substate) {
        self.substate = Some(substate);
        self.updated_at = SystemTime::now();
    }
}

impl Default for ContainerState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn container_phase_created_is_default() {
        let state = ContainerState::new();
        assert_eq!(state.phase, ContainerPhase::Created);
        assert!(state.substate.is_none());
    }

    #[test]
    fn starting_substates_are_ordered() {
        let substates = StartingSubstate::ordered();
        assert_eq!(substates.len(), 7);
        assert_eq!(substates[0], StartingSubstate::PullingImage);
        assert_eq!(substates[6], StartingSubstate::StartingProcess);
    }

    #[test]
    fn stopping_substates_are_ordered() {
        let substates = StoppingSubstate::ordered();
        assert_eq!(substates.len(), 6);
        assert_eq!(substates[0], StoppingSubstate::SignalingProcess);
        assert_eq!(substates[5], StoppingSubstate::CleaningUpIsolation);
    }
}
