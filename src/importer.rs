use crate::model::{Action, COLUMNS, KEYS, KeyConfig, Profile};
use anyhow::{Context, Result};
use serde_json::Value;
use std::{
    fs,
    path::{Path, PathBuf},
};

pub fn import_profiles() -> Result<Vec<Profile>> {
    let home = directories::BaseDirs::new()
        .context("localizar pasta pessoal")?
        .home_dir()
        .to_path_buf();
    let root = home.join("Library/Application Support/HotSpot/StreamDock");
    let profile_root = root.join("profiles");
    let Ok(entries) = fs::read_dir(profile_root) else {
        return Ok(Vec::new());
    };
    let mut profiles = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().is_none_or(|ext| ext != "sdProfile") {
            continue;
        }
        let Ok(bytes) = fs::read(path.join("manifest.json")) else {
            continue;
        };
        let Ok(manifest) = serde_json::from_slice::<Value>(&bytes) else {
            continue;
        };
        let Some(actions) = manifest["Actions"].as_object() else {
            continue;
        };
        if actions.is_empty() {
            continue;
        }
        let name = manifest["Name"]
            .as_str()
            .filter(|s| !s.is_empty())
            .unwrap_or("FIFINE importado");
        let mut profile = Profile::new(name);
        for (position, data) in actions {
            let Some((col, row)) = parse_position(position) else {
                continue;
            };
            let index = row * COLUMNS + col;
            let uuid = data["UUID"].as_str().unwrap_or("");
            let settings = &data["Settings"];
            let state = &data["States"][0];
            let title = state["Title"].as_str().unwrap_or("").to_owned();
            let image = state["Image"]
                .as_str()
                .and_then(|name| resolve_image(&root, &path, uuid, name))
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned();
            profile.pages[0].keys[index] = KeyConfig {
                title,
                image,
                color: [25, 31, 46],
                action: map_action(uuid, settings),
            };
        }
        profiles.push(profile);
    }
    profiles.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(profiles)
}

fn parse_position(value: &str) -> Option<(usize, usize)> {
    let (col, row) = value.split_once(',')?;
    let col = col.parse::<usize>().ok()?;
    let row = row.parse::<usize>().ok()?;
    (col < COLUMNS && row * COLUMNS + col < KEYS).then_some((col, row))
}

fn resolve_image(root: &Path, profile: &Path, uuid: &str, name: &str) -> Option<PathBuf> {
    let candidates = if name.starts_with("Images/") {
        vec![profile.join(name)]
    } else if name.starts_with("static/") {
        let plugin_id = uuid
            .strip_suffix(uuid.rsplit_once('.')?.1)
            .unwrap_or(uuid)
            .trim_end_matches('.');
        vec![
            root.join("plugins")
                .join(format!("{plugin_id}.sdPlugin"))
                .join(name),
            root.join("plugins")
                .join(format!("{uuid}.sdPlugin"))
                .join(name),
            PathBuf::from("/Applications/fifine Control Deck.app/Contents/Resources/plugins")
                .join(format!("{uuid}.sdPlugin"))
                .join(name),
        ]
    } else {
        vec![PathBuf::from(name)]
    };
    for candidate in candidates {
        for path in [
            candidate.clone(),
            candidate.with_extension("png"),
            candidate.with_extension("jpg"),
            candidate.with_extension("jpeg"),
        ] {
            if path.is_file() {
                return Some(path);
            }
        }
    }
    None
}

fn setting<'a>(data: &'a Value, name: &str) -> &'a str {
    data[name].as_str().unwrap_or("")
}

fn map_action(uuid: &str, settings: &Value) -> Action {
    match uuid {
        "com.mpires.streamdock.macros.wheesy" => {
            Action::Wheesy(setting(settings, "command").into())
        }
        "com.mpires.streamdock.macros.warp" => Action::WarpTab(setting(settings, "path").into()),
        "com.mpires.streamdock.macros.ssh" => Action::WarpSsh {
            host: setting(settings, "host").into(),
            name: setting(settings, "name").into(),
        },
        "com.mpires.streamdock.macros.app" => Action::Open(setting(settings, "app").into()),
        "com.mpires.streamdock.macros.url" => Action::Open(setting(settings, "url").into()),
        "com.mpires.streamdock.macros.shell" => Action::Shell(setting(settings, "command").into()),
        "com.hotspot.streamdock.system.website" => Action::Open(setting(settings, "path").into()),
        "com.hotspot.streamdock.system.open" | "com.hotspot.streamdock.system.openApps" => {
            Action::Open(setting(settings, "path").into())
        }
        _ => Action::Unsupported {
            uuid: uuid.into(),
            settings: settings.clone(),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn converts_column_row_to_grid() {
        assert_eq!(parse_position("0,0"), Some((0, 0)));
        assert_eq!(parse_position("4,2"), Some((4, 2)));
        assert_eq!(parse_position("5,2"), None);
    }
    #[test]
    fn preserves_unknown_plugin() {
        assert!(matches!(
            map_action("example.plugin", &serde_json::json!({"x": 1})),
            Action::Unsupported { .. }
        ));
    }
}
