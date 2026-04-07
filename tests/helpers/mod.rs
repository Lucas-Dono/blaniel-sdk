pub mod helpers {
    use std::sync::Arc;

    pub fn create_test_app() -> axum::Router {
        axum::Router::new()
            .route("/api/v1/health", axum::routing::get(|| async {
                axum::Json(npc_types::HealthResponse {
                    status: "healthy".to_string(),
                    timestamp: chrono::Utc::now().timestamp(),
                    version: "0.1.0".to_string(),
                })
            }))
    }
}
