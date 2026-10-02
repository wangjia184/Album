use album::{build_app, AppState};
use axum::body::{to_bytes, Body};
use axum::http::{header, Request, StatusCode};
use tower::ServiceExt;

#[tokio::test]
async fn swagger_ui_serves_html() {
    let app = build_app(AppState::empty());

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/swagger-ui/")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let content_type = response.headers().get(header::CONTENT_TYPE).unwrap();
    assert!(
        content_type.to_str().unwrap().contains("text/html"),
        "expected text/html, got {content_type:?}"
    );

    let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let html = String::from_utf8_lossy(&bytes);
    assert!(
        html.contains("swagger") || html.contains("Swagger"),
        "expected swagger UI markup"
    );
}

#[tokio::test]
async fn openapi_spec_documents_health() {
    let app = build_app(AppState::empty());

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api-doc/openapi.json")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let body = String::from_utf8_lossy(&bytes);
    assert!(
        body.contains("/api/health"),
        "spec missing /api/health: {body}"
    );
    assert!(
        body.contains("HealthStatus") || body.contains("status"),
        "spec missing health schema"
    );
}
