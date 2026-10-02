use std::io;
use std::path::PathBuf;
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

/// Mount key from the `Host` header: strip `:port`, lowercase.
/// Empty string when the header is missing/unusable.
fn host_key(headers: &HeaderMap) -> String {
    headers
        .get(header::HOST)
        .and_then(|value| value.to_str().ok())
        .map(|host| {
            let host = host.split_once(':').map_or(host, |(h, _)| h);
            host.to_ascii_lowercase()
        })
        .unwrap_or_default()
}

/// Resolve the mount root for this request's `Host` (exact match, then `*`).
fn resolve_root(state: &AppState, headers: &HeaderMap) -> Result<PathBuf, ApiError> {
    let key = host_key(headers);
    if key.is_empty() {
        return Err(not_found());
    }
    state.mounts.get(&key).cloned().ok_or_else(not_found)
}

/// Routes are relative to the `/api` nest; no fallback here (the nest's
/// inner fallback handles unknown `/api/*` as JSON 404).
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/fs/list", get(list_root))
        .route("/fs/list/{*path}", get(list_path))
        .route("/fs/file/{*path}", get(file))
}

async fn list_impl(
    state: AppState,
    headers: HeaderMap,
    rel: String,
) -> Result<Json<Value>, ApiError> {
    let root = resolve_root(&state, &headers)?;
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
    headers: HeaderMap,
) -> Result<Json<Value>, ApiError> {
    list_impl(state, headers, String::new()).await
}

async fn list_path(
    State(state): State<AppState>,
    Path(path): Path<String>,
    headers: HeaderMap,
) -> Result<Json<Value>, ApiError> {
    list_impl(state, headers, path).await
}

async fn file(
    State(state): State<AppState>,
    Path(path): Path<String>,
    headers: HeaderMap,
) -> Result<Response, ApiError> {
    let root = resolve_root(&state, &headers)?;
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
