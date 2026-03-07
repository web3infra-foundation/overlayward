use reqwest::header::{AUTHORIZATION, CONTENT_TYPE};
use serde::de::DeserializeOwned;

pub struct HttpClient {
    inner: reqwest::Client,
    base: String,
    token: String,
}

impl HttpClient {
    pub fn new(endpoint: &str, token: &str) -> Self {
        Self { inner: reqwest::Client::new(), base: format!("{endpoint}/api/v1"), token: token.to_string() }
    }

    #[inline] fn url(&self, path: &str) -> String { format!("{}{path}", self.base) }
    #[inline] fn auth(&self, rb: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        if self.token.is_empty() { rb } else { rb.header(AUTHORIZATION, format!("Bearer {}", self.token)) }
    }

    async fn handle<T: DeserializeOwned>(resp: reqwest::Response) -> Result<T, String> {
        let status = resp.status();
        let body = resp.text().await.map_err(|e| e.to_string())?;
        if !status.is_success() { return Err(format!("[{status}] {body}")); }
        let parsed: serde_json::Value = serde_json::from_str(&body).map_err(|e| e.to_string())?;
        if parsed.get("ok").and_then(|v| v.as_bool()) == Some(true) {
            serde_json::from_value(parsed.get("data").cloned().unwrap_or(serde_json::Value::Null)).map_err(|e| e.to_string())
        } else {
            let code = parsed.pointer("/error/code").and_then(|v| v.as_str()).unwrap_or("UNKNOWN");
            let msg = parsed.pointer("/error/message").and_then(|v| v.as_str()).unwrap_or("unknown error");
            Err(format!("[{code}] {msg}"))
        }
    }

    pub async fn get<T: DeserializeOwned>(&self, path: &str) -> Result<T, String> {
        Self::handle(self.auth(self.inner.get(self.url(path))).send().await.map_err(|e| format!("connection: {e}"))?).await
    }
    pub async fn post<T: DeserializeOwned>(&self, path: &str, body: &impl serde::Serialize) -> Result<T, String> {
        let json = serde_json::to_string(body).map_err(|e| e.to_string())?;
        Self::handle(self.auth(self.inner.post(self.url(path))).header(CONTENT_TYPE, "application/json").body(json).send().await.map_err(|e| format!("connection: {e}"))?).await
    }
    pub async fn post_empty(&self, path: &str) -> Result<(), String> {
        let resp = self.auth(self.inner.post(self.url(path))).send().await.map_err(|e| format!("connection: {e}"))?;
        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.map_err(|e| e.to_string())?;
            return Err(format!("[{status}] {body}"));
        }
        Ok(())
    }
    pub async fn put<T: DeserializeOwned>(&self, path: &str, body: &impl serde::Serialize) -> Result<T, String> {
        let json = serde_json::to_string(body).map_err(|e| e.to_string())?;
        Self::handle(self.auth(self.inner.put(self.url(path))).header(CONTENT_TYPE, "application/json").body(json).send().await.map_err(|e| format!("connection: {e}"))?).await
    }
    pub async fn patch<T: DeserializeOwned>(&self, path: &str, body: &impl serde::Serialize) -> Result<T, String> {
        let json = serde_json::to_string(body).map_err(|e| e.to_string())?;
        Self::handle(self.auth(self.inner.patch(self.url(path))).header(CONTENT_TYPE, "application/json").body(json).send().await.map_err(|e| format!("connection: {e}"))?).await
    }
    pub async fn delete(&self, path: &str) -> Result<(), String> {
        let resp = self.auth(self.inner.delete(self.url(path))).send().await.map_err(|e| format!("connection: {e}"))?;
        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.map_err(|e| e.to_string())?;
            return Err(format!("[{status}] {body}"));
        }
        Ok(())
    }
    pub async fn delete_body<T: DeserializeOwned>(&self, path: &str, body: &impl serde::Serialize) -> Result<T, String> {
        let json = serde_json::to_string(body).map_err(|e| e.to_string())?;
        Self::handle(self.auth(self.inner.delete(self.url(path))).header(CONTENT_TYPE, "application/json").body(json).send().await.map_err(|e| format!("connection: {e}"))?).await
    }
}
