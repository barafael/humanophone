use clap::{Args, ValueHint};
use http::uri::Authority;

#[derive(Debug, Args)]
#[command(author, version)]
pub struct ClientArguments {
    #[arg(short, long, value_hint = ValueHint::Url, default_value = "0.0.0.0:8000")]
    pub url: Authority,

    /// The id to report to Quinnipak
    #[arg(short, long)]
    pub id: Option<String>,

    /// Whether to secure the connection (requires certificates for the server)
    #[arg(short, long, default_value_t = false)]
    pub secure: bool,

    /// Whether to periodically ping the server
    #[arg(short, long, default_value_t = false)]
    pub pingpong: bool,
}
