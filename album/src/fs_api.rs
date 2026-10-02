use std::io;
use std::time::SystemTime;

use axum::body::Body;
use axum::extract::{Path, State};
use axum::http::{header, HeaderMap, HeaderValue, StatusCode};
use axum::response::Response;
use axum::routing::get;
use axum::{Json, Router};
use serde_json::{json, Value};
use tokio_util::io::ReaderStream;

use crate::fs::AlbumFs;
use crate::AppState;

type ApiError = (StatusCode, Json<Value>);

fn not_found() -> ApiError {
    (StatusCode::NOT_FOUND, Json(json!({ "error": "not_found" })))
}

fn internal() -> ApiError {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(json!({ "error": "internal" })),
    )
}

fn map_io(err: io::Error) -> ApiError {
    match err.kind() {
        io::ErrorKind::NotFound => not_found(),
        _ => internal(),
    }
}

/// Routes are relative to the `/api` nest; no fallback here (the nest's
/// inner fallback handles unknown `/api/*` as JSON 404).
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/fs/{host}/list", get(list_root))
        .route("/fs/{host}/list/{*path}", get(list_path))
        .route("/fs/{host}/file/{*path}", get(file))
}

async fn list_impl(state: AppState, host: String, rel: String) -> Result<Json<Value>, ApiError> {
    let root = state.mounts.get(&host).cloned().ok_or_else(not_found)?;
    let path_echo = rel.clone();
    let children = tokio::task::spawn_blocking(move || {
        let album = AlbumFs::new(&root)?;
        album.list_children(&rel)
    })
    .await
    .map_err(|_| internal())?
    .map_err(map_io)?;
    Ok(Json(json!({ "path": path_echo, "children": children })))
}

async fn list_root(
    State(state): State<AppState>,
    Path(host): Path<String>,
) -> Result<Json<Value>, ApiError> {
    list_impl(state, host, String::new()).await
}

async fn list_path(
    State(state): State<AppState>,
    Path((host, path)): Path<(String, String)>,
) -> Result<Json<Value>, ApiError> {
    list_impl(state, host, path).await
}

async fn file(
    State(state): State<AppState>,
    Path((host, path)): Path<(String, String)>,
    headers: HeaderMap,
) -> Result<Response, ApiError> {
    let root = state.mounts.get(&host).cloned().ok_or_else(not_found)?;
    let content_type = HeaderValue::from_str(
        mime_guess::from_path(&path)
            .first_or_octet_stream()
            .as_ref(),
    )
    .map_err(|_| internal())?;

    let (std_file, mtime_ns, size) =
        tokio::task::spawn_blocking(move || -> io::Result<(std::fs::File, u128, u64)> {
            let album = AlbumFs::new(&root)?;
            let file = album.open_file(&path)?;
            let meta = file.metadata()?;
            let mtime_ns = meta
                .modified()
                .ok()
                .and_then(|t| t.duration_since(SystemTime::UNIX_EPOCH).ok())
                .map(|d| d.as_nanos())
                .unwrap_or(0);
            Ok((file, mtime_ns, meta.len()))
        })
        .await
        .map_err(|_| internal())?
        .map_err(map_io)?;

    let etag = format!("W/\"{mtime_ns:x}-{size:x}\"");
    if headers
        .get(header::IF_NONE_MATCH)
        .is_some_and(|v| v.as_bytes() == etag.as_bytes())
    {
        return Response::builder()
            .status(StatusCode::NOT_MODIFIED)
            .header(header::ETAG, &etag)
            .body(Body::empty())
            .map_err(|_| internal());
    }

    let tokio_file = tokio::fs::File::from_std(std_file);
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, content_type)
        .header(header::ETAG, &etag)
        .body(Body::from_stream(ReaderStream::new(tokio_file)))
        .map_err(|_| internal())
}
