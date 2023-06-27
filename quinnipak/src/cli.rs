use std::net::SocketAddr;

use crate::secure::SecurityMode;
use clap::Parser;

#[derive(Debug, Parser)]
#[command(author, version)]
pub struct Arguments {
    /// The address to bind on
    #[arg(short, long, default_value = "0.0.0.0:8000")]
    pub address: SocketAddr,

    /// The security mode
    #[command(subcommand)]
    pub mode: Option<SecurityMode>,

    /// The channel size for the chord broadcast
    #[arg(long, default_value_t = 64)]
    pub chords_channel_size: usize,

    /// Whether to monitor consumers for pings
    #[arg(long, default_value_t = false)]
    pub pingpong: bool,
}
