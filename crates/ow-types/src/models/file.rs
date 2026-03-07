use bytes::Bytes;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileReadRequest {
    pub sandbox_id: String,
    pub path: String,
    #[serde(default)]
    pub offset: Option<u64>,
    #[serde(default)]
    pub limit: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileContent {
    #[serde(with = "base64_bytes")]
    pub content: Bytes,
    pub size: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mode: Option<Box<str>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileWriteRequest {
    #[serde(default)]
    pub sandbox_id: String,
    pub path: String,
    pub content: String,
    #[serde(default)]
    pub mode: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileListRequest {
    pub sandbox_id: String,
    pub path: String,
    #[serde(default)]
    pub recursive: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileEntry {
    pub name: Box<str>,
    pub path: Box<str>,
    pub is_dir: bool,
    pub size: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mode: Option<Box<str>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub modified_at: Option<Box<str>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileUploadRequest {
    pub sandbox_id: String,
    pub src: String,
    pub dest: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileDownloadRequest {
    pub sandbox_id: String,
    pub path: String,
}

mod base64_bytes {
    use bytes::Bytes;
    use serde::{Deserialize, Deserializer, Serializer};

    #[inline]
    pub fn serialize<S: Serializer>(data: &Bytes, s: S) -> Result<S::Ok, S::Error> {
        use serde::Serialize;
        let encoded = base64_encode(data);
        encoded.serialize(s)
    }

    #[inline]
    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Bytes, D::Error> {
        let s = String::deserialize(d)?;
        base64_decode(&s).map_err(serde::de::Error::custom)
    }

    #[inline]
    fn base64_encode(data: &[u8]) -> String {
        const CHARS: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
        let mut out = String::with_capacity((data.len() + 2) / 3 * 4);
        for chunk in data.chunks(3) {
            let (b0, b1, b2) = (
                chunk[0] as u32,
                chunk.get(1).copied().unwrap_or(0) as u32,
                chunk.get(2).copied().unwrap_or(0) as u32,
            );
            let triple = (b0 << 16) | (b1 << 8) | b2;
            out.push(CHARS[((triple >> 18) & 0x3F) as usize] as char);
            out.push(CHARS[((triple >> 12) & 0x3F) as usize] as char);
            if chunk.len() > 1 { out.push(CHARS[((triple >> 6) & 0x3F) as usize] as char); } else { out.push('='); }
            if chunk.len() > 2 { out.push(CHARS[(triple & 0x3F) as usize] as char); } else { out.push('='); }
        }
        out
    }

    #[inline]
    fn base64_decode(s: &str) -> Result<Bytes, &'static str> {
        const LUT: [u8; 128] = {
            let mut t = [255u8; 128];
            let mut i = 0u8;
            while i < 26 { t[(b'A' + i) as usize] = i; i += 1; }
            i = 0;
            while i < 26 { t[(b'a' + i) as usize] = 26 + i; i += 1; }
            i = 0;
            while i < 10 { t[(b'0' + i) as usize] = 52 + i; i += 1; }
            t[b'+' as usize] = 62;
            t[b'/' as usize] = 63;
            t
        };
        let s = s.trim_end_matches('=');
        let mut out = Vec::with_capacity(s.len() * 3 / 4);
        let bytes = s.as_bytes();
        let mut i = 0;
        while i + 3 < bytes.len() {
            let (a, b, c, d) = (
                LUT[bytes[i] as usize] as u32, LUT[bytes[i+1] as usize] as u32,
                LUT[bytes[i+2] as usize] as u32, LUT[bytes[i+3] as usize] as u32,
            );
            let triple = (a << 18) | (b << 12) | (c << 6) | d;
            out.push((triple >> 16) as u8);
            out.push((triple >> 8) as u8);
            out.push(triple as u8);
            i += 4;
        }
        let rem = bytes.len() - i;
        if rem >= 2 {
            let a = LUT[bytes[i] as usize] as u32;
            let b = LUT[bytes[i+1] as usize] as u32;
            out.push(((a << 2) | (b >> 4)) as u8);
            if rem >= 3 {
                let c = LUT[bytes[i+2] as usize] as u32;
                out.push((((b & 0xF) << 4) | (c >> 2)) as u8);
            }
        }
        Ok(Bytes::from(out))
    }
}
