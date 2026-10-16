# Engenharia reversa: FIFINE Control Deck e D6

## Artefatos e observações

- Aplicativo instalado: `fifine Control Deck.app`, versão `2.10.179.426` conforme `Info.plist`.
- Biblioteca nativa analisada com REA/Ghidra: `libSDLibrary.1.0.25.dylib`, fat Mach-O SHA-256 `8f719a54277d062d478c3b03e6e9098d2a1a5515835f89641c19baaba9b6a734`. A fatia arm64 usada pelo Ghidra tem SHA-256 `a1c7420cbbe27c5cb6633cc9a399f16759640adf42961f55464bffd0b105cc40`.
- O barramento USB deste Mac mostrou `HOTSPOTEKUSB HID DEMO`, VID `0x3142`, PID `0x0060`. O diagnóstico HID do BetterDeck abriu a interface vendor `0xffa0:1`, serial individual, sem emitir comandos.
- O teste `--hardware-test` enviou um JPEG de 1812 bytes à tecla 1, com relatórios HID de 1025 bytes aceitos pelo driver. O aplicativo egui conectou à mesma interface e carregou o perfil importado sem erro de escrita observado.
- Uma foto do D6 fornecida pelo usuário confirmou que as 15 imagens chegaram às posições esperadas, mas cada painel apareceu girado 180°. O driver agora gira o JPEG completo para o PID `0x0060` antes do envio. Ainda falta conferir visualmente o resultado após essa correção e testar a leitura de uma tecla física.

## Evidências REA

| Afirmação | Evidence ID | Limite |
| --- | --- | --- |
| A biblioteca contém métodos de abertura, leitura e escrita HID em `SDDevice` | `ev_3cb122a90694b659790fc098571673a73ec2084133eeb6778dd1b18521a9e427` | Símbolos e funções estáticos, não rastreamento de execução |
| O SDK oferece comandos para limpar tecla, brilho, heartbeat, sono e envio de imagem | `ev_8c0633bfc949ba605cd9ff2559c04c02e5c3ecc2fe5e7ecdc9aa5f870afd8085` | Inventário de métodos, sem afirmar que todos são usados pelo D6 |
| `sendHeartbeatPack` monta cabeçalho `CRT` e comando `CONNECT` | `ev_75c765e14f6f502a15e7a3e4fa415529fcbf8964c1612fe3266f16ebbb949118` | Pseudocódigo Ghidra, não fonte original |
| `sendScreenOffPack`, `addAdjustBrightnessPack`, `getClearCommand` e `sendPixmapEndCommand` existem no SDK | `ev_8436ea2bd4a9798f7172e672476bd8b48384ba730ce93b093d5fef5c45dcbd55`, `ev_80276162ceee8d0e079829fad184a219d0aece0409b3afebb79424f583bba536`, `ev_0272a73d99534f6b79ee7b88bd86a53d506dfd6f41f98c300c5c792a5ae03c32`, `ev_0fb17b960cb8d54a5f186d14db11031d5d8c39fb6790156d7336832dfe586ea2` | Pseudocódigo estático |

O protocolo completo de pacotes, o mapa de imagens `[11…15, 6…10, 1…5]`, os eventos `ACK`, o intervalo de heartbeat e o tamanho dos JPEGs foram conferidos com a implementação aberta e os ensaios em hardware publicados no [DeckBridge](https://lukasmega.github.io/DeckBridge/devices/) e em seu [código fonte](https://github.com/lukasMega/DeckBridge/tree/main/ts/src/devices). Para o PID `0x0060`, o pacote tem **1024 bytes mais o byte de report ID**. Pacotes curtos podem ser aceitos pela API HID e descartados pelo firmware; por isso o PID determina o tamanho no driver.

Os manifests instalados em `Contents/Resources/plugins` mostram categorias de ações como abrir, páginas, mídia, texto, brilho, sequências, OBS, YouTube, vMix e áudio. O perfil do usuário fica em `~/Library/Application Support/HotSpot/StreamDock/profiles/*.sdProfile/manifest.json`, com posições `coluna,linha`, UUID do plugin, `Settings` e `States`. O importador lê esse formato sem alterar o original.

O protocolo OBS implementado segue a [especificação oficial obs-websocket 5](https://github.com/obsproject/obs-websocket/blob/master/docs/generated/protocol.md), incluindo Hello, Identify, autenticação SHA-256 e Request/RequestResponse.
