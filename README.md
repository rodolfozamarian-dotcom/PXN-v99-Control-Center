# PXN V99 Control Center

**Português** · [English](#english)

Programa gratuito para quem joga com o volante **PXN V99** no PC. Ele resolve a mola de centralização dura do volante e, no **Farming Simulator 25**, faz o volante vibrar com o motor, a embreagem e o terreno.

## ⬇ Download

Baixe sempre a versão mais nova em **[Releases](https://github.com/rodolfozamarian-dotcom/PXN-v99-Control-Center/releases/latest)**. Lá tem dois arquivos:

| Arquivo | O que é | Precisa? |
|---|---|---|
| `PXN.V99.Control.Center_..._x64-setup.exe` | O **aplicativo** para Windows. É ele que controla o volante. | Sim, sempre |
| `FS25_PXNV99Telemetry.zip` | Um **mod para o Farming Simulator 25**. Ele manda os dados do trator (motor, embreagem, terreno) para o aplicativo. | Só se quiser as vibrações no FS25 |

## O que ele faz

- **Mola de centralização ajustável**: você escolhe de 0% a 100% a força com que o volante volta ao centro, na hora, sem precisar trocar o volante para XInput.
- **Vibrações no Farming Simulator 25** (com o mod):
  - **Motor**: vibra mais em marcha lenta e com o motor puxado, e quase nada em alta rotação.
  - **Embreagem**: soltou de uma vez e o motor afundou, o volante trepida forte; soltou devagar, só um toque quando a marcha engata.
  - **Terreno**: asfalto liso, terra e lavoura ásperas, mais forte quanto mais rápido.
  - **Suspensão** e **impactos**: trancos quando uma roda passa num buraco.
- **Monitor ao vivo** do volante, dos pedais, das embreagens das borboletas e do câmbio H.
- **Calibração do centro** guiada.
- Em português, inglês e espanhol.

## Instalação

### Antes de começar

- Windows 10 ou 11.
- Volante PXN V99 ligado no USB e em modo **DirectInput** (a luz do volante fica **vermelha**, que é o modo padrão no PC).

### Passo 1: instalar o aplicativo

1. Baixe o arquivo `PXN.V99.Control.Center_..._x64-setup.exe` em [Releases](https://github.com/rodolfozamarian-dotcom/PXN-v99-Control-Center/releases/latest).
2. Dê dois cliques nele.
3. O Windows pode mostrar uma tela azul dizendo **"O Windows protegeu o computador"**. Isso acontece com todo programa sem assinatura digital paga. Clique em **Mais informações** e depois em **Executar assim mesmo**.
4. Siga o instalador. No fim, o **PXN V99 Control Center** aparece no menu Iniciar.

### Passo 2: instalar o mod no Farming Simulator 25 (opcional, para as vibrações)

1. Baixe o arquivo `FS25_PXNV99Telemetry.zip`.
2. **Não descompacte.** O Farming Simulator lê o mod direto do `.zip`.
3. Copie o arquivo para a pasta de mods do jogo:

   `Documentos\My Games\FarmingSimulator2025\mods`

   Se a sua pasta Documentos fica no OneDrive, o caminho é `OneDrive\Documentos\My Games\FarmingSimulator2025\mods`.
4. Abra o Farming Simulator 25 e carregue o seu savegame. Na **tela de seleção de mods**, marque **PXN V99 Telemetria** e continue.

O mod não muda nada no seu jogo nem no savegame: ele só lê os dados do trator que você está dirigindo e manda para o aplicativo.

### Passo 3: usar

1. Abra o **PXN V99 Control Center** (pode ser antes ou depois do jogo).
2. Para ajustar a mola, vá em **Force Feedback** e clique em **Iniciar centralização**.
   - Na primeira vez, o app pede para você **soltar o volante**: ele dá um ou dois empurrõezinhos para descobrir o sentido do motor. Isso só acontece uma vez.
   - Mexa em **Força de centralização** até ficar do seu gosto. Muda na hora, mesmo com o jogo aberto.
3. Para as vibrações, vá em **Efeitos de jogo** e marque **Ativar efeitos de jogo**. Cada efeito tem o seu ajuste.
4. Pode minimizar o app: ele fica no ícone perto do relógio e continua funcionando. Para fechar de vez, clique com o botão direito no ícone e escolha **Sair**.

Ao fechar o app, o volante volta à mola original de fábrica.

## Dúvidas comuns

**O app diz "Volante não conectado".**
Confira se o volante está no USB e com a luz vermelha (DirectInput). Em XInput o app não consegue controlar a mola.

**Em "Efeitos de jogo" aparece "Jogo não conectado".**
Confira se o `.zip` está na pasta `mods` (sem descompactar) e se o mod foi marcado na seleção de mods do savegame. Ele conecta sozinho em até 2 segundos depois que o jogo carrega.

**A vibração está fraca ou forte demais.**
Ajuste o **Volume geral** e o slider de cada efeito em **Efeitos de jogo**.

**Funciona em outros jogos?**
A mola ajustável funciona em qualquer jogo. As vibrações, por enquanto, só no Farming Simulator 25.

**Funciona no multiplayer?**
Por enquanto o mod é só para singleplayer.

**Funciona em outros volantes?**
Foi feito e testado no PXN V99. O PXN V10 usa a mesma identificação USB e pode funcionar, mas não foi testado.

**É seguro?**
O app só usa os comandos padrão de force feedback que qualquer jogo usa. Não precisa de administrador nem de internet, e tudo roda no seu PC. O código está todo aqui no repositório.

**Como desinstalar?**
Pelo **Configurações do Windows → Aplicativos**, como qualquer programa. Para tirar o mod, apague o `.zip` da pasta `mods`.

---

<a id="english"></a>

# English

Free tool for people who play with the **PXN V99** wheel on PC. It fixes the wheel's stiff centering spring and, in **Farming Simulator 25**, makes the wheel vibrate with the engine, clutch and terrain.

## ⬇ Download

Always get the latest version from **[Releases](https://github.com/rodolfozamarian-dotcom/PXN-v99-Control-Center/releases/latest)**. There are two files:

| File | What it is | Needed? |
|---|---|---|
| `PXN.V99.Control.Center_..._x64-setup.exe` | The Windows **app**. It controls the wheel. | Yes, always |
| `FS25_PXNV99Telemetry.zip` | A **Farming Simulator 25 mod**. It sends the tractor's data (engine, clutch, terrain) to the app. | Only for the FS25 vibrations |

## What it does

- **Adjustable centering spring**: choose from 0% to 100% how hard the wheel pulls back to center, live, without switching the wheel to XInput.
- **Farming Simulator 25 vibrations** (with the mod):
  - **Engine**: stronger at idle and when lugging, fading at high RPM.
  - **Clutch**: dump it and the engine bogs down, the wheel shudders hard; release it slowly and you just feel the gear bite.
  - **Terrain**: smooth asphalt, rough dirt and fields, stronger the faster you go.
  - **Suspension** and **impacts**: kicks when a wheel hits a bump.
- **Live monitor** of the wheel, pedals, paddle clutches and H shifter.
- Guided **center calibration**.
- English, Portuguese and Spanish.

## Installation

### Before you start

- Windows 10 or 11.
- PXN V99 plugged in over USB and in **DirectInput** mode (the wheel light is **red**, the default mode on PC).

### Step 1: install the app

1. Download `PXN.V99.Control.Center_..._x64-setup.exe` from [Releases](https://github.com/rodolfozamarian-dotcom/PXN-v99-Control-Center/releases/latest).
2. Double-click it.
3. Windows may show a blue screen saying **"Windows protected your PC"**. That happens with any program without a paid digital signature. Click **More info**, then **Run anyway**.
4. Follow the installer. When it's done, **PXN V99 Control Center** is in the Start menu.

### Step 2: install the Farming Simulator 25 mod (optional, for the vibrations)

1. Download `FS25_PXNV99Telemetry.zip`.
2. **Don't unzip it.** Farming Simulator reads the mod straight from the `.zip`.
3. Copy the file into the game's mods folder:

   `Documents\My Games\FarmingSimulator2025\mods`

   If your Documents folder is in OneDrive, the path is `OneDrive\Documents\My Games\FarmingSimulator2025\mods`.
4. Start Farming Simulator 25 and load your savegame. On the **mod selection screen**, tick **PXN V99 Telemetry** and continue.

The mod doesn't change your game or your savegame: it only reads the data of the tractor you're driving and sends it to the app.

### Step 3: use it

1. Open **PXN V99 Control Center** (before or after the game, either works).
2. To adjust the spring, go to **Force Feedback** and click **Start centering**.
   - The first time, the app asks you to **let go of the wheel**: it gives one or two small pushes to learn the motor direction. This only happens once.
   - Move **Centering strength** until it feels right. It changes instantly, even with the game running.
3. For the vibrations, go to **Game effects** and tick **Enable game effects**. Each effect has its own slider.
4. You can minimize the app: it stays as an icon next to the clock and keeps working. To close it for good, right-click the icon and choose **Quit**.

When the app closes, the wheel goes back to its original factory spring.

## FAQ

**The app says "Wheel not connected".**
Make sure the wheel is on USB with the red light (DirectInput). In XInput the app can't control the spring.

**"Game effects" says "Game not connected".**
Make sure the `.zip` is in the `mods` folder (not unzipped) and that the mod is ticked in the savegame's mod selection. It connects on its own within 2 seconds after the game loads.

**The vibration is too weak or too strong.**
Adjust **Master volume** and each effect's slider in **Game effects**.

**Does it work in other games?**
The adjustable spring works in any game. The vibrations are Farming Simulator 25 only for now.

**Does it work in multiplayer?**
The mod is singleplayer only for now.

**Does it work with other wheels?**
It was built and tested on the PXN V99. The PXN V10 uses the same USB ID and might work, but it's untested.

**Is it safe?**
The app only uses the standard force feedback commands any game uses. No admin rights, no internet, everything runs on your PC. All the code is in this repository.

**How do I uninstall?**
Through **Windows Settings → Apps**, like any program. To remove the mod, delete the `.zip` from the `mods` folder.

---

## Para desenvolvedores / For developers

Tauri 2 + Rust + React/TypeScript. Requisitos: Node 20+, Rust (toolchain MSVC), Visual Studio com C++ e WebView2.

```bash
npm install
```

```bash
npm run tauri dev
```

```bash
npm run tauri build
```

```
src/                       Interface (React + TypeScript + Tailwind)
  lib/i18n.ts              Textos em PT/EN/ES
  dev/mockTauri.ts         Simulador do núcleo para desenvolver no navegador (?mock)
src-tauri/src/
  hw/device.rs             Acesso HID compartilhado e relatórios USB HID PID
  hw/centering.rs          Centralização (força constante em malha fechada)
  hw/effects.rs            Modelo das vibrações (motor, embreagem, terreno, suspensão, impactos)
  game.rs                  Servidor do pipe \\.\pipe\PXNV99Telemetry (telemetria do jogo)
  shell.rs                 Bandeja do sistema e vigia da tela
fs25-mod/FS25_PXNV99Telemetry/   Mod do Farming Simulator 25
docs/fase-0-viabilidade.md       Engenharia reversa do V99 e resultados dos testes
```

Regras de acesso ao hardware: o volante é aberto só em modo compartilhado (nunca exclusivo); só são enviados relatórios padrão USB HID PID, por ação do usuário e com limite de taxa; nenhum comando proprietário ou não documentado.

Projeto independente, sem vínculo com a PXN. / Independent project, not affiliated with PXN.
