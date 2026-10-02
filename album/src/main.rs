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
    let _ = mounts; // no consumer yet (plan: HTTP wiring deferred)

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("failed to bind listener");
    tracing::info!("listening on {addr}");
    axum::serve(listener, album::build_app())
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
    fn parse_mount_flag_repeated_lowercases_host() {
        let args =
            Args::try_parse_from(["album", "--mount", "DSM=/x", "--mount", "*=/y"]).unwrap();
        assert_eq!(
            args.mount,
            vec![
                ("dsm".to_string(), PathBuf::from("/x")),
                ("*".to_string(), PathBuf::from("/y")),
            ]
        );
    }
}
