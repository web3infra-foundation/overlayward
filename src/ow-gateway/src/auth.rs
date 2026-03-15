use ow_types::{AccessLevel, CallerIdentity, CallerType};

pub trait TokenResolver: Send + Sync + 'static {
    fn resolve(&self, token: &str) -> Option<CallerIdentity>;
}

pub struct MockTokenResolver;

static TOKEN_MAP: phf::Map<&'static str, (AccessLevel, CallerType, &'static str)> = phf::phf_map! {
    "ow-agent-token" => (AccessLevel::Agent, CallerType::Agent, "agent-default"),
    "ow-user-token"  => (AccessLevel::User, CallerType::User, "user-default"),
    "ow-admin-token" => (AccessLevel::Admin, CallerType::Admin, "admin-default"),
    "ow-human-token" => (AccessLevel::Human, CallerType::User, "human-default"),
};

impl TokenResolver for MockTokenResolver {
    #[inline]
    fn resolve(&self, token: &str) -> Option<CallerIdentity> {
        TOKEN_MAP.get(token).map(|&(level, ctype, id)| CallerIdentity::new(id, ctype, level))
    }
}
