use std::net::SocketAddr;

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
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_writer(std::io::stdout)
        .with_target(false)
        .init();

    let args = Args::parse();
    let addr = SocketAddr::new(args.addr, args.port);
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("failed to bind listener");
    tracing::info!("listening on {addr}");
    axum::serve(listener, album::build_app())
        .await
        .expect("server error");
}
