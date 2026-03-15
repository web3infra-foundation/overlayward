use async_trait::async_trait;
use ow_types::*;
use papaya::HashMap;
use std::sync::Arc;
use tokio::sync::broadcast;

// === Store ===

pub struct AuditStore {
    pub audit_events: HashMap<String, Vec<AuditEvent>>,
    pub event_tx: broadcast::Sender<Event>,
}

impl AuditStore {
    pub fn new() -> Arc<Self> {
        let (event_tx, _) = broadcast::channel(256);
        Arc::new(Self {
            audit_events: HashMap::new(),
            event_tx,
        })
    }
}

pub struct AuditBackend {
    store: Arc<AuditStore>,
}

impl AuditBackend {
    pub fn new(store: Arc<AuditStore>) -> Self {
        Self { store }
    }
}

// === AuditManager ===

#[async_trait]
pub trait AuditManager: Send + Sync + 'static {
    async fn query(&self, query: AuditQuery) -> Result<AuditQueryResult, ApiError>;
    async fn detail(&self, sandbox_id: &str, event_id: &str) -> Result<AuditEvent, ApiError>;
    async fn replay(&self, req: AuditReplayRequest) -> Result<Vec<AuditEvent>, ApiError>;
}

#[async_trait]
impl AuditManager for AuditBackend {
    async fn query(&self, query: AuditQuery) -> Result<AuditQueryResult, ApiError> {
        let guard = self.store.audit_events.pin();
        let events = guard.get(&query.sandbox_id).cloned().unwrap_or_default();
        let total = events.len() as u64;
        let items: Vec<AuditEvent> = events
            .into_iter()
            .skip(query.offset as usize)
            .take(query.limit as usize)
            .collect();
        let has_more = (query.offset as u64 + items.len() as u64) < total;
        Ok(AuditQueryResult { events: items, total, has_more })
    }

    async fn detail(&self, sandbox_id: &str, event_id: &str) -> Result<AuditEvent, ApiError> {
        let guard = self.store.audit_events.pin();
        let events = guard.get(sandbox_id).cloned().unwrap_or_default();
        events
            .into_iter()
            .find(|e| &*e.id == event_id)
            .ok_or_else(|| ApiError::not_found("event", event_id))
    }

    async fn replay(&self, req: AuditReplayRequest) -> Result<Vec<AuditEvent>, ApiError> {
        Ok(self.store.audit_events.pin().get(&req.sandbox_id).cloned().unwrap_or_default())
    }
}

// === EventManager ===

#[async_trait]
pub trait EventManager: Send + Sync + 'static {
    fn subscribe(&self) -> broadcast::Receiver<Event>;
    async fn emit(&self, event: Event);
}

#[async_trait]
impl EventManager for AuditBackend {
    fn subscribe(&self) -> broadcast::Receiver<Event> {
        self.store.event_tx.subscribe()
    }

    async fn emit(&self, event: Event) {
        let _ = self.store.event_tx.send(event);
    }
}
