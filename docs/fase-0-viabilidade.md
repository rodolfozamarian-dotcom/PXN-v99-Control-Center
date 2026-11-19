# PXN V99 Control Center — Fase 0/1: descoberta de hardware e viabilidade

Data: 2026-11-15 · Máquina: Windows 11 Pro 26200 · Volante conectado em DirectInput.
Tudo abaixo marcado **[medido]** foi lido deste volante com `tools/hid-probe.ps1` (somente leitura) ou do registro do Windows. **[fonte externa]** vem de pesquisa. **[hipótese]** ainda precisa de teste físico.

## 1. Identificação do dispositivo [medido]

| Item | Valor |
|---|---|
| Nome USB | `PXN-V99`, fabricante `PXN` |
| VID / PID (DirectInput) | `11FF` / `3245`, bcdDevice `0038` |
| PID em XInput | `11FF` / `F201` ("Controller (PXN-V99)") — visto no histórico do registro; **o PID USB muda com o modo**, então o modo é detectável |
| Tipo | USB composto (`usbccgp`), 2 interfaces HID |
| Interface 0 (`MI_00`) | Joystick HID (Generic Desktop / Joystick) **+ USB HID PID (Force Feedback padrão)** |
| Interface 1 (`MI_01`) | Teclado HID + Mouse HID (usado pelo volante para emular teclas) |
| Drivers | Somente drivers nativos da Microsoft (`HidUsb`, `input.inf`). Nenhum driver PXN instalado |
| Driver de FFB | `pid.dll` da Microsoft, CLSID `{EEC6993A-B3FD-11D2-A916-00C04FB98638}` (classe HID PID genérica) |
| Coleção vendor-defined | **Não existe.** Não há canal HID proprietário visível neste modo |
| Software PXN no PC | Nenhum instalado |

O Linux trata o mesmo ID (`11ff:3245`, compartilhado com o PXN V10) com o driver genérico `hid-universal-pidff` [fonte externa], o que confirma que o FFB é PID padrão e não protocolo proprietário.

## 2. Relatórios HID [medido]

**Entrada, Report ID 1 (19 bytes)** — 8 eixos de 16 bits (X = direção, com sinal −32767..32767; Y, Z, Rx, Ry, Rz, Slider), 1 hat, 24 botões.
**Entrada, Report ID 2** — PID State: efeito tocando, atuadores habilitados, pausa, safety switch, índice do bloco de efeito.

**Saída (todos PID padrão, 18 bytes):**

| RID | Relatório | Uso |
|---|---|---|
| 1 | Set Effect | tipo, duração, ganho do efeito (8 bits), direção |
| 2 | Set Envelope | attack/fade |
| 3 | **Set Condition** | offset, coeficientes ±, saturação ±, deadband — é aqui que Spring/Damper/Friction/Inertia são definidos |
| 4 | Set Periodic | magnitude, offset, fase, período |
| 5 | Set Constant Force | magnitude |
| 6 | Set Ramp Force | início/fim |
| 7, 8, 14 | Custom force | dados de força customizada |
| 10 | Effect Operation | start / start solo / stop, loop |
| 11 | Block Free | libera efeito |
| 12 | Device Control | enable/disable actuators, stop all, reset, pause, continue |
| 13 | **Device Gain** | ganho global 0..255 |

**Feature:** RID 5 Create New Effect, RID 6 Block Load, RID 7 PID Pool. Leitura do PID Pool: `RAM pool = 600`, `efeitos simultâneos = 10`, pool gerenciado pelo dispositivo, até 14 blocos de efeito.

**Efeitos registrados no DirectInput:** Constant, Ramp, Square, Sine, Triangle, Sawtooth Up/Down, Spring, Damper, Inertia, Friction, Custom.

Pendente: a amostragem de entrada retornou 0 relatórios em 1,5 s com o volante parado. Provável envio apenas quando há mudança; frequência real só será medida com o volante em movimento.

## 3. De onde vem o Spring Force

- **Efeito dinâmico do jogo** [medido + fonte externa]: `FarmingSimulator2025Game.exe` importa `DINPUT8.dll` / `DirectInput8Create`. Guias da comunidade indicam que o FS25 usa **apenas o efeito Spring** como force feedback (damper não é usado). Caminho: jogo → DirectInput → `pid.dll` → Output Reports 1/3/10 → firmware.
- **Controle do próprio jogo** [medido]: `gameSettings.xml` tem `<forceFeedback>0.500000</forceFeedback>`. É o primeiro ganho da cadeia.
- **Mola de firmware** [hipótese]: se o volante centraliza sozinho na área de trabalho, sem nenhum jogo aberto, existe também uma mola interna do firmware, que nenhum relatório PID documentado controla. Precisa de teste físico.

## 4. Coexistência com o jogo

- DirectInput exige modo **exclusivo** para Force Feedback; só um aplicativo o detém. Se o Control Center adquirisse o volante em exclusivo, tiraria o FFB do jogo. **Portanto o app nunca usará aquisição exclusiva enquanto um jogo estiver rodando.**
- A coleção HID do joystick abre em modo compartilhado com leitura **e escrita** [medido]. Isso permite ler eixos/botões e enviar Output Reports PID diretamente via `hid.dll`, em paralelo ao jogo, sem tocar no DirectInput dele.
- O Test Lab (que cria efeitos próprios) só funcionará com o jogo fechado.

## 5. Estratégias para reduzir o Spring em DirectInput

| # | Mecanismo | Alcance | Risco / limitação | Confiança |
|---|---|---|---|---|
| A | **Device Gain (RID 13) via HID compartilhado** | Ganho global, ao vivo, sem reiniciar o jogo. No FS25 global ≈ spring, já que o jogo só usa spring | Reduz todos os efeitos, não só spring. O jogo pode reenviar o ganho ao readquirir o volante (alt-tab) → o app reaplica. Depende de o firmware honrar o relatório | Média — **precisa de teste físico** |
| B | **Proxy `dinput8.dll` na pasta do jogo** (opcional, por jogo) | Escala por tipo de efeito (Spring, Damper, Constant…) dentro de `CreateEffect`/`SetParameters`, lendo os ganhos do app em memória compartilhada → ajuste ao vivo e realmente independente | É um arquivo local carregado pelo próprio jogo, não injeção em processo. Precisa ser instalado/removido por jogo; não usar em jogos com anti-cheat | Alta (técnica consolidada), a validar no FS25 |
| C | Reescrever Set Condition (RID 3) do bloco do jogo | Spring apenas | Índice do bloco é adivinhado; o jogo sobrescreve a cada atualização | Baixa — descartada |
| D | Dispositivo virtual (vJoy) com repasse de FFB | Controle total | Driver de terceiros, muda o ID do dispositivo no jogo (refazer bindings), latência | Média — só como último recurso |
| E | Ajustar `forceFeedback` do próprio jogo via perfil | Spring no FS25 | Só com o jogo fechado; não é ao vivo | Alta |
| F | Filtro de driver HID em kernel | Controle total | Exige driver assinado; desproporcional | Descartada |

**Plano:** A primeiro (zero instalação, ao vivo). Se o firmware ignorar o Device Gain ou se for necessário separar spring dos demais efeitos em outros jogos, B. E entra nos perfis.

## 6. Matriz de viabilidade

| Funcionalidade | Mecanismo | Status | Limitações | Confiança | Driver/serviço extra |
|---|---|---|---|---|---|
| Detecção automática e reconexão | Enumeração HID por VID/PID + notificação de dispositivo | **Comprovado** | — | Alta | Não |
| Identificar modo DInput/XInput | PID USB `3245` vs `F201` | **Comprovado** (IDs observados) | — | Alta | Não |
| Leitura de eixos, pedais, botões, hat | Input Report 1, handle compartilhado | **Comprovado** (descritor) | Mapeamento eixo→pedal a confirmar movendo cada pedal | Alta | Não |
| HUD ≥ 60 Hz | Leitura assíncrona orientada a evento | Parcial | Taxa real ainda não medida | Média | Não |
| Estado do FFB (tocando, atuadores) | Input Report 2 | **Comprovado** (descritor) | — | Alta | Não |
| Overall Force ao vivo com jogo aberto | Device Gain RID 13 | Parcial | Aguardando teste físico | Média | Não |
| Spring independente ao vivo | Proxy `dinput8.dll` | Parcial | Por jogo; sem anti-cheat | Alta | Não (DLL local) |
| Damper/Constant/Periodic/Friction/Inertia independentes | Proxy `dinput8.dll` | Parcial | Idem | Alta | Não |
| Enable/Disable FFB, Stop All | Device Control RID 12 | Parcial | Aguardando teste físico | Média-alta | Não |
| Test Lab (spring, damper, constant, periódico, friction) | DirectInput exclusivo do próprio app | Parcial | Só com o jogo fechado | Alta | Não |
| Centering Strength (mola do firmware) | Desconhecido | **Não comprovado** | Sem relatório documentado | Baixa | — |
| Steering Angle 270°/900° no hardware | Desconhecido (botões do volante / app oficial) | **Não comprovado** | Sem canal HID proprietário visível | Baixa | — |
| Sensibilidade / deadzone / linearidade no hardware | — | **Não comprovado** | Idem | Baixa | — |
| Curvas de pedal / deadzone por software | Processamento no app | Parcial | Só afeta o jogo com dispositivo virtual; sem ele serve apenas para visualização | Média | vJoy para ter efeito no jogo |
| Button mapping no hardware | — | **Não comprovado** | — | Baixa | — |
| Versão de firmware | `bcdDevice` = 0.38 | **Comprovado** (apenas revisão USB) | Pode não ser a versão "comercial" | Média | Não |
| Calibração lógica de centro, Steering Test, gráficos | Dados do Input Report 1 | **Comprovado** | Calibração física do motor não acessível | Alta | Não |
| Perfis, auto-carregamento por processo | Arquivos locais + lista de processos | **Comprovado** | — | Alta | Não |
| Atualização de firmware | — | **Fora de escopo** | Sem procedimento oficial | — | — |

Os controles das linhas "Não comprovado" ficarão **desabilitados com a explicação**, não simulados.

## 7. Arquitetura proposta

```
┌────────────── UI: React + TypeScript + Tailwind ──────────────┐
│ Dashboard · FFB · Live Monitor · Calibration · Test Lab · ... │
└───────────────▲──────────────────────────┬────────────────────┘
        eventos (telemetria)        comandos validados (IPC Tauri)
┌───────────────┴──────────────────────────▼────────────────────┐
│ Núcleo Rust (processo do app, sem privilégio de administrador)│
│  device   descoberta, hotplug, modo DInput/XInput             │
│  input    thread de leitura HID → decodificação → telemetria  │
│  ffb      Device Gain / Device Control, fila com rate-limit   │
│  safety   limites, watchdog, parada imediata, restauração     │
│  testlab  DirectInput exclusivo, só sem jogo aberto           │
│  profiles armazenamento local, detecção de processo           │
│  diag     log de eventos, relatório técnico                   │
└───────┬───────────────────────────────────────────┬───────────┘
        │ hid.dll (compartilhado)                   │ memória compartilhada (ganhos)
   PXN V99 (MI_00)                      pxn_dinput8 (proxy opcional na pasta do jogo)
                                                    │
                                        jogo → DirectInput → pid.dll → V99
```

- **Tauri 2 + Rust**: um único idioma cobre o núcleo, o acesso HID/DirectInput (crate `windows`) e a DLL proxy. Requer instalar o toolchain Rust (não está na máquina; Node 24, .NET 10, VS 2026 e WebView2 já estão).
- Alternativa sem instalar nada: host .NET 10 + WebView2, com a DLL proxy em C++.
- A UI nunca fala com o hardware; só envia comandos tipados que o núcleo valida (faixa, taxa, estado de segurança).
- Threads separadas para leitura, escrita e UI; escrita com fila e limite de taxa.

## 8. Próximos passos

1. Fase 2: scaffold, leitura real, Live Monitor.
2. Teste físico A (com você no volante): Device Gain em 100 % → 50 % → 20 % com o FS25 aberto.
3. Conforme o resultado: consolidar A ou implementar o proxy B.

## 9. Resultados dos testes físicos (16/11/2026)

Testes feitos no volante, com o app aberto em DirectInput.

| Teste | Resultado |
|---|---|
| Volante sem nenhum jogo aberto | Volta ao centro com força: a mola é **do firmware**, ativa sempre que nenhum programa usa o Force Feedback |
| Farming Simulator 25 | A opção de Force Feedback do jogo não fica editável; o jogo **não envia efeitos** a este volante. A mola sentida no jogo é a do firmware |
| Device Gain (RID 13) de 7 % a 100 % | Aceito sem erro, **sem efeito** na mola do firmware (ele só escala efeitos PID) |
| `HidP_InitializeReportForID` | Devolve `REPORT_DOES_NOT_EXIST` para IDs válidos neste descritor; os relatórios são montados sem ela |
| Enable Actuators + criação de efeito | **Desliga a mola do firmware** (volante fica solto) |
| Efeito Sine (vibração 25 %, 1 s) | **Funciona**: o firmware executa efeitos PID enviados direto pelo HID compartilhado |
| Efeito Spring com direção 0° | Sem torque |
| 8 efeitos Spring empilhados, direção 90° | Puxa para o centro, mas fraco e sem chegar ao centro: a mola PID é proporcional ao ângulo sobre o curso de 900° |
| Força constante em malha fechada pelo app | **Centraliza até o centro**, com força ajustável ao vivo |

### Solução adotada para o Centering Strength

`src-tauri/src/hw/centering.rs`: o app lê a posição do volante e comanda um efeito PID Constant Force (Set Effect com direção de 90°, gatilho `0xFF`, duração de 30 s renovada a cada 12 s; magnitude atualizada pelo relatório Set Constant Force). Parâmetros: força máxima, ângulo de força máxima, amortecimento e zona morta. O sentido do motor é calibrado na primeira vez com dois pulsos fracos (neste volante: polaridade −1).

Consequências para a matriz da seção 6:

- **Centering Strength**: passa de "não comprovado" para **comprovado**, via malha do app (não é um parâmetro do firmware).
- **Overall Force por Device Gain**: comprovado apenas para efeitos PID; não atua na mola do firmware.
- **Spring enviado por jogos / proxy `dinput8.dll`**: irrelevante para o FS25, que não envia efeitos. Continua válido para jogos que usam Force Feedback.

### Pendências

- Validar a malha com o Farming Simulator 25 em execução (coexistência com o jogo).
- O volante não envia o relatório PID State (RID 2) nos testes feitos; o estado de atuadores continua "Não disponível".
- Se um jogo com Force Feedback próprio adquirir o volante, os efeitos dele e a malha do app se somam; ainda não testado.
