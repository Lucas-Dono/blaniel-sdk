/// Cache TTL strategies for different data types
pub struct CacheStrategy;

impl CacheStrategy {
    /// NPC state cache TTL (1 minute - frequently updated)
    pub const NPC_STATE_TTL: u64 = 60;
    pub const NPC_POSITION_TTL: u64 = 30;
    pub const NPC_EMOTIONS_TTL: u64 = 300;
    pub const PATHFINDING_TTL: u64 = 30;
    pub const SIMPLE_CHAT_TTL: u64 = 300;
    pub const AI_CHAT_TTL: u64 = 30;
    pub const AMBIENT_DIALOGUE_TTL: u64 = 300;
    pub const PROXIMITY_TTL: u64 = 10;
    pub const METADATA_TTL: u64 = 600;

    pub fn ttl_for_key(key: &str) -> u64 {
        if key.starts_with("npc:state:") {
            Self::NPC_STATE_TTL
        } else if key.starts_with("npc:position:") {
            Self::NPC_POSITION_TTL
        } else if key.starts_with("npc:emotions:") {
            Self::NPC_EMOTIONS_TTL
        } else if key.starts_with("path:") {
            Self::PATHFINDING_TTL
        } else if key.starts_with("chat:") {
            Self::SIMPLE_CHAT_TTL
        } else if key.starts_with("dialogue:ambient:") {
            Self::AMBIENT_DIALOGUE_TTL
        } else if key.starts_with("proximity:") {
            Self::PROXIMITY_TTL
        } else if key.starts_with("npc:meta:") {
            Self::METADATA_TTL
        } else {
            // Default TTL
            60
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ttl_for_key() {
        assert_eq!(
            CacheStrategy::ttl_for_key("npc:state:agent123"),
            CacheStrategy::NPC_STATE_TTL
        );
        assert_eq!(
            CacheStrategy::ttl_for_key("npc:position:agent123"),
            CacheStrategy::NPC_POSITION_TTL
        );
        assert_eq!(
            CacheStrategy::ttl_for_key("chat:agent123:hello"),
            CacheStrategy::SIMPLE_CHAT_TTL
        );
        assert_eq!(
            CacheStrategy::ttl_for_key("path:0:0:0:10:10:10"),
            CacheStrategy::PATHFINDING_TTL
        );
        assert_eq!(CacheStrategy::ttl_for_key("unknown:key"), 60);
    }
}
