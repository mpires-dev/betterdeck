use crate::model::{ObsAction, ObsConnection};
use anyhow::{Context, Result, bail};
use base64::{Engine, engine::general_purpose::STANDARD};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::net::TcpStream;
use tungstenite::{Message, WebSocket, connect, stream::MaybeTlsStream};

type Socket = WebSocket<MaybeTlsStream<TcpStream>>;

pub fn execute(connection: &ObsConnection, action: &ObsAction) -> Result<()> {
    let (mut socket, _) =
        connect(connection.address.as_str()).context("conectar ao OBS WebSocket")?;
    let hello = read_json(&mut socket)?;
    if hello["op"] != 0 {
        bail!("OBS não enviou Hello");
    }
    let mut identify = json!({"rpcVersion": 1});
    if let (Some(challenge), Some(salt)) = (
        hello["d"]["authentication"]["challenge"].as_str(),
        hello["d"]["authentication"]["salt"].as_str(),
    ) {
        let secret = STANDARD.encode(Sha256::digest(
            format!("{}{}", connection.password, salt).as_bytes(),
        ));
        let response = STANDARD.encode(Sha256::digest(format!("{secret}{challenge}").as_bytes()));
        identify["authentication"] = response.into();
    }
    socket.send(Message::Text(
        json!({"op": 1, "d": identify}).to_string().into(),
    ))?;
    if read_json(&mut socket)?["op"] != 2 {
        bail!("OBS recusou autenticação");
    }
    match action {
        ObsAction::ToggleRecord => {
            request(&mut socket, "ToggleRecord", json!({}))?;
        }
        ObsAction::ToggleStream => {
            request(&mut socket, "ToggleStream", json!({}))?;
        }
        ObsAction::ToggleVirtualCamera => {
            request(&mut socket, "ToggleVirtualCam", json!({}))?;
        }
        ObsAction::SwitchScene(name) => {
            request(
                &mut socket,
                "SetCurrentProgramScene",
                json!({"sceneName": name}),
            )?;
        }
        ObsAction::ToggleInputMute(name) => {
            request(&mut socket, "ToggleInputMute", json!({"inputName": name}))?;
        }
        ObsAction::ToggleSourceVisibility { scene, source } => {
            let item = request(
                &mut socket,
                "GetSceneItemId",
                json!({"sceneName": scene, "sourceName": source}),
            )?;
            let id = item["sceneItemId"]
                .as_i64()
                .context("fonte não encontrada na cena")?;
            let state = request(
                &mut socket,
                "GetSceneItemEnabled",
                json!({"sceneName": scene, "sceneItemId": id}),
            )?;
            let enabled = state["sceneItemEnabled"]
                .as_bool()
                .context("estado da fonte ausente")?;
            request(
                &mut socket,
                "SetSceneItemEnabled",
                json!({"sceneName": scene, "sceneItemId": id, "sceneItemEnabled": !enabled}),
            )?;
        }
        ObsAction::ToggleStudioMode => {
            let state = request(&mut socket, "GetStudioModeEnabled", json!({}))?;
            let enabled = state["studioModeEnabled"]
                .as_bool()
                .context("estado do modo estúdio ausente")?;
            request(
                &mut socket,
                "SetStudioModeEnabled",
                json!({"studioModeEnabled": !enabled}),
            )?;
        }
    }
    Ok(())
}

fn request(socket: &mut Socket, name: &str, data: Value) -> Result<Value> {
    socket.send(Message::Text(json!({"op": 6, "d": {"requestType": name, "requestId": "betterdeck-1", "requestData": data}}).to_string().into()))?;
    loop {
        let response = read_json(socket)?;
        if response["op"] != 7 {
            continue;
        }
        if response["d"]["requestStatus"]["result"] != true {
            bail!(
                "OBS {}: {}",
                name,
                response["d"]["requestStatus"]["comment"]
                    .as_str()
                    .unwrap_or("falha")
            );
        }
        return Ok(response["d"]["responseData"].clone());
    }
}

fn read_json(socket: &mut Socket) -> Result<Value> {
    loop {
        match socket.read().context("ler resposta do OBS")? {
            Message::Text(text) => {
                return serde_json::from_str(&text).context("decodificar mensagem do OBS");
            }
            Message::Close(_) => bail!("OBS encerrou a conexão"),
            _ => {}
        }
    }
}
