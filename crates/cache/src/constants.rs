/// Cache TTL constants in seconds
pub mod cache_ttl {
    /// Action plan TTL: 5 minutes (300 seconds)
    pub const ACTION_PLAN_TTL: u64 = 300;

    /// Scene data TTL: 24 hours (86400 seconds)
    pub const SCENE_DATA_TTL: u64 = 86400;

    /// Navigation config TTL: 1 hour (3600 seconds)
    pub const NAV_CONFIG_TTL: u64 = 3600;

    /// Action system config TTL: 1 hour (3600 seconds)
    pub const ACTION_CONFIG_TTL: u64 = 3600;

    /// AI context TTL: 1 hour (3600 seconds)
    pub const AI_CONTEXT_TTL: u64 = 3600;

    /// Action history TTL: 7 days (604800 seconds)
    pub const ACTION_HISTORY_TTL: u64 = 604800;
}
