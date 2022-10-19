use color_eyre::Result;
use futures_util::{SinkExt, TryStreamExt};
use tokio_tungstenite::{connect_async, tungstenite::Message};

use crate::rpc::{JsonRPC, Request};

pub async fn run() -> Result<()> {
    let (mut ws, _) = connect_async("ws://localhost:8080/ws").await?;
    let req: JsonRPC = Request::new(1.into(), "hello".to_string(), ()).into();
    ws.send(Message::Text(req.to_string())).await?;

    if let Some(msg) = ws.try_next().await? {
        let rpc: JsonRPC = msg.into_text()?.parse()?;
        println!("{rpc:?}");
    };
    Ok(())
}
