use ow_core_traits::ProcessHandle;
use ow_core_runtime::log_store::LogStore;

pub struct LinuxProcess {
    pub pid: i32,
    pub log_store: LogStore,
    log_tasks: std::sync::Mutex<Vec<tokio::task::JoinHandle<()>>>,
}

impl LinuxProcess {
    pub fn new(pid: i32, log_store: LogStore, log_tasks: Vec<tokio::task::JoinHandle<()>>) -> Self {
        Self { pid, log_store, log_tasks: std::sync::Mutex::new(log_tasks) }
    }

    pub async fn drain_logs(&self) {
        let tasks: Vec<_> = {
            let mut guard = self.log_tasks.lock().unwrap();
            guard.drain(..).collect()
        };
        for t in tasks {
            let _ = t.await;
        }
    }
}

pub fn to_handle(proc: LinuxProcess) -> ProcessHandle {
    ProcessHandle::new(proc)
}
