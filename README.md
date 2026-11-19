# PXN V99 Control Center

Aplicativo desktop (Tauri 2 + Rust + React) para monitorar e controlar o volante PXN V99 em DirectInput, sem tirar o volante do jogo.

## Funções

| Área | Situação |
|---|---|
| Detecção, leitura de eixos/pedais/botões/câmbio, reconexão | Validado no volante |
| Centralização controlada pelo app (força constante em malha fechada) | Validado no volante |
| Efeitos de jogo do FS25: motor, embreagem, terreno, suspensão, impactos | Motor e embreagem validados; terreno e suspensão em ajuste |
| Calibração: conferência do centro e recalibração guiada pelo próprio volante | Implementado |
| Interface em português, inglês e espanhol | Implementado |

## Executar

```bash
npm install
```

```bash
npm run tauri dev
```

Instalador Windows (NSIS):

```bash
npm run tauri build
```

Requisitos: Node 20+, Rust (toolchain MSVC), Visual Studio com C++ e WebView2.

## Estrutura

```
src/                 Interface (React + TypeScript + Tailwind)
  lib/api.ts         Contrato com o núcleo (comandos e eventos)
  lib/store.ts       Estado, configurações locais e derivações
  components/        Mostradores (volante, pedais, botões, ganho)
  pages/             Telas
src-tauri/src/
  lib.rs             Comandos expostos à interface (validados)
  core.rs            Estado compartilhado e log de eventos
  model.rs           Tipos serializados
  hw/ffi.rs          API do Windows usada (kernel32, hid, setupapi)
  hw/discovery.rs    Localiza o V99 e identifica DirectInput/XInput
  hw/device.rs       Handle compartilhado, decodificação, escrita PID
  hw/worker.rs       Threads de leitura/reconexão e de emissão
  hw/centering.rs    Malha de centralização (força constante em malha fechada)
  game.rs            Servidor do pipe `\\.\pipe\PXNV99Telemetry` (telemetria do jogo)
fs25-mod/FS25_PXNV99Telemetry/   Mod do Farming Simulator 25 que envia a telemetria
tools/hid-probe.ps1  Sonda HID somente leitura (diagnóstico)
tools/fake-fs25-telemetry.ps1  Simula o mod para testar o app sem o jogo
```

## Regras de acesso ao hardware

- O volante é aberto em modo compartilhado; nunca em modo exclusivo.
- Só são enviados relatórios padrão USB HID PID, por ação explícita do usuário, validados e com limite de taxa.
- Nenhum comando proprietário ou não documentado é enviado.
