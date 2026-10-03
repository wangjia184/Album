use album::mount::MountTable;
use album::{build_app, AppState};
use axum::body::{to_bytes, Body};
use axum::http::{header, Request, StatusCode};
use axum::Router;
use tower::ServiceExt;

fn fixture() -> tempfile::TempDir {
    let tmp = tempfile::tempdir().expect("tempdir");
    let root = tmp.path();
    std::fs::create_dir(root.join("b_dir")).expect("b_dir");
    std::fs::create_dir(root.join("a_dir")).expect("a_dir");
    std::fs::write(root.join("a_dir").join("nested.txt"), "nested contents").expect("nested.txt");
    std::fs::write(root.join("z_file.txt"), "z file contents").expect("z_file.txt");
    std::fs::write(root.join("a_file.jpg"), b"jpg").expect("a_file.jpg");
    std::fs::write(root.join("m_clip.mp4"), b"mp4").expect("m_clip.mp4");
    std::fs::write(root.join("other.bin"), b"bin").expect("other.bin");
    tmp
}

fn app_with(tmp: &tempfile::TempDir) -> Router {
    let mounts = MountTable::from_iter([
        ("localhost".to_string(), tmp.path().to_path_buf()),
        ("*".to_string(), tmp.path().to_path_buf()),
    ]);
    build_app(AppState {
        mounts,
        ..Default::default()
    })
}

async fn get(app: &Router, uri: &str) -> axum::response::Response {
    get_host(app, uri, "localhost:3000").await
}

async fn get_host(app: &Router, uri: &str, host: &str) -> axum::response::Response {
    app.clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(uri)
                .header(header::HOST, host)
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
async fn list_root_ok_and_sorted() {
    let tmp = fixture();
    let app = app_with(&tmp);

    let response = get(&app, "/api/fs/list").await;
    assert_eq!(response.status(), StatusCode::OK);

    let json = body_json(response).await;
    assert_eq!(json["path"], "");
    let expected_root = tmp
        .path()
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    assert_eq!(json["rootName"], expected_root);
    let children = json["children"].as_array().expect("children array");
    let names: Vec<&str> = children
        .iter()
        .map(|c| c["name"].as_str().expect("name"))
        .collect();
    assert_eq!(
        names,
        [
            "a_dir",
            "b_dir",
            "a_file.jpg",
            "m_clip.mp4",
            "other.bin",
            "z_file.txt"
        ]
    );
    let kinds: Vec<&str> = children
        .iter()
        .map(|c| c["kind"].as_str().expect("kind"))
        .collect();
    assert_eq!(kinds, ["dir", "dir", "image", "video", "other", "other"]);
}

#[tokio::test]
async fn list_nested_path_echoes_normalized_path() {
    let tmp = fixture();
    let app = app_with(&tmp);

    let response = get(&app, "/api/fs/list/a_dir").await;
    assert_eq!(response.status(), StatusCode::OK);

    let json = body_json(response).await;
    assert_eq!(json["path"], "a_dir");
    let expected_root = tmp
        .path()
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    assert_eq!(json["rootName"], expected_root);
    let children = json["children"].as_array().expect("children array");
    assert!(children
        .iter()
        .any(|c| c["name"] == "nested.txt" && c["kind"] == "other"));
}

#[tokio::test]
async fn list_dotdot_returns_404_json() {
    let tmp = fixture();
    let app = app_with(&tmp);

    let response = get(&app, "/api/fs/list/../etc").await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);

    let json = body_json(response).await;
    assert_eq!(json["error"], "not_found");
}

#[tokio::test]
async fn host_header_strips_port_and_maps_mount() {
    let tmp = fixture();
    let mounts = MountTable::from_iter([("example.test".to_string(), tmp.path().to_path_buf())]);
    let app = build_app(AppState {
        mounts,
        ..Default::default()
    });

    let response = get_host(&app, "/api/fs/list", "example.test:3000").await;
    assert_eq!(response.status(), StatusCode::OK);

    let json = body_json(response).await;
    assert_eq!(json["path"], "");
    assert!(json["children"].as_array().is_some_and(|c| !c.is_empty()));
}

#[tokio::test]
async fn unknown_host_falls_back_to_star_or_404() {
    let tmp = fixture();

    // mounts only ("*", tmp): Host: nope.example → 200
    let star_only = MountTable::from_iter([("*".to_string(), tmp.path().to_path_buf())]);
    let app = build_app(AppState {
        mounts: star_only,
        ..Default::default()
    });
    let response = get_host(&app, "/api/fs/list", "nope.example").await;
    assert_eq!(response.status(), StatusCode::OK);
    let json = body_json(response).await;
    assert_eq!(json["path"], "");

    // mounts empty: Host: nope → 404
    let empty = MountTable::from_iter([]);
    let app = build_app(AppState {
        mounts: empty,
        ..Default::default()
    });
    let response = get_host(&app, "/api/fs/list", "nope").await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    let json = body_json(response).await;
    assert_eq!(json["error"], "not_found");
}

#[tokio::test]
async fn legacy_host_path_segment_404() {
    let tmp = fixture();
    let app = app_with(&tmp);

    let response = get(&app, "/api/fs/t/list").await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);

    let json = body_json(response).await;
    assert_eq!(json["error"], "not_found");
}

#[tokio::test]
async fn file_returns_raw_bytes_and_content_type() {
    let tmp = fixture();
    let app = app_with(&tmp);

    let response = get(&app, "/api/fs/file/z_file.txt").await;
    assert_eq!(response.status(), StatusCode::OK);

    let content_type = response
        .headers()
        .get(header::CONTENT_TYPE)
        .expect("content-type header")
        .to_str()
        .unwrap()
        .to_string();
    assert!(
        content_type.contains("text/plain") || content_type.contains("application/octet-stream"),
        "expected text/plain or octet-stream, got {content_type}"
    );

    let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    assert_eq!(bytes.as_ref(), b"z file contents");
}

#[tokio::test]
async fn file_etag_then_304() {
    let tmp = fixture();
    let app = app_with(&tmp);

    let response = get(&app, "/api/fs/file/z_file.txt").await;
    assert_eq!(response.status(), StatusCode::OK);
    let etag = response
        .headers()
        .get(header::ETAG)
        .expect("etag header")
        .to_str()
        .unwrap()
        .to_string();
    assert!(
        etag.starts_with("W/\"") && etag.ends_with('"'),
        "expected weak ETag W/\"...\", got {etag}"
    );
    let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    assert!(!bytes.is_empty(), "expected non-empty file body");

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/fs/file/z_file.txt")
                .header(header::HOST, "localhost:3000")
                .header(header::IF_NONE_MATCH, &etag)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NOT_MODIFIED);
    let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    assert!(bytes.is_empty(), "304 must have an empty body");
}

#[cfg(unix)]
#[tokio::test]
async fn file_symlink_escape_404() {
    let tmp = fixture();
    std::os::unix::fs::symlink("/", tmp.path().join("escape_link")).expect("symlink");
    let app = app_with(&tmp);

    let response = get(&app, "/api/fs/file/escape_link/etc/passwd").await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);

    let json = body_json(response).await;
    assert_eq!(json["error"], "not_found");
}

#[tokio::test]
async fn meta_image_returns_dimensions_and_basic() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let png = image::RgbImage::from_pixel(4, 2, image::Rgb([10, 20, 30]));
    png.save(tmp.path().join("pic.png")).expect("save png");
    let app = app_with(&tmp);

    let response = get(&app, "/api/fs/meta/pic.png").await;
    assert_eq!(response.status(), StatusCode::OK);

    let json = body_json(response).await;
    assert_eq!(json["name"], "pic.png");
    assert_eq!(json["format"], "image/png");
    assert_eq!(json["width"], 4);
    assert_eq!(json["height"], 2);
    assert!(json["size"].as_u64().unwrap_or(0) > 0);
    assert!(json["modified"].is_number());
    assert!(json["exif"].is_null() || json["exif"].is_object());
}

#[tokio::test]
async fn meta_non_image_returns_basic_without_dimensions() {
    let tmp = tempfile::tempdir().expect("tempdir");
    std::fs::write(tmp.path().join("notes.txt"), "hello").expect("write");
    let app = app_with(&tmp);

    let response = get(&app, "/api/fs/meta/notes.txt").await;
    assert_eq!(response.status(), StatusCode::OK);

    let json = body_json(response).await;
    assert_eq!(json["name"], "notes.txt");
    assert_eq!(json["format"], "text/plain");
    assert_eq!(json["size"], 5);
    assert!(json["width"].is_null());
    assert!(json["height"].is_null());
    assert!(json["exif"].is_null());
}

#[tokio::test]
async fn meta_missing_404() {
    let tmp = fixture();
    let app = app_with(&tmp);

    let response = get(&app, "/api/fs/meta/nope.jpg").await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    let json = body_json(response).await;
    assert_eq!(json["error"], "not_found");
}

#[cfg(unix)]
#[tokio::test]
async fn meta_symlink_escape_404() {
    let tmp = fixture();
    std::os::unix::fs::symlink("/", tmp.path().join("escape_link")).expect("symlink");
    let app = app_with(&tmp);

    let response = get(&app, "/api/fs/meta/escape_link/etc/passwd").await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    let json = body_json(response).await;
    assert_eq!(json["error"], "not_found");
}

#[tokio::test]
async fn health_still_works_with_fs_routes() {
    let tmp = fixture();
    let app = app_with(&tmp);

    let response = get(&app, "/api/health").await;
    assert_eq!(response.status(), StatusCode::OK);
    let json = body_json(response).await;
    assert_eq!(json["status"], "ok");

    let response = get(&app, "/api/nope").await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    let json = body_json(response).await;
    assert_eq!(json["error"], "not_found");
}

/// Tempdir with `pics/` holding 10 jpgs + 1 mp4 (mp4 must never be picked).
fn pics_fixture() -> tempfile::TempDir {
    let tmp = tempfile::tempdir().expect("tempdir");
    let pics = tmp.path().join("pics");
    std::fs::create_dir(&pics).expect("pics");
    for i in 0..10 {
        std::fs::write(pics.join(format!("img_{i}.jpg")), b"jpg").expect("write img");
    }
    std::fs::write(pics.join("clip.mp4"), b"mp4").expect("write mp4");
    tmp
}

#[tokio::test]
async fn thumbs_returns_three_unique_direct_images() {
    let tmp = pics_fixture();
    let app = app_with(&tmp);

    let response = get(&app, "/api/fs/thumbs/pics?n=3").await;
    assert_eq!(response.status(), StatusCode::OK);

    let json = body_json(response).await;
    let images = json["images"].as_array().expect("images array");
    assert_eq!(images.len(), 3, "default-ish n=3 must pick 3");
    let mut names: Vec<String> = images
        .iter()
        .map(|v| v.as_str().expect("string image name").to_string())
        .collect();
    for name in &names {
        assert!(name.ends_with(".jpg"), "must only pick images, got {name}");
    }
    names.sort();
    names.dedup();
    assert_eq!(names.len(), 3, "picks must be unique");
}

#[tokio::test]
async fn thumbs_no_direct_images_returns_empty_array() {
    let tmp = fixture(); // a_dir contains only nested.txt
    let app = app_with(&tmp);

    let response = get(&app, "/api/fs/thumbs/a_dir?n=3").await;
    assert_eq!(response.status(), StatusCode::OK);

    let json = body_json(response).await;
    assert!(json["images"].is_array());
    assert_eq!(json["images"].as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn thumbs_missing_path_404_json() {
    let tmp = fixture();
    let app = app_with(&tmp);

    let response = get(&app, "/api/fs/thumbs/nope").await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);

    let json = body_json(response).await;
    assert_eq!(json["error"], "not_found");
}

#[tokio::test]
async fn thumbs_default_n_is_three() {
    let tmp = pics_fixture();
    let app = app_with(&tmp);

    let response = get(&app, "/api/fs/thumbs/pics").await;
    assert_eq!(response.status(), StatusCode::OK);

    let json = body_json(response).await;
    assert_eq!(json["images"].as_array().expect("images").len(), 3);
}

#[tokio::test]
async fn thumbs_n_clamped_to_eight() {
    let tmp = pics_fixture(); // 10 images
    let app = app_with(&tmp);

    let response = get(&app, "/api/fs/thumbs/pics?n=100").await;
    assert_eq!(response.status(), StatusCode::OK);

    let json = body_json(response).await;
    assert_eq!(json["images"].as_array().expect("images").len(), 8);
}

#[tokio::test]
async fn thumbs_recurses_when_no_direct_images() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let album_dir = tmp.path().join("album");
    std::fs::create_dir_all(album_dir.join("sub")).expect("sub");
    std::fs::write(album_dir.join("sub").join("nested.jpg"), b"jpg").expect("write");
    let app = app_with(&tmp);

    let response = get(&app, "/api/fs/thumbs/album?n=3").await;
    assert_eq!(response.status(), StatusCode::OK);

    let json = body_json(response).await;
    let images = json["images"].as_array().expect("images array");
    assert_eq!(images.len(), 1);
    assert_eq!(images[0], "sub/nested.jpg");
}
