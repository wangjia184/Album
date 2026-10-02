use axum::extract::Json;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::get;
use axum::Router;
use serde::Serialize;
use serde_json::json;
use utoipa::{OpenApi, ToSchema};

#[derive(OpenApi)]
#[openapi(
    info(title = "Album API", version = "0.1.0"),
    paths(health),
    components(schemas(HealthStatus, ApiError)),
    tags((name = "health", description = "Liveness probe"))
)]
pub struct ApiDoc;

#[derive(Debug, Serialize, ToSchema)]
pub struct HealthStatus {
    /// Service status; "ok" when healthy.
    pub status: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ApiError {
    pub error: String,
}

/// Health check.
#[utoipa::path(
    get,
    path = "/api/health",
    tag = "health",
    responses(
        (status = 200, description = "Service is healthy", body = HealthStatus),
        (status = 404, description = "Not found", body = ApiError)
    )
)]
async fn health() -> Json<HealthStatus> {
    tracing::debug!("health check");
    Json(HealthStatus {
        status: "ok".to_string(),
    })
}

async fn api_not_found() -> impl IntoResponse {
    (StatusCode::NOT_FOUND, Json(json!({ "error": "not_found" })))
}

pub fn router() -> Router {
    let api_routes = Router::new()
        .route("/health", get(health))
        .fallback(api_not_found);

    Router::new().nest("/api", api_routes)
}
