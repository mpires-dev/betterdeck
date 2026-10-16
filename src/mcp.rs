use crate::model::{Action, DeckConfig, KEYS, KeyConfig, Page, Profile};
use anyhow::{Context, Result, bail};
use serde_json::{Value, json};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;
use std::time::Duration;

#[cfg(unix)]
use std::os::unix::net::{UnixListener, UnixStream};

pub struct Request {
    pub name: String,
    pub arguments: Value,
    pub reply: Sender<Result<Value>>,
}

pub struct Bridge {
    pub requests: Receiver<Request>,
    #[cfg(unix)]
    path: PathBuf,
}

impl Bridge {
    pub fn start(config_path: &Path) -> Result<Self> {
        #[cfg(unix)]
        {
            let path = socket_path(config_path);
            if path.exists() {
                if UnixStream::connect(&path).is_ok() {
                    bail!("outra instância do BetterDeck já está ativa");
                }
                std::fs::remove_file(&path)?;
            }
            let listener = UnixListener::bind(&path).context("abrir conexão MCP local")?;
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600))?;
            let (tx, requests) = mpsc::channel::<Request>();
            thread::spawn(move || {
                for stream in listener.incoming() {
                    let Ok(stream) = stream else { break };
                    let tx = tx.clone();
                    thread::spawn(move || {
                        let _ = handle_connection(stream, tx);
                    });
                }
            });
            Ok(Self { requests, path })
        }
        #[cfg(not(unix))]
        {
            let _ = config_path;
            bail!("MCP local requer Unix socket")
        }
    }
}

#[cfg(unix)]
impl Drop for Bridge {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}

fn socket_path(config_path: &Path) -> PathBuf {
    config_path.with_file_name("betterdeck.sock")
}

#[cfg(unix)]
fn handle_connection(mut stream: UnixStream, tx: Sender<Request>) -> Result<()> {
    stream.set_read_timeout(Some(Duration::from_secs(10)))?;
    let mut line = String::new();
    BufReader::new(stream.try_clone()?).read_line(&mut line)?;
    let input: Value = serde_json::from_str(&line)?;
    let name = input["name"]
        .as_str()
        .context("nome da ferramenta ausente")?
        .to_owned();
    let (reply, answer) = mpsc::channel();
    tx.send(Request {
        name,
        arguments: input["arguments"].clone(),
        reply,
    })?;
    let value = answer.recv_timeout(Duration::from_secs(10))?;
    let response = match value {
        Ok(value) => json!({"ok": true, "value": value}),
        Err(error) => json!({"ok": false, "error": error.to_string()}),
    };
    writeln!(stream, "{response}")?;
    Ok(())
}

#[cfg(unix)]
fn call_app(config_path: &Path, name: &str, arguments: &Value) -> Result<Value> {
    let mut stream = UnixStream::connect(socket_path(config_path))
        .context("BetterDeck não está aberto. Inicie o app para controlar o D6")?;
    stream.set_read_timeout(Some(Duration::from_secs(12)))?;
    writeln!(stream, "{}", json!({"name": name, "arguments": arguments}))?;
    let mut line = String::new();
    BufReader::new(stream).read_line(&mut line)?;
    if line.is_empty() {
        bail!("BetterDeck não respondeu")
    }
    let response: Value = serde_json::from_str(&line)?;
    if !response["ok"].as_bool().unwrap_or(false) {
        bail!(
            "{}",
            response["error"].as_str().unwrap_or("erro na operação")
        );
    }
    Ok(response["value"].clone())
}

#[cfg(not(unix))]
fn call_app(_: &Path, _: &str, _: &Value) -> Result<Value> {
    bail!("MCP local requer Unix socket")
}

fn tool(name: &str, description: &str, properties: Value, required: &[&str]) -> Value {
    json!({"name": name, "description": description,
        "inputSchema": {"type": "object", "properties": properties, "required": required,
            "additionalProperties": false}})
}

fn tools() -> Vec<Value> {
    vec![
        tool(
            "get_status",
            "Read D6 connection state, active profile/page and brightness",
            json!({}),
            &[],
        ),
        tool(
            "get_deck",
            "Read profiles, pages and all 15 key configurations (OBS password omitted)",
            json!({}),
            &[],
        ),
        tool(
            "set_brightness",
            "Set D6 display brightness from 0 to 100",
            json!({"value": {"type":"integer","minimum":0,"maximum":100}}),
            &["value"],
        ),
        tool(
            "set_active_profile",
            "Select a profile by zero-based index",
            json!({"profile": {"type":"integer","minimum":0}}),
            &["profile"],
        ),
        tool(
            "set_active_page",
            "Select a page in the active profile by zero-based index",
            json!({"page": {"type":"integer","minimum":0}}),
            &["page"],
        ),
        tool(
            "create_profile",
            "Create and select a profile",
            json!({"name": {"type":"string"}}),
            &["name"],
        ),
        tool(
            "create_page",
            "Create and select a page in the active profile",
            json!({"name": {"type":"string"}}),
            &["name"],
        ),
        tool(
            "configure_key",
            "Configure a key on the active page. action uses BetterDeck tagged JSON, e.g. {\"type\":\"Open\",\"value\":\"https://example.com\"} or {\"type\":\"Hotkey\",\"value\":\"cmd+shift+4\"}. Omitted fields stay unchanged",
            json!({"key": {"type":"integer","minimum":1,"maximum":15}, "title": {"type":"string"}, "color": {"type":"string","description":"#RRGGBB"}, "image": {"type":"string","description":"absolute local image path"}, "action": {"type":"object"}}),
            &["key"],
        ),
        tool(
            "clear_key",
            "Clear a key on the active page",
            json!({"key": {"type":"integer","minimum":1,"maximum":15}}),
            &["key"],
        ),
        tool(
            "press_key",
            "Run the action assigned to a key on the active page",
            json!({"key": {"type":"integer","minimum":1,"maximum":15}}),
            &["key"],
        ),
        tool(
            "set_sleep",
            "Turn D6 screen off or on",
            json!({"sleep": {"type":"boolean"}}),
            &["sleep"],
        ),
        tool(
            "set_obs_connection",
            "Set OBS WebSocket address and optionally password",
            json!({"address": {"type":"string"}, "password": {"type":"string"}}),
            &["address"],
        ),
    ]
}

pub fn run_stdio(config_path: &Path) -> Result<()> {
    let stdin = std::io::stdin();
    let mut stdout = std::io::stdout().lock();
    for line in stdin.lock().lines() {
        let line = line?;
        let request: Value = match serde_json::from_str(&line) {
            Ok(value) => value,
            Err(_) => continue,
        };
        let Some(id) = request.get("id") else {
            continue;
        };
        let method = request["method"].as_str().unwrap_or("");
        let result = match method {
            "initialize" => Ok(
                json!({"protocolVersion": request["params"]["protocolVersion"].as_str().unwrap_or("2025-06-18"), "capabilities": {"tools": {"listChanged": false}}, "serverInfo": {"name":"betterdeck","version":env!("CARGO_PKG_VERSION")}}),
            ),
            "ping" => Ok(json!({})),
            "tools/list" => Ok(json!({"tools": tools()})),
            "tools/call" => {
                let name = request["params"]["name"].as_str().unwrap_or("");
                let args = &request["params"]["arguments"];
                if !tools().iter().any(|tool| tool["name"] == name) {
                    Err(anyhow::anyhow!("ferramenta desconhecida: {name}"))
                } else {
                    call_app(config_path, name, args).map(
                        |value| json!({"content": [{"type":"text", "text": value.to_string()}]}),
                    )
                }
            }
            _ => Err(anyhow::anyhow!("método desconhecido: {method}")),
        };
        let response = match result {
            Ok(value) => json!({"jsonrpc":"2.0","id":id,"result":value}),
            Err(error) if method == "tools/call" => {
                json!({"jsonrpc":"2.0","id":id,"result":{"content":[{"type":"text","text":error.to_string()}],"isError":true}})
            }
            Err(error) => {
                json!({"jsonrpc":"2.0","id":id,"error":{"code":-32601,"message":error.to_string()}})
            }
        };
        writeln!(stdout, "{}", response)?;
        stdout.flush()?;
    }
    Ok(())
}

fn usize_arg(args: &Value, key: &str) -> Result<usize> {
    Ok(args[key]
        .as_u64()
        .context(format!("{key} deve ser inteiro positivo"))? as usize)
}

fn key_index(args: &Value) -> Result<usize> {
    let key = usize_arg(args, "key")?;
    if !(1..=KEYS).contains(&key) {
        bail!("key deve estar entre 1 e 15")
    }
    Ok(key - 1)
}

fn nonempty_name(args: &Value) -> Result<String> {
    let name = args["name"].as_str().context("name deve ser texto")?.trim();
    if name.is_empty() {
        bail!("name não pode ser vazio")
    }
    Ok(name.to_owned())
}

pub fn apply(app: &mut crate::BetterDeckApp, name: &str, args: &Value) -> Result<Value> {
    let config: &mut DeckConfig = &mut app.config;
    match name {
        "get_status" => Ok(
            json!({"connection": app.status, "active_profile": config.active_profile,
            "active_page": config.current_profile().active_page, "brightness": config.brightness,
            "sleep": app.asleep}),
        ),
        "get_deck" => {
            let mut value = serde_json::to_value(config)?;
            value["obs"]
                .as_object_mut()
                .map(|obs| obs.remove("password"));
            Ok(value)
        }
        "set_brightness" => {
            let value = usize_arg(args, "value")?;
            if value > 100 {
                bail!("value deve estar entre 0 e 100")
            }
            config.brightness = value as u8;
            Ok(json!({"brightness": value}))
        }
        "set_active_profile" => {
            let index = usize_arg(args, "profile")?;
            if index >= config.profiles.len() {
                bail!("perfil inexistente")
            }
            config.active_profile = index;
            Ok(json!({"active_profile": index}))
        }
        "set_active_page" => {
            let index = usize_arg(args, "page")?;
            if index >= config.current_profile().pages.len() {
                bail!("página inexistente")
            }
            config.current_profile_mut().active_page = index;
            Ok(json!({"active_page": index}))
        }
        "create_profile" => {
            config.profiles.push(Profile::new(nonempty_name(args)?));
            config.active_profile = config.profiles.len() - 1;
            Ok(json!({"active_profile": config.active_profile}))
        }
        "create_page" => {
            let profile = config.current_profile_mut();
            profile.pages.push(Page::new(nonempty_name(args)?));
            profile.active_page = profile.pages.len() - 1;
            Ok(json!({"active_page": profile.active_page}))
        }
        "configure_key" => {
            let index = key_index(args)?;
            let mut key = config.current_page().keys[index].clone();
            if let Some(title) = args.get("title") {
                key.title = title.as_str().context("title deve ser texto")?.to_owned();
            }
            if let Some(color) = args.get("color") {
                let color = color.as_str().context("color deve ser #RRGGBB")?;
                let hex = color.strip_prefix('#').context("color deve ser #RRGGBB")?;
                if hex.len() != 6 {
                    bail!("color deve ser #RRGGBB")
                }
                key.color = [
                    u8::from_str_radix(&hex[0..2], 16)?,
                    u8::from_str_radix(&hex[2..4], 16)?,
                    u8::from_str_radix(&hex[4..6], 16)?,
                ];
            }
            if let Some(image) = args.get("image") {
                let path = image.as_str().context("image deve ser caminho")?;
                if !path.is_empty()
                    && (!Path::new(path).is_absolute() || !Path::new(path).is_file())
                {
                    bail!("image deve apontar para um arquivo local absoluto existente")
                }
                key.image = path.to_owned();
            }
            if let Some(action) = args.get("action") {
                key.action =
                    serde_json::from_value::<Action>(action.clone()).context("ação inválida")?;
            }
            config.current_page_mut().keys[index] = key.clone();
            Ok(json!({"key": index+1, "configuration": key}))
        }
        "clear_key" => {
            let index = key_index(args)?;
            config.current_page_mut().keys[index] = KeyConfig::default();
            Ok(json!({"key": index+1, "cleared": true}))
        }
        "press_key" => {
            let index = key_index(args)?;
            app.press(index);
            Ok(json!({"key": index+1, "pressed": true}))
        }
        "set_sleep" => {
            let sleep = args["sleep"].as_bool().context("sleep deve ser booleano")?;
            if sleep != app.asleep {
                app.dispatch_action(Action::Sleep);
            }
            Ok(json!({"sleep": app.asleep}))
        }
        "set_obs_connection" => {
            config.obs.address = args["address"]
                .as_str()
                .context("address deve ser texto")?
                .to_owned();
            if let Some(password) = args.get("password") {
                config.obs.password = password
                    .as_str()
                    .context("password deve ser texto")?
                    .to_owned();
            }
            Ok(json!({"address": config.obs.address}))
        }
        _ => bail!("ferramenta desconhecida: {name}"),
    }
}
