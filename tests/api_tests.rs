use axum::{
    body::Body,
    http::{Request, StatusCode},
    routing::{get, post},
    Router,
};
use tower::ServiceExt;

mod helpers;

#[tokio::test]
async fn test_health_endpoint() {
    let app = Router::new().route("/api/v1/health", get(async || {
        axum::Json(npc_types::HealthResponse {
            status: "healthy".to_string(),
            timestamp: chrono::Utc::now().timestamp(),
            version: "0.1.0".to_string(),
        })
    }));

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}
