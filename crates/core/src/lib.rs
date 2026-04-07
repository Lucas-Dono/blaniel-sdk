pub mod ai_client;
pub mod dialogue;
pub mod hybrid_response;
pub mod llm;
pub mod movement;
pub mod navigation_engine;
pub mod orchestration;
pub mod predictive_cache;
pub mod priority_queue;

pub use ai_client::NextJsClient;
pub use dialogue::AmbientDialogueManager;
pub use hybrid_response::{
    HybridResponse, HybridResponseManager, SmartChatHandler, UrgencyDetector, ResponseQuality,
};
pub use llm::{
    AgentMetricsSummary, GlobalMetricsSummary, LLMProviderRegistry, RequestMetricsCollector,
    RequestRecord, UniversalLLMClient, UserMetricsSummary,
};
pub use movement::{NavigationMesh, PathfindingEngine};
pub use navigation_engine::{
    MovementCommand, MovementCommandParser, MovementEngine, SceneRegistry,
};
pub use orchestration::RequestRouter;
pub use predictive_cache::{PredictiveCacheWarmer, ResponseTemplates, CacheWarmingTask};
pub use priority_queue::{NPCPriorityQueue, NPCRequest, Priority, QueueStats, QueueCleanupTask};
