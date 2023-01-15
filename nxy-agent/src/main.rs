use std::{env::args, net::TcpStream, path::PathBuf, sync::Mutex, time::Duration};

use eyre::Result;
use once_cell::sync::Lazy;
use state::State;
use tracing::instrument;
use tungstenite::{client::connect, stream::MaybeTlsStream, Message, WebSocket};

use nxy_common::{JsonRPC, Request, Response};

mod activate;
mod handler;
mod state;

pub static STATE: Lazy<Mutex<State>> = Lazy::new(|| {
    let path = args()
        .nth(1)
        .map(|p| p.parse::<PathBuf>().expect("first arg not a vaild path"));
    let state = state::load(path);
    Mutex::new(state)
});

#[instrument]
fn main() -> Result<()> {
    install_tracing();

    let server_url = std::env::args()
        .nth(2)
        .expect("second argument must be server address eg. ws://localhost:8080");

    run(&server_url)
}

fn run(server_url: &str) -> Result<()> {
    loop {
        let (mut socket, _) = connect_with_backoff(server_url)?;
        loop {
            let msg = socket.read_message()?;
            let rpc: JsonRPC = msg.into_text()?.parse()?;
            match rpc {
                JsonRPC::Request(request) => {
                    let res: JsonRPC = handle_request(request)?.into();
                    socket.write_message(Message::Text(res.to_string()))?;
                }
                JsonRPC::Response(res) => {
                    tracing::warn!(?res, "received response, this should happen")
                }
                JsonRPC::Notification(notification) => tracing::info!(?notification),
            }
        }
    }
}

fn connect_with_backoff(
    server_url: &str,
) -> Result<(
    WebSocket<MaybeTlsStream<TcpStream>>,
    tungstenite::handshake::client::Response,
)> {
    let mut retry_period = Duration::from_millis(500);
    loop {
        match connect(format!("{server_url}/api/v1/agent/ws")) {
            Ok(ws) => return Ok(ws),
            Err(e) => {
                tracing::warn!(
                    "unable to astablish connection to server, retrying in {:?}",
                    retry_period
                );
                tracing::debug!(?e);
                std::thread::sleep(retry_period);
                retry_period = backoff(retry_period);
            }
        }
    }
}

fn backoff(duration: Duration) -> Duration {
    if duration >= Duration::from_secs(4) {
        return Duration::from_secs(4);
    }
    duration * 2
}

#[instrument(skip_all, err, fields(id = %request.id, method = request.method))]
fn handle_request(request: Request) -> Result<Response> {
    tracing::debug!("start processing request");
    let response = match request.method.as_str() {
        "$/ping" => handler::ping(&request),
        "$/status" => handler::status(&request),
        "$/download" => handler::download(&request),
        "$/activate" => handler::activate(&request),
        _ => handler::unknown(&request),
    };
    tracing::debug!("done processing request");
    response
}

fn install_tracing() {
    use tracing_error::ErrorLayer;
    use tracing_subscriber::prelude::*;
    use tracing_subscriber::{fmt, EnvFilter};

    let fmt_layer = fmt::layer().pretty();
    let filter_layer = EnvFilter::try_from_default_env()
        .or_else(|_| EnvFilter::try_new("info"))
        .unwrap();

    tracing_subscriber::registry()
        .with(filter_layer)
        .with(fmt_layer)
        .with(ErrorLayer::default())
        .init();
}
