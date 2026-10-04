use album::{build_app, AppState};
use axum::body::{to_bytes, Body};
use axum::http::{header, Request, StatusCode};
use tower::ServiceExt;

async fn body_json(response: axum::response::Response) -> serde_json::Value {
    let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    serde_json::from_slice(&bytes).unwrap()
}

async fn get_site(state: AppState) -> serde_json::Value {
    let app = build_app(state);
    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/site")
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
    body_json(response).await
}

#[tokio::test]
async fn site_defaults_to_empty_strings() {
    let json = get_site(AppState::empty()).await;
    assert_eq!(json["siteName"], "");
    assert_eq!(json["siteNote"], "");
}

#[tokio::test]
async fn site_returns_configured_values() {
    let state = AppState {
        site_name: "Album".to_string(),
        site_note: "湘ICP备17022195号".to_string(),
        ..AppState::empty()
    };
    let json = get_site(state).await;
    assert_eq!(json["siteName"], "Album");
    assert_eq!(json["siteNote"], "湘ICP备17022195号");
}