use crate::{
    model::{Action, MediaKey, ObsConnection},
    obs,
};
use anyhow::{Context, Result, bail};
use enigo::{Direction, Enigo, Key, Keyboard, Settings};
use percent_encoding::{NON_ALPHANUMERIC, utf8_percent_encode};
use std::{
    io::{BufRead, BufReader, Write},
    os::unix::net::UnixStream,
    process::Command,
    sync::mpsc::Sender,
    thread,
    time::{Duration, Instant},
};

pub fn execute(
    action: &Action,
    connection: &ObsConnection,
    navigation: &Sender<Action>,
) -> Result<()> {
    execute_inner(action, connection, navigation, 0)
}

fn execute_inner(
    action: &Action,
    connection: &ObsConnection,
    navigation: &Sender<Action>,
    depth: usize,
) -> Result<()> {
    if depth > 8 {
        bail!("sequência profunda demais");
    }
    match action {
        Action::None => Ok(()),
        Action::NextPage
        | Action::PreviousPage
        | Action::GoToPage(_)
        | Action::SwitchProfile(_)
        | Action::Brightness(_)
        | Action::Sleep => navigation
            .send(action.clone())
            .context("enviar navegação à interface"),
        Action::Open(target) => {
            if target.trim().is_empty() {
                bail!("informe o app, arquivo ou URL");
            }
            if target.contains("://") || std::path::Path::new(target).exists() {
                run(Command::new("open").arg(target))
            } else {
                run(Command::new("open").arg("-a").arg(target))
            }
        }
        Action::Hotkey(combo) => hotkey(combo),
        Action::Text(value) => {
            let mut enigo = Enigo::new(&Settings::default()).context("iniciar teclado virtual")?;
            enigo.text(value).context("digitar texto")
        }
        Action::Media(key) => media(*key),
        Action::Volume(delta) => {
            let script = if *delta == 0 {
                "set volume output muted (not (output muted of (get volume settings)))".to_owned()
            } else {
                format!(
                    "set currentVolume to output volume of (get volume settings)\nset volume output volume (currentVolume + ({delta}))"
                )
            };
            run(Command::new("osascript").arg("-e").arg(script))
        }
        Action::Shell(script) => {
            if script.trim().is_empty() {
                bail!("comando vazio");
            }
            run(Command::new("/bin/zsh").arg("-c").arg(script))
        }
        Action::HttpGet(url) => {
            if !(url.starts_with("http://") || url.starts_with("https://")) {
                bail!("URL HTTP inválida");
            }
            run(Command::new("curl").args([
                "--fail",
                "--silent",
                "--show-error",
                "--max-time",
                "10",
                url,
            ]))
        }
        Action::Obs(action) => obs::execute(connection, action),
        Action::Unsupported { uuid, .. } => bail!("plugin ainda não portado: {uuid}"),
        Action::Wheesy(command) => wheesy(command),
        Action::WarpTab(path) => {
            let url = format!(
                "warp://action/new_tab?path={}",
                utf8_percent_encode(path, NON_ALPHANUMERIC)
            );
            run(Command::new("open").arg(url))
        }
        Action::WarpSsh { host, name } => warp_ssh(host, name),
        Action::Sequence(actions) => {
            if actions.len() > 32 {
                bail!("sequência limitada a 32 ações");
            }
            for step in actions {
                execute_inner(step, connection, navigation, depth + 1)?;
            }
            Ok(())
        }
        Action::Delay(ms) => {
            thread::sleep(Duration::from_millis((*ms).min(30_000)));
            Ok(())
        }
    }
}

fn wheesy(command: &str) -> Result<()> {
    if command.trim().is_empty() {
        bail!("comando do Wheesy vazio");
    }
    let home = directories::BaseDirs::new()
        .context("localizar pasta pessoal")?
        .home_dir()
        .to_path_buf();
    let socket = home.join("Library/Application Support/Wheesy/control.sock");
    match wheesy_request(&socket, command) {
        Ok(()) => return Ok(()),
        Err(error)
            if error.downcast_ref::<std::io::Error>().is_none_or(|io| {
                !matches!(
                    io.kind(),
                    std::io::ErrorKind::NotFound | std::io::ErrorKind::ConnectionRefused
                )
            }) =>
        {
            return Err(error);
        }
        Err(_) => {}
    }
    let app = home.join("Development/weesy/dist/Wheesy.app");
    run(Command::new("open").arg("-g").arg(&app))?;
    let deadline = Instant::now() + Duration::from_secs(12);
    loop {
        thread::sleep(Duration::from_millis(250));
        match wheesy_request(&socket, command) {
            Ok(()) => return Ok(()),
            Err(error) if Instant::now() >= deadline => return Err(error),
            Err(_) => {}
        }
    }
}

fn wheesy_request(socket: &std::path::Path, command: &str) -> Result<()> {
    let mut stream = UnixStream::connect(socket)?;
    stream.set_read_timeout(Some(Duration::from_secs(4)))?;
    stream.write_all(
        serde_json::json!({"action": command})
            .to_string()
            .as_bytes(),
    )?;
    stream.write_all(b"\n")?;
    let mut line = String::new();
    BufReader::new(stream).read_line(&mut line)?;
    let response: serde_json::Value =
        serde_json::from_str(&line).context("resposta inválida do Wheesy")?;
    if response["ok"] != true {
        bail!(
            "Wheesy: {}",
            response["error"].as_str().unwrap_or("comando recusado")
        );
    }
    Ok(())
}

fn warp_ssh(host: &str, name: &str) -> Result<()> {
    if host.is_empty()
        || !host
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || ".-_@:".contains(c))
    {
        bail!("host SSH inválido");
    }
    let home = directories::BaseDirs::new()
        .context("localizar pasta pessoal")?
        .home_dir()
        .to_path_buf();
    let config = if name.trim().is_empty() {
        "deck-ssh".to_owned()
    } else {
        name.chars()
            .map(|c| {
                if c.is_ascii_alphanumeric() || ".-_".contains(c) {
                    c
                } else {
                    '-'
                }
            })
            .collect()
    };
    let directory = home.join(".warp/launch_configurations");
    std::fs::create_dir_all(&directory)?;
    std::fs::write(
        directory.join(format!("{config}.yaml")),
        format!(
            "---\nname: {config}\nwindows:\n  - tabs:\n      - title: {host}\n        layout:\n          cwd: {}\n          commands:\n            - exec: ssh {host}\n",
            home.display()
        ),
    )?;
    run(Command::new("open").arg(format!("warp://launch/{config}")))
}

fn run(command: &mut Command) -> Result<()> {
    let output = command.output().context("iniciar ação")?;
    if !output.status.success() {
        bail!(
            "ação falhou: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    Ok(())
}

fn media(key: MediaKey) -> Result<()> {
    let key = match key {
        MediaKey::PlayPause => Key::MediaPlayPause,
        MediaKey::Next => Key::MediaNextTrack,
        MediaKey::Previous => Key::MediaPrevTrack,
        MediaKey::Mute => Key::VolumeMute,
        MediaKey::VolumeUp => Key::VolumeUp,
        MediaKey::VolumeDown => Key::VolumeDown,
    };
    let mut enigo = Enigo::new(&Settings::default()).context("iniciar teclado virtual")?;
    enigo
        .key(key, Direction::Click)
        .context("enviar tecla de mídia")
}

fn hotkey(combo: &str) -> Result<()> {
    let mut parts: Vec<&str> = combo
        .split('+')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .collect();
    let Some(last) = parts.pop() else {
        bail!("atalho vazio");
    };
    let key = parse_key(last)?;
    let modifiers: Vec<Key> = parts.into_iter().map(parse_key).collect::<Result<_>>()?;
    let mut enigo = Enigo::new(&Settings::default()).context("iniciar teclado virtual")?;
    for modifier in &modifiers {
        enigo.key(*modifier, Direction::Press)?;
    }
    let result = enigo.key(key, Direction::Click).context("enviar atalho");
    for modifier in modifiers.iter().rev() {
        let _ = enigo.key(*modifier, Direction::Release);
    }
    result
}

fn parse_key(value: &str) -> Result<Key> {
    let normalized = value.to_ascii_lowercase();
    let key = match normalized.as_str() {
        "cmd" | "command" | "meta" => Key::Meta,
        "ctrl" | "control" => Key::Control,
        "shift" => Key::Shift,
        "alt" | "option" => Key::Alt,
        "enter" | "return" => Key::Return,
        "esc" | "escape" => Key::Escape,
        "space" => Key::Space,
        "tab" => Key::Tab,
        "backspace" => Key::Backspace,
        "up" => Key::UpArrow,
        "down" => Key::DownArrow,
        "left" => Key::LeftArrow,
        "right" => Key::RightArrow,
        _ if normalized.chars().count() == 1 => {
            Key::Unicode(normalized.chars().next().unwrap_or(' '))
        }
        _ => bail!("tecla desconhecida: {value}"),
    };
    Ok(key)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parses_shortcut_tokens() {
        assert_eq!(parse_key("Cmd").expect("cmd"), Key::Meta);
        assert_eq!(parse_key("4").expect("digit"), Key::Unicode('4'));
        assert!(parse_key("nonexistent").is_err());
    }
}
