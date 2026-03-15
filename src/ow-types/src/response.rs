use serde::{Deserialize, Serialize};
use crate::ApiError;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ApiResponse<T: Serialize> {
    Ok { ok: OkTrue, data: T },
    Err { ok: OkFalse, error: ApiError },
}

#[derive(Debug, Clone, Copy)]
pub struct OkTrue;
#[derive(Debug, Clone, Copy)]
pub struct OkFalse;

impl Serialize for OkTrue {
    #[inline(always)]
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_bool(true)
    }
}

impl Serialize for OkFalse {
    #[inline(always)]
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_bool(false)
    }
}

impl<'de> Deserialize<'de> for OkTrue {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let v = bool::deserialize(d)?;
        if v { Ok(Self) } else { Err(serde::de::Error::custom("expected true")) }
    }
}

impl<'de> Deserialize<'de> for OkFalse {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let v = bool::deserialize(d)?;
        if !v { Ok(Self) } else { Err(serde::de::Error::custom("expected false")) }
    }
}

impl<T: Serialize> ApiResponse<T> {
    #[inline]
    pub fn ok(data: T) -> Self {
        Self::Ok { ok: OkTrue, data }
    }

    #[inline]
    pub fn err(error: ApiError) -> ApiResponse<()> {
        ApiResponse::Err { ok: OkFalse, error }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginatedData<T> {
    pub items: Vec<T>,
    pub total: u64,
    pub limit: u32,
    pub offset: u32,
    pub has_more: bool,
}

impl<T> PaginatedData<T> {
    #[inline]
    pub fn new(items: Vec<T>, total: u64, limit: u32, offset: u32) -> Self {
        let has_more = (offset as u64 + items.len() as u64) < total;
        Self { items, total, limit, offset, has_more }
    }
}
