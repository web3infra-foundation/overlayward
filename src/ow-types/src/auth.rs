use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AccessLevel {
    Agent = 0,
    User = 1,
    Admin = 2,
    Human = 3,
}

impl AccessLevel {
    #[inline(always)]
    pub fn has_permission(self, required: Self) -> bool {
        match required {
            Self::Agent => true,
            Self::User => !matches!(self, Self::Agent),
            Self::Admin => matches!(self, Self::Admin | Self::Human),
            Self::Human => matches!(self, Self::Human),
        }
    }

    #[inline(always)]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Agent => "agent",
            Self::User => "user",
            Self::Admin => "admin",
            Self::Human => "human",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CallerType {
    Agent,
    User,
    Framework,
    Orchestrator,
    Admin,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallerIdentity {
    pub id: Box<str>,
    pub caller_type: CallerType,
    pub access_level: AccessLevel,
}

impl CallerIdentity {
    #[inline]
    pub fn new(id: impl Into<Box<str>>, caller_type: CallerType, access_level: AccessLevel) -> Self {
        Self { id: id.into(), caller_type, access_level }
    }

    #[inline(always)]
    pub fn can(&self, required: AccessLevel) -> bool {
        self.access_level.has_permission(required)
    }
}
