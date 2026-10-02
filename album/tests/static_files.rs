use album::{build_app, AppState};
use axum::body::{to_bytes, Body};
use axum::http::{header, Request, StatusCode};
use tower::ServiceExt;

async fn get(uri: &str) -> (StatusCode, Option<String>, String) {
    let app = build_app(AppState::empty());
    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(uri)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let status = response.status();
    let content_type = response
        .headers()
        .get(header::CONTENT_TYPE)
        .map(|v| v.to_str().unwrap().to_string());
    let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    (
        status,
        content_type,
        String::from_utf8_lossy(&bytes).into_owned(),
    )
}

#[tokio::test]
async fn root_serves_index_html() {
    let (status, content_type, body) = get("/").await;

    assert_eq!(status, StatusCode::OK);
    let content_type = content_type.expect("missing content-type header");
    assert!(
        content_type.contains("text/html"),
        "expected text/html, got {content_type}"
    );
    assert!(!body.is_empty(), "expected non-empty HTML body");
    let lower = body.to_ascii_lowercase();
    assert!(
        body.contains("Album scaffold") || lower.contains("<div id=\"app\""),
        "body does not look like the app index.html: {body}"
    );
}

#[tokio::test]
async fn unknown_path_falls_back_to_spa_shell() {
    let (status, content_type, body) = get("/does-not-exist").await;

    assert_eq!(status, StatusCode::OK);
    let content_type = content_type.expect("missing content-type header");
    assert!(
        content_type.contains("text/html"),
        "expected text/html, got {content_type}"
    );
    assert!(!body.is_empty(), "expected non-empty SPA shell body");
}

#[tokio::test]
async fn path_traversal_returns_404_without_leaking_cargo_toml() {
    for uri in [
        "/../Cargo.toml",
        "/%2e%2e/Cargo.toml",
        "/assets/../../Cargo.toml",
    ] {
        let (status, _, body) = get(uri).await;
        assert_eq!(
            status,
            StatusCode::NOT_FOUND,
            "expected 404 for {uri}, got {status}"
        );
        assert!(
            !body.contains("[package]") && !body.contains("name = \"album\""),
            "response for {uri} leaked Cargo.toml contents: {body}"
        );
    }
}

#[tokio::test]
async fn known_asset_served_with_non_html_content_type() {
    let (status, content_type, body) = get("/favicon.svg").await;

    assert_eq!(status, StatusCode::OK);
    let content_type = content_type.expect("missing content-type header");
    assert!(
        content_type.contains("image/svg"),
        "expected image/svg content type, got {content_type}"
    );
    assert!(!body.is_empty(), "expected non-empty favicon body");
    assert!(body.contains("<svg"), "favicon body is not SVG");
}
