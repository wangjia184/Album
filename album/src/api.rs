use axum::extract::{Json, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::get;
use axum::Router;
use serde::Serialize;
use serde_json::json;
use utoipa::{OpenApi, ToSchema};

use crate::AppState;

#[derive(OpenApi)]
#[openapi(
    info(title = "Album API", version = "0.1.0"),
    paths(health, site),
    components(schemas(HealthStatus, SiteInfo, ApiError)),
    tags(
        (name = "health", description = "Liveness probe"),
        (name = "site", description = "Navbar site branding")
    )
)]
pub struct ApiDoc;

#[derive(Debug, Serialize, ToSchema)]
pub struct HealthStatus {
    /// Service status; "ok" when healthy.
    pub status: String,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SiteInfo {
    /// Navbar center title.
    pub site_name: String,
    /// Navbar right-side free-text note.
    pub site_note: String,
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

/// Site branding for the navbar.
#[utoipa::path(
    get,
    path = "/api/site",
    tag = "site",
    responses(
        (status = 200, description = "Site name and right-side note", body = SiteInfo),
        (status = 404, description = "Not found", body = ApiError)
    )
)]
async fn site(State(state): State<AppState>) -> Json<SiteInfo> {
    Json(SiteInfo {
        site_name: state.site_name.clone(),
        site_note: state.site_note.clone(),
    })
}

async fn api_not_found() -> impl IntoResponse {
    (StatusCode::NOT_FOUND, Json(json!({ "error": "not_found" })))
}

pub fn router() -> Router<crate::AppState> {
    let api_routes = Router::new()
        .route("/health", get(health))
        .route("/site", get(site))
        .merge(crate::fs_api::routes())
        .fallback(api_not_found);

    Router::new().nest("/api", api_routes)
}
