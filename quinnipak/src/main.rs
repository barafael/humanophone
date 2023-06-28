use std::sync::Arc;

use anyhow::Context;
use clap::Parser;
use quinnipak::handle_connection;
use quinnipak::secure::{load_certs, load_keys};
use quinnipak::{cli::Arguments, secure::SecurityMode};
use tokio::{net::TcpListener, sync::broadcast};
use tokio_rustls::TlsAcceptor;
use tracing::{info, warn};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let args = Arguments::parse();

    let (chords_tx, _) = broadcast::channel(args.chords_channel_size);

    info!("Listening on {:?}", args.address);
    let listener = TcpListener::bind(args.address).await?;

    let acceptor = match args.mode {
        None => None,
        Some(SecurityMode::Secure { cert, key }) => {
            info!("Loading certificates and keys");
            let certs = load_certs(cert)?;
            let mut keys = load_keys(key)?;

            let config = rustls::ServerConfig::builder()
                .with_safe_defaults()
                .with_no_client_auth()
                .with_single_cert(certs, keys.remove(0))
                .context("Create server config")?;
            Some(TlsAcceptor::from(Arc::new(config)))
        }
    };

    while let Ok((stream, _)) = listener.accept().await {
        let chords_tx = chords_tx.clone();
        let acceptor = acceptor.clone();

        tokio::spawn(async move {
            if let Err(e) = handle_connection(stream, chords_tx, acceptor, args.pingpong).await {
                warn!("Error while handling connection: {e:?}");
            }
        });
    }
    Ok(())
}
