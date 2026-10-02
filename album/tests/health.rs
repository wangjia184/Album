use album::{build_app, AppState};
use axum::body::{to_bytes, Body};
use axum::http::{header, Request, StatusCode};
use tower::ServiceExt;

async fn body_json(response: axum::response::Response) -> serde_json::Value {
    let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    serde_json::from_slice(&bytes).unwrap()
}

#[tokio::test]
async fn health_returns_200_with_ok_status() {
    let app = build_app(AppState::empty());

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let content_type = response.headers().get(header::CONTENT_TYPE).unwrap();
    assert!(
        content_type.to_str().unwrap().contains("application/json"),
        "expected application/json, got {content_type:?}"
    );

    let json = body_json(response).await;
    assert_eq!(json["status"], "ok");
}

#[tokio::test]
async fn unknown_api_route_returns_json_404() {
    let app = build_app(AppState::empty());

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/nope")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    let json = body_json(response).await;
    assert_eq!(json["error"], "not_found");
}

#[tokio::test]
async fn cors_allows_vite_dev_origin() {
    let app = build_app(AppState::empty());

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/health")
                .header(header::ORIGIN, "http://localhost:5173")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    let acao = response
        .headers()
        .get(header::ACCESS_CONTROL_ALLOW_ORIGIN)
        .expect("missing access-control-allow-origin header");
    assert_eq!(acao, "http://localhost:5173");
}
