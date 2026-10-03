use std::collections::HashMap;
use std::sync::Arc;

use album::mount::MountTable;
use album::queue::ImageQueue;
use album::{build_app, AppState};
use axum::body::{to_bytes, Body};
use axum::http::{header, Request, StatusCode};
use axum::Router;
use tower::ServiceExt;

/// Tempdir mount root + app with a pre-seeded (already-done) queue.
fn app_seeded(paths: &[&str]) -> Router {
    let tmp = tempfile::tempdir().expect("tempdir");
    let root = tmp.path().to_path_buf();
    // Keep the TempDir alive by leaking it: tests are short-lived processes.
    std::mem::forget(tmp);
    let mounts = MountTable::from_iter([
        ("localhost".to_string(), root.clone()),
        ("*".to_string(), root.clone()),
    ]);
    let queue = ImageQueue::from_paths(paths.iter().map(|s| s.to_string()).collect());
    let queues = Arc::new(HashMap::from([(root, queue)]));
    build_app(AppState { mounts, queues })
}

async fn get(app: &Router, uri: &str) -> axum::response::Response {
    app.clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(uri)
                .header(header::HOST, "localhost:3000")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap()
}

async fn body_json(response: axum::response::Response) -> serde_json::Value {
    let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    serde_json::from_slice(&bytes).unwrap()
}

#[tokio::test]
async fn queue_basic_window_and_next_offset() {
    let app = app_seeded(&["a", "b", "c", "d"]);
    let response = get(&app, "/api/fs/queue?offset=1&limit=2").await;
    assert_eq!(response.status(), StatusCode::OK);
    let json = body_json(response).await;
    let images: Vec<&str> = json["images"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap())
        .collect();
    assert_eq!(images, ["b", "c"]);
    assert_eq!(json["nextOffset"], 2);
    assert_eq!(json["done"], true);
}

#[tokio::test]
async fn queue_offset_beyond_len_wraps() {
    let app = app_seeded(&["a", "b", "c", "d"]);
    let response = get(&app, "/api/fs/queue?offset=10&limit=4").await;
    assert_eq!(response.status(), StatusCode::OK);
    let json = body_json(response).await;
    let images: Vec<&str> = json["images"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap())
        .collect();
    assert_eq!(images, ["c", "d", "a", "b"]);
    assert_eq!(json["nextOffset"], 3);
}

#[tokio::test]
async fn queue_window_wraps_at_tail() {
    let app = app_seeded(&["a", "b", "c", "d"]);
    let response = get(&app, "/api/fs/queue?offset=3&limit=3").await;
    assert_eq!(response.status(), StatusCode::OK);
    let json = body_json(response).await;
    let images: Vec<&str> = json["images"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap())
        .collect();
    assert_eq!(images, ["d", "a", "b"]);
    assert_eq!(json["nextOffset"], 0);
}

#[tokio::test]
async fn queue_empty_ok_no_modulo_zero() {
    let app = app_seeded(&[]);
    let response = get(&app, "/api/fs/queue?offset=99&limit=12").await;
    assert_eq!(response.status(), StatusCode::OK);
    let json = body_json(response).await;
    assert_eq!(json["images"].as_array().unwrap().len(), 0);
    assert_eq!(json["nextOffset"], 0);
    assert_eq!(json["done"], true);
}

#[tokio::test]
async fn queue_limit_clamp_and_default() {
    // limit=0 → 1
    let app = app_seeded(&["a", "b", "c"]);
    let json = body_json(get(&app, "/api/fs/queue?offset=0&limit=0").await).await;
    assert_eq!(json["images"].as_array().unwrap().len(), 1);

    // limit=999 → 64 (seed 70 so wraps repeat)
    let many: Vec<String> = (0..70).map(|i| format!("p{i}.jpg")).collect();
    let refs: Vec<&str> = many.iter().map(String::as_str).collect();
    let app = app_seeded(&refs);
    let json = body_json(get(&app, "/api/fs/queue?offset=0&limit=999").await).await;
    assert_eq!(json["images"].as_array().unwrap().len(), 64);

    // no limit → 12 (seed 20)
    let many: Vec<String> = (0..20).map(|i| format!("q{i}.jpg")).collect();
    let refs: Vec<&str> = many.iter().map(String::as_str).collect();
    let app = app_seeded(&refs);
    let json = body_json(get(&app, "/api/fs/queue?offset=0").await).await;
    assert_eq!(json["images"].as_array().unwrap().len(), 12);
}

#[tokio::test]
async fn queue_negative_offset_400() {
    let app = app_seeded(&["a"]);
    let response = get(&app, "/api/fs/queue?offset=-1").await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn queue_appstate_empty_404() {
    let app = build_app(AppState::empty());
    let response = get(&app, "/api/fs/queue?offset=0&limit=1").await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    let json = body_json(response).await;
    assert_eq!(json["error"], "not_found");
}

#[tokio::test]
async fn queue_spawns_at_startup() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let root = tmp.path();
    std::fs::create_dir_all(root.join("nested")).expect("nested");
    std::fs::write(root.join("nested").join("found.jpg"), b"j").expect("found.jpg");

    let mounts = MountTable::from_iter([
        ("localhost".to_string(), root.to_path_buf()),
        ("*".to_string(), root.to_path_buf()),
    ]);
    let app = build_app(AppState::new(mounts));

    let mut found = false;
    let mut done = false;
    for _ in 0..100 {
        let json = body_json(get(&app, "/api/fs/queue?offset=0&limit=64").await).await;
        let images: Vec<&str> = json["images"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap())
            .collect();
        if images.contains(&"nested/found.jpg") {
            found = true;
        }
        done = json["done"].as_bool().unwrap_or(false);
        if found && done {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }
    assert!(found, "startup scan never surfaced nested/found.jpg");
    assert!(done, "queue never reached done");
}
