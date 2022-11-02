use std::{env::args, path::PathBuf, sync::Mutex, time::Duration};

use color_eyre::Result;
use futures_util::{SinkExt, TryStreamExt};
use once_cell::sync::Lazy;
use state::State;
use tokio::net::TcpStream;
use tokio_tungstenite::{connect_async, tungstenite::Message, MaybeTlsStream, WebSocketStream};
use tracing::instrument;

use rpc::{JsonRPC, Request, Response};

mod handler;
mod state;

pub static STATE: Lazy<Mutex<State>> = Lazy::new(|| {
    let path = args()
        .nth(1)
        .map(|p| p.parse::<PathBuf>().expect("first arg not a vaild path"));
    let state = state::load(path);
    Mutex::new(state)
});

#[tokio::main]
#[instrument]
async fn main() -> Result<()> {
    install_tracing();
    color_eyre::install()?;

    run().await
}

async fn run() -> Result<()> {
    loop {
        let (mut ws, _) = connect().await?;
        while let Ok(Some(msg)) = ws.try_next().await {
            let rpc: JsonRPC = msg.into_text()?.parse()?;
            match rpc {
                JsonRPC::Request(request) => {
                    let res: JsonRPC = handle_request(request).await?.into();
                    ws.send(Message::Text(res.to_string())).await?;
                }
                JsonRPC::Response(res) => {
                    tracing::warn!(?res, "received response, this should happen")
                }
                JsonRPC::Notification(notification) => tracing::info!(?notification),
            }
        }
    }
}

async fn connect() -> Result<(
    WebSocketStream<MaybeTlsStream<TcpStream>>,
    tokio_tungstenite::tungstenite::http::Response<()>,
)> {
    let mut retry_period = Duration::from_millis(500);
    loop {
        match connect_async("ws://localhost:8080/ws").await {
            Ok(ws) => return Ok(ws),
            Err(e) => {
                tracing::warn!(
                    "unable to astablish connection to server, retrying in {:?}",
                    retry_period
                );
                tracing::debug!(?e);
                tokio::time::sleep(retry_period).await;
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
async fn handle_request(request: Request) -> Result<Response> {
    tracing::debug!("start processing request");
    let response = match request.method.as_str() {
        "ping" => handler::ping(&request),
        "status" => handler::status(&request).await,
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
