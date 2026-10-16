use crate::i18n::Language;
use serde::{Deserialize, Serialize};

pub const KEYS: usize = 15;
pub const COLUMNS: usize = 5;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DeckConfig {
    pub profiles: Vec<Profile>,
    pub active_profile: usize,
    pub brightness: u8,
    #[serde(default)]
    pub language: Language,
    #[serde(default)]
    pub obs: ObsConnection,
}

impl Default for DeckConfig {
    fn default() -> Self {
        Self {
            profiles: vec![Profile::new("Principal")],
            active_profile: 0,
            brightness: 80,
            language: Language::default(),
            obs: ObsConnection::default(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ObsConnection {
    pub address: String,
    pub password: String,
}

impl Default for ObsConnection {
    fn default() -> Self {
        Self {
            address: "ws://127.0.0.1:4455".into(),
            password: String::new(),
        }
    }
}

impl DeckConfig {
    pub fn current_profile(&self) -> &Profile {
        &self.profiles[self.active_profile.min(self.profiles.len() - 1)]
    }

    pub fn current_profile_mut(&mut self) -> &mut Profile {
        let index = self.active_profile.min(self.profiles.len() - 1);
        &mut self.profiles[index]
    }

    pub fn current_page(&self) -> &Page {
        self.current_profile().current_page()
    }

    pub fn current_page_mut(&mut self) -> &mut Page {
        self.current_profile_mut().current_page_mut()
    }

    pub fn normalize(&mut self) {
        if self.profiles.is_empty() {
            self.profiles.push(Profile::new("Principal"));
        }
        self.active_profile = self.active_profile.min(self.profiles.len() - 1);
        self.brightness = self.brightness.min(100);
        for profile in &mut self.profiles {
            if profile.pages.is_empty() {
                profile.pages.push(Page::new("Página 1"));
            }
            profile.active_page = profile.active_page.min(profile.pages.len() - 1);
            for page in &mut profile.pages {
                page.keys.resize_with(KEYS, KeyConfig::default);
                page.keys.truncate(KEYS);
            }
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Profile {
    pub name: String,
    pub pages: Vec<Page>,
    pub active_page: usize,
}

impl Profile {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            pages: vec![Page::new("Página 1")],
            active_page: 0,
        }
    }

    pub fn current_page(&self) -> &Page {
        &self.pages[self.active_page.min(self.pages.len() - 1)]
    }

    pub fn current_page_mut(&mut self) -> &mut Page {
        let index = self.active_page.min(self.pages.len() - 1);
        &mut self.pages[index]
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Page {
    pub name: String,
    pub keys: Vec<KeyConfig>,
}

impl Page {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            keys: vec![KeyConfig::default(); KEYS],
        }
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct KeyConfig {
    pub title: String,
    pub color: [u8; 3],
    pub image: String,
    pub action: Action,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", content = "value")]
pub enum Action {
    #[default]
    None,
    Open(String),
    Hotkey(String),
    Text(String),
    Media(MediaKey),
    Volume(i8),
    Brightness(u8),
    Sleep,
    NextPage,
    PreviousPage,
    GoToPage(usize),
    SwitchProfile(usize),
    Shell(String),
    HttpGet(String),
    Obs(ObsAction),
    Sequence(Vec<Action>),
    Delay(u64),
    Unsupported {
        uuid: String,
        settings: serde_json::Value,
    },
    Wheesy(String),
    WarpTab(String),
    WarpSsh {
        host: String,
        name: String,
    },
}

impl Action {
    pub const KINDS: [&'static str; 20] = [
        "Nenhuma ação",
        "Abrir app/arquivo/site",
        "Atalho",
        "Texto",
        "Mídia",
        "Volume",
        "Brilho do D6",
        "Apagar tela",
        "Próxima página",
        "Página anterior",
        "Ir para página",
        "Trocar perfil",
        "Comando shell",
        "HTTP GET",
        "OBS Studio",
        "Sequência",
        "Plugin importado",
        "Wheesy",
        "Warp: nova aba",
        "Warp: SSH",
    ];

    pub fn kind(&self) -> usize {
        match self {
            Self::None => 0,
            Self::Open(_) => 1,
            Self::Hotkey(_) => 2,
            Self::Text(_) => 3,
            Self::Media(_) => 4,
            Self::Volume(_) => 5,
            Self::Brightness(_) => 6,
            Self::Sleep => 7,
            Self::NextPage => 8,
            Self::PreviousPage => 9,
            Self::GoToPage(_) => 10,
            Self::SwitchProfile(_) => 11,
            Self::Shell(_) => 12,
            Self::HttpGet(_) => 13,
            Self::Obs(_) => 14,
            Self::Sequence(_) | Self::Delay(_) => 15,
            Self::Unsupported { .. } => 16,
            Self::Wheesy(_) => 17,
            Self::WarpTab(_) => 18,
            Self::WarpSsh { .. } => 19,
        }
    }

    pub fn from_kind(kind: usize) -> Self {
        match kind {
            1 => Self::Open(String::new()),
            2 => Self::Hotkey("cmd+shift+4".into()),
            3 => Self::Text(String::new()),
            4 => Self::Media(MediaKey::PlayPause),
            5 => Self::Volume(10),
            6 => Self::Brightness(80),
            7 => Self::Sleep,
            8 => Self::NextPage,
            9 => Self::PreviousPage,
            10 => Self::GoToPage(0),
            11 => Self::SwitchProfile(0),
            12 => Self::Shell(String::new()),
            13 => Self::HttpGet(String::new()),
            14 => Self::Obs(ObsAction::ToggleRecord),
            15 => Self::Sequence(Vec::new()),
            17 => Self::Wheesy(String::new()),
            18 => Self::WarpTab(String::new()),
            19 => Self::WarpSsh {
                host: String::new(),
                name: "deck-ssh".into(),
            },
            _ => Self::None,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum ObsAction {
    ToggleRecord,
    ToggleStream,
    ToggleVirtualCamera,
    SwitchScene(String),
    ToggleInputMute(String),
    ToggleSourceVisibility { scene: String, source: String },
    ToggleStudioMode,
}

impl ObsAction {
    pub const KINDS: [&'static str; 7] = [
        "Gravação",
        "Transmissão",
        "Câmera virtual",
        "Cena",
        "Mudo da entrada",
        "Visibilidade da fonte",
        "Modo estúdio",
    ];
    pub fn kind(&self) -> usize {
        match self {
            Self::ToggleRecord => 0,
            Self::ToggleStream => 1,
            Self::ToggleVirtualCamera => 2,
            Self::SwitchScene(_) => 3,
            Self::ToggleInputMute(_) => 4,
            Self::ToggleSourceVisibility { .. } => 5,
            Self::ToggleStudioMode => 6,
        }
    }
    pub fn from_kind(kind: usize) -> Self {
        match kind {
            1 => Self::ToggleStream,
            2 => Self::ToggleVirtualCamera,
            3 => Self::SwitchScene(String::new()),
            4 => Self::ToggleInputMute(String::new()),
            5 => Self::ToggleSourceVisibility {
                scene: String::new(),
                source: String::new(),
            },
            6 => Self::ToggleStudioMode,
            _ => Self::ToggleRecord,
        }
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq)]
pub enum MediaKey {
    PlayPause,
    Next,
    Previous,
    Mute,
    VolumeUp,
    VolumeDown,
}

impl MediaKey {
    pub const ALL: [Self; 6] = [
        Self::PlayPause,
        Self::Next,
        Self::Previous,
        Self::Mute,
        Self::VolumeUp,
        Self::VolumeDown,
    ];
    pub fn label(self) -> &'static str {
        match self {
            Self::PlayPause => "Play/Pause",
            Self::Next => "Próxima",
            Self::Previous => "Anterior",
            Self::Mute => "Mudo",
            Self::VolumeUp => "Volume +",
            Self::VolumeDown => "Volume -",
        }
    }
}
