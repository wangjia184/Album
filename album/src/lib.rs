pub mod api;
pub mod static_files;

use axum::http::{header, HeaderValue, Method};
use axum::Router;
use tower_http::cors::CorsLayer;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

pub fn build_app() -> Router {
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
        .merge(swagger)
        .fallback(static_files::static_handler)
        .layer(cors)
}
