use axum::http::{header, StatusCode, Uri};
use axum::response::{IntoResponse, Response};
use percent_encoding::percent_decode_str;
use rust_embed::RustEmbed;

#[derive(RustEmbed)]
#[folder = "../ui/dist"]
struct Assets;

fn decode_path(raw: &str) -> String {
    percent_decode_str(raw).decode_utf8_lossy().into_owned()
}

fn has_parent_segment(path: &str) -> bool {
    path.split('/').any(|segment| segment == "..")
}

fn index_response() -> Response {
    match Assets::get("index.html") {
        Some(index) => (
            [(header::CONTENT_TYPE, "text/html; charset=utf-8")],
            index.data.into_owned(),
        )
            .into_response(),
        None => {
            eprintln!(
                "static_files: embedded ui/dist/index.html is missing — \
                 run `npm run build` in ui/ and rebuild the album crate"
            );
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "ui/dist/index.html is missing; build the ui first",
            )
                .into_response()
        }
    }
}

pub async fn static_handler(uri: Uri) -> Response {
    let path = decode_path(uri.path());

    if has_parent_segment(&path) {
        return (StatusCode::NOT_FOUND, "Not Found").into_response();
    }

    let key = path.trim_start_matches('/');
    let lookup = if key.is_empty() { "index.html" } else { key };

    match Assets::get(lookup) {
        Some(file) => {
            let content_type = file.metadata.mimetype();
            (
                [(header::CONTENT_TYPE, content_type)],
                file.data.into_owned(),
            )
                .into_response()
        }
        None => index_response(),
    }
}
