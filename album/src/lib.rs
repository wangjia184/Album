pub mod api;
pub mod fs;
pub mod fs_api;
pub mod mount;
pub mod queue;
pub mod static_files;

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use axum::http::{header, HeaderValue, Method};
use axum::Router;
use tower_http::cors::CorsLayer;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use crate::mount::MountTable;
use crate::queue::ImageQueue;

/// Hostname mounts plus one playback queue per unique mount root.
#[derive(Debug, Clone, Default)]
pub struct AppState {
    pub mounts: MountTable,
    pub queues: Arc<HashMap<PathBuf, Arc<ImageQueue>>>,
}

impl AppState {
    /// State with no mounts (health/CORS/swagger/static tests).
    pub fn empty() -> Self {
        Self::default()
    }

    /// State for a full server: spawns one playback queue per unique mount root.
    pub fn new(mounts: MountTable) -> Self {
        let mut queues: HashMap<PathBuf, Arc<ImageQueue>> = HashMap::new();
        for root in mounts.roots() {
            queues
                .entry(root.clone())
                .or_insert_with(|| ImageQueue::start(root.clone()));
        }
        Self {
            mounts,
            queues: Arc::new(queues),
        }
    }
}

pub fn build_app(state: AppState) -> Router {
    let cors = CorsLayer::new()
        .allow_origin([
            HeaderValue::from_static("http://localhost:5173"),
            HeaderValue::from_static("http://127.0.0.1:5173"),
        ])
        .allow_methods([Method::GET, Method::HEAD, Method::OPTIONS])
        .allow_headers([header::CONTENT_TYPE, header::ACCEPT]);

    let swagger =
        SwaggerUi::new("/swagger-ui").url("/api-doc/openapi.json", api::ApiDoc::openapi());

    api::router()
        .with_state(state)
        .merge(swagger)
        .fallback(static_files::static_handler)
        .layer(cors)
}
