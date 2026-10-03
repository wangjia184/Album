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

fn images_of(json: &serde_json::Value) -> Vec<(u64, String)> {
    json["images"]
        .as_array()
        .expect("images array")
        .iter()
        .map(|item| {
            (
                item["index"].as_u64().expect("index"),
                item["path"].as_str().expect("path").to_string(),
            )
        })
        .collect()
}

#[tokio::test]
async fn queue_forward_window_indices() {
    let app = app_seeded(&["a", "b", "c", "d"]);
    let response = get(&app, "/api/fs/queue?offset=1&limit=2").await;
    assert_eq!(response.status(), StatusCode::OK);
    let json = body_json(response).await;
    let items = images_of(&json);
    assert_eq!(items, vec![(1, "b".to_string()), (2, "c".to_string())]);
    assert_eq!(json["done"], true);
    assert_eq!(json["total"], 4);
    assert!(json.get("nextOffset").is_none(), "nextOffset must be gone");
}

#[tokio::test]
async fn queue_offset_beyond_len_wraps() {
    let app = app_seeded(&["a", "b", "c", "d"]);
    let json = body_json(get(&app, "/api/fs/queue?offset=10&limit=4").await).await;
    let items = images_of(&json);
    assert_eq!(
        items,
        vec![
            (2, "c".to_string()),
            (3, "d".to_string()),
            (0, "a".to_string()),
            (1, "b".to_string())
        ]
    );
}

#[tokio::test]
async fn queue_window_wraps_at_tail() {
    let app = app_seeded(&["a", "b", "c", "d"]);
    let json = body_json(get(&app, "/api/fs/queue?offset=3&limit=3").await).await;
    let items = images_of(&json);
    assert_eq!(
        items,
        vec![
            (3, "d".to_string()),
            (0, "a".to_string()),
            (1, "b".to_string())
        ]
    );
}

#[tokio::test]
async fn queue_empty_ok_no_modulo_zero() {
    let app = app_seeded(&[]);
    let json = body_json(get(&app, "/api/fs/queue?offset=99&limit=12&center=true").await).await;
    assert_eq!(json["images"].as_array().unwrap().len(), 0);
    assert_eq!(json["done"], true);
    assert_eq!(json["total"], 0);
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
async fn queue_center_true_places_offset_at_mid() {
    // 20 paths, offset=5, limit=10 (even → mid=4): items[4].index==5,
    // indices contiguous mod 20 starting at (5-4)=1
    let many: Vec<String> = (0..20).map(|i| format!("p{i}")).collect();
    let refs: Vec<&str> = many.iter().map(String::as_str).collect();
    let app = app_seeded(&refs);
    let json = body_json(get(&app, "/api/fs/queue?offset=5&limit=10&center=true").await).await;
    let items = images_of(&json);
    assert_eq!(items.len(), 10);
    assert_eq!(items[4].0, 5, "offset must land at mid");
    let indices: Vec<u64> = items.iter().map(|(i, _)| *i).collect();
    assert_eq!(indices, (1..=10).collect::<Vec<u64>>());
    // paths match indices
    for (idx, path) in &items {
        assert_eq!(path, &format!("p{idx}"));
    }
}

#[tokio::test]
async fn queue_center_true_offset_below_mid_wraps() {
    // 3 paths, offset=0, limit=5 → mid=2: items[2].index==0, len 5
    let app = app_seeded(&["a", "b", "c"]);
    let json = body_json(get(&app, "/api/fs/queue?offset=0&limit=5&center=true").await).await;
    let items = images_of(&json);
    assert_eq!(items.len(), 5);
    assert_eq!(items[2].0, 0, "offset must land at mid after wrap");
}

#[tokio::test]
async fn queue_center_invalid_value_400() {
    let app = app_seeded(&["a"]);
    let response = get(&app, "/api/fs/queue?offset=0&center=maybe").await;
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
        let items = images_of(&json);
        if items.iter().any(|(_, p)| p == "nested/found.jpg") {
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
