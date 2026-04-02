use std::collections::HashMap;
use std::sync::{RwLock, RwLockReadGuard};
use crate::builder::ContainerSpec;
use ow_core_traits::*;

pub enum ContainerSlot {
    Pending(ContainerSpec),
    Live(ContainerHandle),
}

pub struct ContainerHandle {
    pub id: String,
    pub spec: ContainerSpec,
    pub state: crate::state::ContainerState,
    pub isolation: IsolationHandle,
    pub filesystem: FilesystemHandle,
    pub network: NetworkHandle,
    pub process: ProcessHandle,
    pub log_store: crate::log_store::LogStore,
}

pub type ContainerId = String;

pub struct ContainerRegistry {
    inner: RwLock<HashMap<ContainerId, ContainerSlot>>,
}

impl ContainerRegistry {
    pub fn new() -> Self {
        Self {
            inner: RwLock::new(HashMap::new()),
        }
    }

    pub fn register(&self, spec: ContainerSpec) -> ContainerId {
        let id = generate_container_id();
        let mut map = self.inner.write().unwrap();
        map.insert(id.clone(), ContainerSlot::Pending(spec));
        id
    }

    pub fn get(&self, id: &str) -> Option<ContainerSlotRef<'_>> {
        let map = self.inner.read().unwrap();
        if map.contains_key(id) {
            Some(ContainerSlotRef {
                _guard: map,
                id: id.to_string(),
            })
        } else {
            None
        }
    }

    pub fn take_pending(&self, id: &str) -> Option<ContainerSpec> {
        let mut map = self.inner.write().unwrap();
        match map.get(id)? {
            ContainerSlot::Pending(_) => {
                if let Some(ContainerSlot::Pending(spec)) = map.remove(id) {
                    Some(spec)
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    pub fn promote(&self, id: ContainerId, handle: ContainerHandle) {
        let mut map = self.inner.write().unwrap();
        map.insert(id, ContainerSlot::Live(handle));
    }

    pub fn remove(&self, id: &str) -> bool {
        let mut map = self.inner.write().unwrap();
        map.remove(id).is_some()
    }

    pub fn list(&self) -> Vec<ContainerId> {
        let map = self.inner.read().unwrap();
        map.keys().cloned().collect()
    }
}

pub struct ContainerSlotRef<'a> {
    _guard: RwLockReadGuard<'a, HashMap<ContainerId, ContainerSlot>>,
    id: String,
}

impl<'a> std::ops::Deref for ContainerSlotRef<'a> {
    type Target = ContainerSlot;
    fn deref(&self) -> &Self::Target {
        self._guard.get(&self.id).unwrap()
    }
}

fn generate_container_id() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let ts = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
    format!("ow-{:x}", ts)
}

impl Default for ContainerRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ow_core_traits::*;

    fn test_spec() -> ContainerSpec {
        ContainerSpec {
            image: ImageSpec { reference: "alpine".into() },
            isolation: IsolationConfig { hostname: "test".into(), namespaces: vec![] },
            filesystem: FilesystemConfig { rootfs_path: "/tmp".into(), readonly: false },
            network: NetworkConfig { enabled: false },
            process: ProcessConfig { args: vec!["echo".into()], env: vec![], working_dir: "/".into() },
        }
    }

    #[test]
    fn register_and_get() {
        let registry = ContainerRegistry::new();
        let id = registry.register(test_spec());
        assert!(registry.get(&id).is_some());
    }

    #[test]
    fn register_creates_pending_slot() {
        let registry = ContainerRegistry::new();
        let id = registry.register(test_spec());
        let slot = registry.get(&id).unwrap();
        assert!(matches!(&*slot, ContainerSlot::Pending(_)));
    }

    #[test]
    fn list_returns_all() {
        let registry = ContainerRegistry::new();
        registry.register(test_spec());
        registry.register(test_spec());
        registry.register(test_spec());
        assert_eq!(registry.list().len(), 3);
    }

    #[test]
    fn remove_reduces_count() {
        let registry = ContainerRegistry::new();
        let id = registry.register(test_spec());
        registry.register(test_spec());
        assert_eq!(registry.list().len(), 2);
        assert!(registry.remove(&id));
        assert_eq!(registry.list().len(), 1);
    }

    #[test]
    fn get_nonexistent_returns_none() {
        let registry = ContainerRegistry::new();
        assert!(registry.get("nonexistent").is_none());
    }
}
