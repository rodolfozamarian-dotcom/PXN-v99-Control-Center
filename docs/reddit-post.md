# Post para o Reddit

Arquivos para anexar (pasta `release/0.4.1/`):
- `PXN V99 Control Center_0.4.1_x64-setup.exe` (app, 1,5 MB)
- `FS25_PXNV99Telemetry.zip` (mod do FS25, 8 KB)

Antes de postar:
1. Hospede os dois arquivos num lugar com link permanente (GitHub Releases é o mais aceito no Reddit; Google Drive funciona, mas gera mais desconfiança).
2. Tire 3 ou 4 prints do app **com o jogo rodando** (Efeitos de jogo com os medidores mexendo, Monitor ao vivo, Painel). Um GIF ou vídeo curto do volante centralizando vale mais que tudo.
3. Troque os `[LINK]` abaixo pelos links reais.
4. Leia as regras de autopromoção de cada subreddit. Sugestão: r/farmingsimulator (inglês, maior público), e depois comunidades brasileiras de Farming Simulator.
5. Deixe o Patreon de fora do primeiro post. Se houver adesão, ele entra numa atualização ("se quiser apoiar o desenvolvimento…"), o que o Reddit recebe muito melhor.

---

## Versão em inglês (r/farmingsimulator)

**Título (escolha um):**
- I fixed the PXN V99's rock-hard centering spring in DirectInput, and added engine/clutch/terrain vibration for FS25 (free tool + mod)
- PXN V99 owners: adjustable centering in DirectInput + realistic FS25 wheel vibration (free app and mod)

**Texto:**

If you play FS25 with a **PXN V99**, you know the problem: in DirectInput the wheel has a strong centering spring that you can't turn down. The PXN app doesn't let you change it, and the usual "fix" is switching to XInput, which kills the force feedback.

It turns out FS25 doesn't send any force feedback to this wheel at all. What you feel is the wheel's own firmware spring. So I built a small Windows app that takes over the centering, and a FS25 mod that feeds it real data from the game.

**PXN V99 Control Center (app)**
- Adjustable centering strength, 0–100%, live, **without leaving DirectInput**
- Fine-tuning: how firm it is near the center, damping (no oscillation), deadzone
- Runs alongside the game, never locks the wheel; the game keeps full control of the inputs
- Live monitor: wheel angle, pedals, both paddle clutches, H shifter with Low/High and P
- Center check and guided recalibration
- Minimizes to the system tray and keeps working in the background
- English, Portuguese and Spanish

**PXN V99 Telemetry (FS25 mod) + game effects**
- **Engine**: vibrates at the real engine frequency. Strong at idle and when lugging under load, fading as RPM goes up
- **Clutch**: dump the clutch and the RPM drops, the wheel shudders hard. Release it slowly and you just feel a small bump when the gear bites
- **Terrain**: texture depends on the ground under the wheels (asphalt is smooth, fields and soft ground are rough) and grows with speed
- **Suspension**: when one front wheel goes over a bump and the other doesn't, you feel the kick in the wheel
- **Impacts**: potholes and landings
- Every effect has its own slider. Everything goes silent when you pause, open a menu, leave the vehicle or turn the engine off

**How it works (for the curious)**
The app only sends standard USB HID force-feedback commands (the same kind any game uses), no proprietary or undocumented commands. No admin rights, no internet, everything runs locally. The mod only reads the game state and sends it to the app on your PC; it doesn't change anything in your savegame.

**Install**
1. Wheel in DirectInput mode (red light)
2. Install the app: [LINK]
3. Put `FS25_PXNV99Telemetry.zip` in `Documents/My Games/FarmingSimulator2025/mods` and enable it for your savegame: [LINK]
4. In the app: Game effects → enable → Start centering

**Good to know**
- Tested on the PXN V99 on Windows 11. The V10 reports the same USB ID and might work, but I haven't tested it.
- Windows SmartScreen will warn you because the installer isn't code-signed (that costs money). Click "More info" → "Run anyway".
- The mod is singleplayer for now.
- The centering works in any game; the vibration effects are FS25-only for now.

It's free. I made it because this spring was ruining the game for me, and I know a lot of people have the same complaint. Feedback, bug reports and ideas are very welcome, especially from other V99 owners.

---

## Versão em português (comunidades brasileiras)

**Título:**
- Resolvi a mola dura do PXN V99 em DirectInput e coloquei vibração de motor, embreagem e terreno no FS25 (app e mod grátis)

**Texto:**

Quem joga FS25 com o **PXN V99** conhece o problema: em DirectInput o volante tem uma mola de centralização forte que não dá para diminuir. O app da PXN não permite mudar, e a "solução" de sempre é passar para XInput, perdendo o force feedback.

Descobri que o FS25 não manda force feedback nenhum para esse volante: o que você sente é a mola do próprio firmware. Então fiz um app para Windows que assume a centralização e um mod do FS25 que manda os dados reais do jogo para ele.

**PXN V99 Control Center (app)**
- Força de centralização ajustável de 0 a 100%, ao vivo, **sem sair do DirectInput**
- Ajuste fino: firmeza perto do centro, amortecimento (sem ficar oscilando) e zona morta
- Roda junto com o jogo, sem travar o volante; o jogo continua com controle total dos comandos
- Monitor ao vivo: ângulo do volante, pedais, as duas embreagens das borboletas, câmbio H com Low/High e P
- Conferência do centro e recalibração guiada
- Minimiza para a bandeja do sistema e continua funcionando em segundo plano
- Português, inglês e espanhol

**PXN V99 Telemetria (mod do FS25) + efeitos de jogo**
- **Motor**: vibra na frequência real do motor. Forte em marcha lenta e com o motor puxado, sumindo conforme a rotação sobe
- **Embreagem**: soltou de uma vez e a rotação caiu, o volante trepida forte. Soltou devagar, só um toque de que a marcha engatou
- **Terreno**: textura conforme o solo sob as rodas (asfalto liso, lavoura e terra fofa ásperas), crescendo com a velocidade
- **Suspensão**: quando uma roda dianteira passa num buraco e a outra não, você sente o tranco no volante
- **Impactos**: buracos e aterrissagens
- Cada efeito tem seu próprio ajuste. Tudo silencia ao pausar, abrir menu, sair do veículo ou desligar o motor

**Como funciona (para os curiosos)**
O app só envia comandos padrão de force feedback USB HID (o mesmo tipo que qualquer jogo usa), nada proprietário ou não documentado. Não precisa de administrador nem de internet; tudo roda no seu PC. O mod só lê o estado do jogo e manda para o app; não altera nada no seu savegame.

**Instalação**
1. Volante em modo DirectInput (luz vermelha)
2. Instale o app: [LINK]
3. Coloque o `FS25_PXNV99Telemetry.zip` em `Documentos/My Games/FarmingSimulator2025/mods` e ative no savegame: [LINK]
4. No app: Efeitos de jogo → ativar → Iniciar centralização

**Bom saber**
- Testado no PXN V99 com Windows 11. O V10 usa o mesmo ID USB e pode funcionar, mas não testei.
- O Windows SmartScreen vai avisar porque o instalador não tem assinatura digital (que é paga). Clique em "Mais informações" → "Executar assim mesmo".
- O mod é só para singleplayer por enquanto.
- A centralização funciona em qualquer jogo; os efeitos de vibração, por enquanto, só no FS25.

É grátis. Fiz porque essa mola estava estragando o jogo para mim, e sei que muita gente tem a mesma reclamação. Feedback, bugs e ideias são muito bem-vindos, principalmente de quem também tem o V99.
