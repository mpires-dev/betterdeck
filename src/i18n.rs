use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Language {
    English,
    #[default]
    Portuguese,
}

impl Language {
    pub fn label(self) -> &'static str {
        match self {
            Self::English => "English",
            Self::Portuguese => "Português",
        }
    }

    pub fn tr(self, text: &'static str) -> &'static str {
        if self == Self::Portuguese {
            return text;
        }
        match text {
            "Conectar agentes ao BetterDeck" => "Connect agents to BetterDeck",
            "Conectar agentes" => "Connect agents",
            "Use o BetterDeck com seus clientes MCP locais." => {
                "Use BetterDeck with your local MCP clients."
            }
            "Fechar" => "Close",
            "●  Servidor MCP ativo · o BetterDeck pode ficar na barra de menus" => {
                "●  MCP server active · BetterDeck can stay in the menu bar"
            }
            "●  Servidor MCP indisponível · feche outra instância e reinicie o BetterDeck" => {
                "●  MCP server unavailable · close the other instance and restart BetterDeck"
            }
            "Claude Code e outros clientes MCP" => "Claude Code and other MCP clients",
            "Salve na raiz do projeto como .mcp.json." => "Save as .mcp.json in the project root.",
            "Adicione ao arquivo de configuração do Codex." => {
                "Add to the Codex configuration file."
            }
            "Acesso local a status, perfis, páginas, teclas, brilho, tela e OBS." => {
                "Local access to status, profiles, pages, keys, brightness, display, and OBS."
            }
            "✓ Copiado" => "✓ Copied",
            "Copiar código" => "Copy code",
            "D6 conectado" => "D6 connected",
            "D6 desconectado" => "D6 disconnected",
            "Setup MCP Connection" => "Set up MCP connection",
            "Endereço" => "Address",
            "Senha" => "Password",
            "PERFIS" => "PROFILES",
            "PÁGINAS" => "PAGES",
            "Excluir perfil" => "Delete profile",
            "Novo perfil" => "New profile",
            "Excluir página" => "Delete page",
            "Nova página" => "New page",
            "Brilho da tela" => "Display brightness",
            "Acordar D6" => "Wake D6",
            "Apagar D6" => "Turn off D6",
            "DICA RÁPIDA" => "QUICK TIP",
            "Clique para editar. Arraste uma tecla sobre outra para trocar de posição." => {
                "Click to edit. Drag a key onto another to swap positions."
            }
            "CONFIGURAÇÃO DE TECLAS" => "KEY CONFIGURATION",
            "Teclas do D6" => "D6 keys",
            "15 teclas · 5 × 3" => "15 keys · 5 × 3",
            "Teste a tecla selecionada" => "Test selected key",
            "Pressione a tecla física no D6 ou use o teste abaixo." => {
                "Press the physical D6 key or use the test below."
            }
            "Testar tecla" => "Test key",
            "Solte sobre outra tecla" => "Drop onto another key",
            "Clique para editar · arraste para trocar · botão direito para testar" => {
                "Click to edit · drag to swap · right-click to test"
            }
            "Selecionada" => "Selected",
            "Sem ação" => "No action",
            "EDITOR DE TECLA" => "KEY EDITOR",
            "Pronta para configurar" => "Ready to configure",
            "Adicione uma ação abaixo" => "Add an action below",
            "Título" => "Title",
            "Nome da tecla" => "Key name",
            "Cor da tecla" => "Key color",
            "Imagem da tecla" => "Key image",
            "Escolher imagem" => "Choose image",
            "Trocar imagem" => "Change image",
            "Imagem selecionada" => "Selected image",
            "Ação ao pressionar" => "On press",
            "Limpar tecla" => "Clear key",
            "Permissão do macOS" => "macOS permission",
            "Atalhos e texto exigem Acessibilidade. Comandos e URLs só são executados ao pressionar uma tecla configurada." => {
                "Shortcuts and text require Accessibility access. Commands and URLs run only when a configured key is pressed."
            }
            "Nome de app, caminho ou URL" => "App name, path, or URL",
            "Atalho (ex.: cmd+shift+4)" => "Shortcut (e.g. cmd+shift+4)",
            "Texto a digitar" => "Text to type",
            "Mudança de volume (0 = mudo)" => "Volume change (0 = mute)",
            "Página" => "Page",
            "Perfil" => "Profile",
            "Comando zsh" => "zsh command",
            "Nome da cena" => "Scene name",
            "Nome da entrada" => "Input name",
            "Cena" => "Scene",
            "Fonte" => "Source",
            "Comando do Wheesy" => "Wheesy command",
            "Pasta da nova aba" => "New tab directory",
            "Nome da configuração" => "Configuration name",
            "Atraso em milissegundos" => "Delay in milliseconds",
            "+ Ação" => "+ Action",
            "+ Atraso" => "+ Delay",
            "Nenhuma ação" => "No action",
            "Abrir app/arquivo/site" => "Open app/file/site",
            "Atalho" => "Hotkey",
            "Texto" => "Text",
            "Mídia" => "Media",
            "Volume" => "Volume",
            "Brilho do D6" => "D6 brightness",
            "Apagar tela" => "Turn off display",
            "Próxima página" => "Next page",
            "Página anterior" => "Previous page",
            "Ir para página" => "Go to page",
            "Trocar perfil" => "Switch profile",
            "Comando shell" => "Shell command",
            "Sequência" => "Sequence",
            "Plugin importado" => "Imported plugin",
            "Warp: nova aba" => "Warp: new tab",
            "Gravação" => "Recording",
            "Transmissão" => "Streaming",
            "Câmera virtual" => "Virtual camera",
            "Mudo da entrada" => "Input mute",
            "Visibilidade da fonte" => "Source visibility",
            "Modo estúdio" => "Studio Mode",
            "Próxima" => "Next",
            "Anterior" => "Previous",
            "Mudo" => "Mute",
            "Mostrar BetterDeck" => "Show BetterDeck",
            "Idioma" => "Language",
            "Procurando D6…" => "Looking for D6…",
            _ => text,
        }
    }

    pub fn status(self, status: &str) -> String {
        if self == Self::Portuguese {
            return status.to_owned();
        }
        if let Some(device) = status.strip_prefix("D6 conectado ") {
            return format!("D6 connected {device}");
        }
        if status == "D6 desconectado" {
            return "D6 disconnected".into();
        }
        if status == "Procurando D6…" {
            return "Looking for D6…".into();
        }
        if status == "D6 reconectando…" {
            return "Reconnecting to D6…".into();
        }
        if let Some(error) = status.strip_prefix("Erro ao salvar: ") {
            return format!("Save failed: {error}");
        }
        if let Some(error) = status.strip_prefix("Barra de menus: ") {
            return format!("Menu bar: {error}");
        }
        for (portuguese, english) in [
            ("Não foi possível abrir o D6: ", "Could not open D6: "),
            ("Leitura USB: ", "USB read: "),
            ("falha ao escrever no D6", "failed to write to D6"),
            ("escrita HID incompleta: ", "incomplete HID write: "),
            (
                "comando USB excede o pacote",
                "USB command exceeds packet size",
            ),
            ("imagem JPEG grande demais", "JPEG image is too large"),
            ("imagem JPEG excede 10 KiB", "JPEG image exceeds 10 KiB"),
            ("sequência profunda demais", "sequence is too deeply nested"),
            (
                "sequência limitada a 32 ações",
                "sequence is limited to 32 actions",
            ),
            (
                "enviar navegação à interface",
                "send navigation to the interface",
            ),
            (
                "informe o app, arquivo ou URL",
                "enter an app, file, or URL",
            ),
            ("iniciar teclado virtual", "start virtual keyboard"),
            ("digitar texto", "type text"),
            ("comando vazio", "empty command"),
            ("URL HTTP inválida", "invalid HTTP URL"),
            ("plugin ainda não portado: ", "plugin not yet ported: "),
            ("comando do Wheesy vazio", "empty Wheesy command"),
            ("localizar pasta pessoal", "find home directory"),
            ("resposta inválida do Wheesy", "invalid Wheesy response"),
            ("host SSH inválido", "invalid SSH host"),
            ("iniciar ação", "start action"),
            ("ação falhou: ", "action failed: "),
            ("enviar tecla de mídia", "send media key"),
            ("atalho vazio", "empty shortcut"),
            ("enviar atalho", "send shortcut"),
            ("tecla desconhecida: ", "unknown key: "),
            ("conectar ao OBS WebSocket", "connect to OBS WebSocket"),
            ("OBS não enviou Hello", "OBS did not send Hello"),
            ("OBS recusou autenticação", "OBS rejected authentication"),
            ("fonte não encontrada na cena", "source not found in scene"),
            ("estado da fonte ausente", "source state is missing"),
            (
                "estado do modo estúdio ausente",
                "Studio Mode state is missing",
            ),
            ("ler resposta do OBS", "read OBS response"),
            ("decodificar mensagem do OBS", "decode OBS message"),
            ("OBS encerrou a conexão", "OBS closed the connection"),
            ("criar ícone da barra de menus", "create menu bar icon"),
            (
                "AppKit exige a thread principal",
                "AppKit requires the main thread",
            ),
            (
                "macOS recusou alterar a presença no Dock",
                "macOS refused to change Dock visibility",
            ),
        ] {
            if let Some(rest) = status.strip_prefix(portuguese) {
                return format!("{english}{rest}");
            }
        }
        status.to_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::Language;
    use crate::model::DeckConfig;

    #[test]
    fn translates_interface_text_and_status() {
        assert_eq!(Language::English.tr("Ação ao pressionar"), "On press");
        assert_eq!(
            Language::Portuguese.tr("Ação ao pressionar"),
            "Ação ao pressionar"
        );
        assert_eq!(
            Language::English.status("D6 conectado (3142:0060)"),
            "D6 connected (3142:0060)"
        );
    }

    #[test]
    fn language_setting_preserves_older_configurations() {
        let mut saved = serde_json::to_value(DeckConfig::default()).unwrap();
        saved.as_object_mut().unwrap().remove("language");
        let old: DeckConfig = serde_json::from_value(saved).unwrap();
        assert_eq!(old.language, Language::Portuguese);

        let mut english = old;
        english.language = Language::English;
        let restored: DeckConfig =
            serde_json::from_value(serde_json::to_value(english).unwrap()).unwrap();
        assert_eq!(restored.language, Language::English);
    }
}
