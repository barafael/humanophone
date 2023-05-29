use std::{
    fs::File,
    io::BufReader,
    path::{Path, PathBuf},
};

use anyhow::Context;
use clap::Subcommand;

use rustls_pemfile::{certs, pkcs8_private_keys};
use tokio_rustls::rustls::{Certificate, PrivateKey};

const DEFAULT_PATH_TO_CERT: &str = "../jun/certs/localhost.crt";
const DEFAULT_PATH_TO_KEY: &str = "../jun/certs/localhost.key";

#[derive(Debug, Subcommand)]
#[group(required = true, multiple = true)]
pub enum SecurityMode {
    /// Use a certificate and key file for SSL-encrypted communication
    Secure {
        #[arg(short, long, default_value = DEFAULT_PATH_TO_CERT)]
        cert: PathBuf,

        #[arg(short, long, default_value = DEFAULT_PATH_TO_KEY)]
        key: PathBuf,
    },
}

pub fn load_certs(path: impl AsRef<Path>) -> anyhow::Result<Vec<Certificate>> {
    certs(&mut BufReader::new(File::open(path)?))
        .context("invalid cert")
        .map(|certs| certs.into_iter().map(Certificate).collect())
        .context("Failed to load local certificates")
}

pub fn load_keys(path: impl AsRef<Path>) -> anyhow::Result<Vec<PrivateKey>> {
    pkcs8_private_keys(&mut BufReader::new(File::open(path)?))
        .context("invalid key")
        .map(|keys| keys.into_iter().map(PrivateKey).collect())
        .context("Failed to load local keys")
}
