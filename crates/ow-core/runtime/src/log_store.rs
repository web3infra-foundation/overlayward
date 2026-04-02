use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

const DEFAULT_MAX_LINES: usize = 10_000;

#[derive(Clone)]
pub struct LogStore {
    inner: Arc<Mutex<LogStoreInner>>,
}

struct LogStoreInner {
    lines: VecDeque<LogLine>,
    max_lines: usize,
}

#[derive(Debug, Clone)]
pub struct LogLine {
    pub stream: LogStream,
    pub content: String,
    pub timestamp: std::time::SystemTime,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LogStream {
    Stdout,
    Stderr,
}

impl LogStore {
    pub fn new() -> Self {
        Self::with_capacity(DEFAULT_MAX_LINES)
    }

    pub fn with_capacity(max_lines: usize) -> Self {
        Self {
            inner: Arc::new(Mutex::new(LogStoreInner {
                lines: VecDeque::with_capacity(max_lines.min(1024)),
                max_lines,
            })),
        }
    }

    pub fn push(&self, stream: LogStream, content: String) {
        let mut inner = self.inner.lock().unwrap();
        if inner.lines.len() >= inner.max_lines {
            inner.lines.pop_front();
        }
        inner.lines.push_back(LogLine {
            stream,
            content,
            timestamp: std::time::SystemTime::now(),
        });
    }

    pub fn tail(&self, n: Option<usize>) -> Vec<LogLine> {
        let inner = self.inner.lock().unwrap();
        match n {
            Some(n) => inner.lines.iter().rev().take(n).rev().cloned().collect(),
            None => inner.lines.iter().cloned().collect(),
        }
    }

    pub fn len(&self) -> usize {
        self.inner.lock().unwrap().lines.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl Default for LogStore {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn push_and_read() {
        let store = LogStore::new();
        store.push(LogStream::Stdout, "line1".into());
        store.push(LogStream::Stderr, "line2".into());
        let lines = store.tail(None);
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0].content, "line1");
        assert_eq!(lines[1].content, "line2");
    }

    #[test]
    fn ringbuffer_eviction() {
        let store = LogStore::with_capacity(3);
        store.push(LogStream::Stdout, "a".into());
        store.push(LogStream::Stdout, "b".into());
        store.push(LogStream::Stdout, "c".into());
        store.push(LogStream::Stdout, "d".into());
        let lines = store.tail(None);
        assert_eq!(lines.len(), 3);
        assert_eq!(lines[0].content, "b");
        assert_eq!(lines[2].content, "d");
    }

    #[test]
    fn tail_n() {
        let store = LogStore::new();
        for i in 0..10 {
            store.push(LogStream::Stdout, format!("line{}", i));
        }
        let last3 = store.tail(Some(3));
        assert_eq!(last3.len(), 3);
        assert_eq!(last3[0].content, "line7");
    }
}
