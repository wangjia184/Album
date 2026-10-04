use std::net::SocketAddr;
use std::path::PathBuf;

use album::mount::{parse_mount, MountTable};
use clap::Parser;

/// Album browser backend — axum API + embedded SPA.
#[derive(Parser, Debug)]
#[command(name = "album", version, about)]
struct Args {
    /// TCP port to listen on.
    #[arg(long, default_value_t = 3000)]
    port: u16,

    /// IP address to bind.
    #[arg(long, default_value = "0.0.0.0")]
    addr: std::net::IpAddr,

    /// Host-to-path mount (repeatable, HOST=PATH; '*' is default).
    #[arg(long = "mount", value_name = "HOST=PATH", value_parser = parse_mount)]
    mount: Vec<(String, PathBuf)>,

    /// Navbar center site title.
    #[arg(long, env = "ALBUM_SITE_NAME", default_value = "")]
    site_name: String,

    /// Navbar right-side note (free text, e.g. filing number).
    #[arg(long, env = "ALBUM_SITE_NOTE", default_value = "")]
    site_note: String,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_writer(std::io::stdout)
        .with_target(false)
        .init();

    let args = Args::parse();
    let addr = SocketAddr::new(args.addr, args.port);

    if args.mount.is_empty() {
        tracing::info!("mounts: (none)");
    } else {
        for (host, path) in &args.mount {
            tracing::info!("mounts: {host}={}", path.display());
        }
    }
    let mounts: MountTable = args.mount.into_iter().collect();
    if !args.site_name.is_empty() {
        tracing::info!("site_name: {}", args.site_name);
    }
    if !args.site_note.is_empty() {
        tracing::info!("site_note: {}", args.site_note);
    }

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("failed to bind listener");
    tracing::info!("listening on {addr}");
    let state = album::AppState::new(mounts, args.site_name, args.site_note);
    axum::serve(listener, album::build_app(state))
        .await
        .expect("server error");
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn parse_defaults_port_addr_mount() {
        let args = Args::try_parse_from(["album"]).unwrap();
        assert_eq!(args.port, 3000);
        assert_eq!(
            args.addr,
            std::net::IpAddr::V4(std::net::Ipv4Addr::UNSPECIFIED)
        );
        assert!(args.mount.is_empty());
    }

    #[test]
    fn parse_site_defaults_empty() {
        let args = Args::try_parse_from(["album"]).unwrap();
        assert_eq!(args.site_name, "");
        assert_eq!(args.site_note, "");
    }

    #[test]
    fn parse_site_name_and_note_flags() {
        let args = Args::try_parse_from([
            "album",
            "--site-name",
            "Album",
            "--site-note",
            "湘ICP备17022195号",
        ])
        .unwrap();
        assert_eq!(args.site_name, "Album");
        assert_eq!(args.site_note, "湘ICP备17022195号");
    }

    #[test]
    fn parse_mount_flag_repeated_lowercases_host() {
        let args = Args::try_parse_from(["album", "--mount", "DSM=/x", "--mount", "*=/y"]).unwrap();
        assert_eq!(
            args.mount,
            vec![
                ("dsm".to_string(), PathBuf::from("/x")),
                ("*".to_string(), PathBuf::from("/y")),
            ]
        );
    }
}
