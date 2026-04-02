use std::any::Any;
use std::fmt;

pub struct IsolationHandle {
    pub(crate) inner: Box<dyn Any + Send + Sync>,
}

impl IsolationHandle {
    pub fn new<T: Any + Send + Sync>(inner: T) -> Self {
        Self { inner: Box::new(inner) }
    }
    pub fn downcast_ref<T: Any>(&self) -> Option<&T> {
        self.inner.downcast_ref()
    }
}

impl fmt::Debug for IsolationHandle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("IsolationHandle").finish_non_exhaustive()
    }
}

pub struct FilesystemHandle {
    pub(crate) inner: Box<dyn Any + Send + Sync>,
}

impl FilesystemHandle {
    pub fn new<T: Any + Send + Sync>(inner: T) -> Self {
        Self { inner: Box::new(inner) }
    }
    pub fn downcast_ref<T: Any>(&self) -> Option<&T> {
        self.inner.downcast_ref()
    }
}

impl fmt::Debug for FilesystemHandle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FilesystemHandle").finish_non_exhaustive()
    }
}

pub struct NetworkHandle {
    pub(crate) inner: Box<dyn Any + Send + Sync>,
}

impl NetworkHandle {
    pub fn new<T: Any + Send + Sync>(inner: T) -> Self {
        Self { inner: Box::new(inner) }
    }
    pub fn downcast_ref<T: Any>(&self) -> Option<&T> {
        self.inner.downcast_ref()
    }
}

impl fmt::Debug for NetworkHandle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("NetworkHandle").finish_non_exhaustive()
    }
}

pub struct ProcessHandle {
    pub(crate) inner: Box<dyn Any + Send + Sync>,
}

impl ProcessHandle {
    pub fn new<T: Any + Send + Sync>(inner: T) -> Self {
        Self { inner: Box::new(inner) }
    }
    pub fn downcast_ref<T: Any>(&self) -> Option<&T> {
        self.inner.downcast_ref()
    }
}

impl fmt::Debug for ProcessHandle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ProcessHandle").finish_non_exhaustive()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ExitStatus {
    pub code: Option<i32>,
    pub signal: Option<i32>,
}

impl ExitStatus {
    pub fn success() -> Self {
        Self { code: Some(0), signal: None }
    }
    pub fn exited(code: i32) -> Self {
        Self { code: Some(code), signal: None }
    }
    pub fn signaled(signal: i32) -> Self {
        Self { code: None, signal: Some(signal) }
    }
    pub fn is_success(&self) -> bool {
        self.code == Some(0)
    }
}
