<h1 align="center">HIDra</h1>

<p align="center">
  Configure mouses e teclados gamer no <b>Linux, Windows e macOS</b> — falando
  direto com o hardware por HID, sem depender do programa do fabricante.
  <br>
  Compatível com o mouse <b>Redragon Cobra Pro (M711-PRO)</b> e o teclado
  <b>Redragon Brahma Pro (K586RGB-PRO)</b>, cujos programas só existem para
  Windows.
</p>

<p align="center">
  <b><a href="https://luizjr.github.io/HIDra/">luizjr.github.io/HIDra</a></b> ·
  <a href="README.md">Read in English 🇬🇧</a>
</p>

---

Os fabricantes só publicam o programa de configuração para Windows, um programa
por produto, e param de atualizar. O HIDra é um app só, que fala o protocolo USB
dos próprios aparelhos — sua iluminação, DPI e remapeamento continuam
funcionando independente do sistema que você usa.

> Não é afiliado, endossado nem patrocinado por nenhum fabricante. Os nomes de
> produtos aparecem apenas para dizer qual hardware é compatível.

## Telas

| Mouse | Teclado |
| --- | --- |
| ![Iluminação, DPI e botões do mouse](docs/screenshots/mouse.png) | ![Efeitos de iluminação do teclado](docs/screenshots/keyboard.png) |

Cores por tecla, pintadas uma a uma e gravadas no próprio teclado:

![Editor de cores por tecla](docs/screenshots/perkey.png)

## Dispositivos compatíveis

<!-- devices:start -->
| Dispositivo | ID USB | Situação | Recursos |
| --- | --- | --- | --- |
| Redragon Cobra Pro (M711-PRO) — receptor 2.4 GHz | `25A7:FA07` | ✅ validado | iluminação, DPI, botões, polling |
| Redragon Cobra Pro (M711-PRO) — cabo | `25A7:FA08` | ✅ validado | iluminação, DPI, botões, polling |
| Redragon Brahma Pro (K586RGB-PRO) | `0C45:5004` | ✅ validado | 18 efeitos + cor por tecla, 3 perfis, remapeamento |
| Redragon UCAL K673SP-RGB-PRO — cabo | `258A:010C` | ✅ validado | iluminação via cabo, 18 efeitos + cor por tecla (modelo verificado); sem remapeamento/perfis |
| Redragon UCAL K673SP-RGB-PRO — receptor 2.4 GHz | `3554:FA09` | 🟡 relatado | iluminação 2.4 GHz + Smart Notifications (modelo verificado); cores por tecla salvas exigem USB |
<!-- devices:end -->

**Seu aparelho não está aí?** O HIDra não é um projeto Redragon — ele começou
com esses dois porque eram os que estavam na mesa do autor. Qualquer marca cabe
na tabela, e muitos periféricos são o mesmo hardware com outro nome, falando um
protocolo que o HIDra já conhece; adicionar costuma ser **uma entrada** em
[`src-tauri/src/devices.rs`](src-tauri/src/devices.rs):

```sh
python3 scripts/probe-device.py     # acha o canal de config e imprime a entrada
python3 scripts/sync-devices.py     # regera regra udev, tabelas e site
```

O passo a passo completo está em
[docs/ADDING-DEVICES.pt-BR.md](docs/ADDING-DEVICES.pt-BR.md). Se preferir não
mexer em código, abra um
[pedido de dispositivo](https://github.com/luizjr/HIDra/issues/new?template=device-request.yml)
com a saída de `python3 scripts/probe-device.py --issue`.

## Smart Notifications (Linux)

No UCAL K673SP-RGB-PRO conectado por cabo, abra **Smart Notifications** ao lado
de **Cores por tecla**. Adicione uma regra, selecione teclas no teclado, escolha
cor, duração e piscar opcional e salve. Combine aplicativo (por exemplo, Viber), texto do
título/descrição ou ambos. Deixe Aplicativo vazio para reconhecer o texto de
notificações de sites em qualquer navegador. O aplicativo ou navegador precisa
enviar uma notificação real para a área de trabalho. Selecione **Origem da
notificação → Gmail (Navegador)** para reconhecer automaticamente `mail.google.com`
no título ou na descrição, em qualquer navegador, sem filtrar pelo remetente.
Para regras personalizadas, **Usar descrição** copia o terceiro campo de uma
notificação recente.

Ative o interruptor para entregar o controle da iluminação às notificações. Os
efeitos normais não funcionam simultaneamente. Escolha uma **Cor de fundo** e
salve com as regras para manter as outras teclas iluminadas. Desativar ou fechar o HIDra
restaura o modo de iluminação capturado. O estado do interruptor e as regras são
preservados entre execuções. O HIDra precisa permanecer aberto. O conteúdo das
notificações recentes fica apenas na memória. **Testar regra** verifica as teclas
e a cor atribuídas.

Novas regras usam **Até a notificação ser dispensada** por padrão: a remoção da
notificação da área de trabalho, não a leitura dentro do aplicativo. Outras opções
mantêm o alerta por **1, 5 ou 10 minutos**. As teclas podem piscar entre a cor do
alerta e a cor de fundo. Cada notificação é acompanhada separadamente, inclusive
atualizações; dispensar uma não apaga outra na mesma tecla. **Limpar alertas**
limpa apenas os alertas do teclado. **Testar regra** dura dez segundos.

Notificações FDO usam o ID retornado pelo servidor e o sinal `NotificationClosed`.
No Ubuntu/GNOME o alerta continua após o banner sumir se a notificação permanece
na central. Aplicativos GTK são identificados por aplicativo e ID; a remoção pelo
aplicativo é observada, mas GTK não fornece um sinal padrão de dispensa pelo
usuário. Use **Limpar alertas** ou uma duração fixa quando o aplicativo não informar
a remoção. Apenas notificações recebidas enquanto o modo está ativo são acompanhadas;
o histórico anterior não é importado. Regras antigas com duração em segundos
migram para a nova opção padrão de dispensa.

Scripts personalizados são outra **Origem da notificação**. Informe o caminho
absoluto de um script executável (com shebang) e um intervalo de 5–86400 segundos.
Com o modo ativo, o script executa imediatamente e aguarda o intervalo após cada
execução. Código de saída **1** ativa as teclas; **0** limpa o alerta desse script.
A duração padrão é **Até o script retornar 0**. Outros códigos, erros e o limite
de 10 segundos mostram o erro e preservam o último estado. Execuções da mesma
regra não se sobrepõem, não abrem terminal e descartam stdout/stderr. Desativar,
alterar regras ou fechar o HIDra interrompe os scripts. Configure argumentos e
ambiente no próprio script. **Testar regra** apenas demonstra a iluminação.

A paleta interna **Cor personalizada** oferece seleção de saturação/brilho,
controles de matiz, saturação e brilho, canais RGB e entrada HEX.

O HIDra também tem um ícone na barra superior/bandeja, com **Mostrar HIDra** e
**Sair do HIDra**. Quando o Smart Notifications está ativo, fechar a janela oferece
**Manter na barra superior**, **Sair do HIDra** ou **Cancelar**. Manter na barra
superior oculta a janela enquanto notificações e scripts continuam funcionando;
use o menu do ícone para reabrir a mesma janela. Sair interrompe o monitoramento
e os scripts e restaura a iluminação anterior. Com o modo desativado, fechar
encerra normalmente. Se a área de trabalho não tiver uma bandeja ativa, a opção
de manter no fundo fica indisponível para não ocultar a janela sem acesso.
GNOME exige uma extensão AppIndicator ativa (o Ubuntu inclui Ubuntu AppIndicators).

O monitor usa o D-Bus da sessão, sem root nem substituir o serviço de notificações.
Se a sessão bloquear o monitoramento, o painel apresenta um erro. Quadros RGB
temporários usam uma conexão HID aberta; o quadro de fundo ocioso exige atualização
periódica, sem redescoberta nem leitura repetida da configuração. Outros dispositivos
e plataformas ainda não oferecem este modo.

O UCAL K673SP-RGB-PRO também possui um receptor **2,4 GHz** separado (`3554:FA09`).
O HIDra verifica a identidade completa e usa o protocolo próprio de relatórios de
saída; apenas adicionar esse ID ao transporte com cabo não funciona. O Smart
Notifications transmite quadros completos por RF, com atualização periódica do
fundo. A edição
de cores por tecla salvas ainda exige o cabo. A recuperação de iluminação por
cabo e por receptor não pode usar o outro transporte. Desative o Smart Notifications
antes de alternar entre cabo e RF. Pressione uma tecla para despertar o teclado
e use Atualizar dispositivos se necessário.
O suporte RF permanece experimental: apagamentos simultâneos e intermitentes
dos LEDs foram relatados e continuam em investigação.

### Configurações e inicialização automática

O botão **Configurações** na lateral contém Idioma e **Iniciar o HIDra automaticamente
ao entrar na sessão**. No Linux, ativar registra o executável no diretório XDG
autostart do usuário com `--tray`; desativar remove a entrada. Vale para o próximo
login e não exige root nem serviço. O Smart Notifications preserva sua própria
preferência e só é retomado se estava ativo. AppImages registram o arquivo original,
não a montagem temporária. Mantenha o executável no caminho registrado ou
desative/ative novamente após movê-lo.

`hidra --tray` (ou `--tray-mode`) inicia sem mostrar a janela principal; sem esse
argumento, a janela aparece normalmente. Não há janela piscando antes de ocultar.
Se a bandeja ainda não estiver disponível no login, o HIDra aguarda brevemente e
mostra a janela como alternativa. Use **Mostrar HIDra** no ícone para reabri-la.
A inicialização automática fica indisponível em versões com servidor de
desenvolvimento; use uma versão independente que não dependa do Vite.

## Instalação

Baixe o instalador do seu sistema na
[última release](https://github.com/luizjr/HIDra/releases/latest).

### Linux — `.deb`, `.rpm` ou AppImage

```sh
sudo dpkg -i HIDra_*_amd64.deb        # Debian / Ubuntu / Mint / Pop!_OS
sudo rpm -i HIDra-*.x86_64.rpm        # Fedora / openSUSE
chmod +x HIDra_*.AppImage && ./HIDra_*.AppImage
```

O executável se chama `hidra`, então o app também abre pelo terminal com esse
nome.

O `.deb` e o `.rpm` já instalam a regra udev — basta **reconectar o aparelho**
(ou o dongle) depois de instalar. Com o AppImage, ou rodando do código-fonte,
instale a regra uma vez:

```sh
sudo sh packaging/install-udev.sh          # --uninstall reverte
```

A regra usa `uaccess`, que entrega os nós `hidraw` a quem está logado na sessão
local — sem root, sem setuid e sem nada rodando em segundo plano.

### Windows — `.exe`

Rode o instalador `.exe`. Ele não é assinado digitalmente, então o SmartScreen
mostra "O Windows protegeu o seu PC": clique em **Mais informações → Executar
assim mesmo**. Não precisa de driver.

### macOS — `.dmg`

Abra o `.dmg` e arraste o HIDra para Aplicativos. Como não é notarizado, a
primeira abertura precisa de **clique direito → Abrir**, ou:

```sh
xattr -dr com.apple.quarantine /Applications/HIDra.app
```

> As versões de Windows e macOS saem do CI, mas foram muito menos testadas que a
> de Linux. Se algo não funcionar,
> [abra uma issue](https://github.com/luizjr/HIDra/issues/new/choose).

### Versões de desenvolvimento

Todo push na `main` recompila os três instaladores e substitui a
[pré-release `nightly`](https://github.com/luizjr/HIDra/releases/tag/nightly).
Use-a para testar uma correção antes de virar versão; fora isso, use a
[última release](https://github.com/luizjr/HIDra/releases/latest).

## Mantendo atualizado

O HIDra procura uma nova versão alguns segundos depois de abrir, e sempre que
você clica em **Procurar atualizações** na barra lateral. Quando há uma, aparece
uma faixa acima dos painéis oferecendo a instalação.

Como ele instala depende de como você instalou:

| Instalação | O que acontece |
| --- | --- |
| Windows, macOS, AppImage no Linux | O HIDra baixa a nova versão, confere a assinatura e reinicia já nela |
| `.deb` / `.rpm` no Linux | O HIDra baixa o pacote assinado e entrega ao `dpkg`/`rpm` via `pkexec`, então o sistema pede autorização como pediria para qualquer pacote |
| Compilado do código, ou empacotado por terceiros | O HIDra avisa que saiu uma versão e leva até ela; nunca mexe numa instalação que não foi ele que fez |

Todo artefato é assinado com a chave de release do projeto, e o app confere o
download com ela antes de instalar qualquer coisa — um arquivo adulterado é
recusado, não instalado. Versões nightly nunca são oferecidas como atualização.

A verificação é uma requisição HTTPS ao GitHub pedindo os dados da release. Não
há telemetria, não há conta, e nada sobre você ou seu hardware é enviado.


## O que ele faz

**Mouses (família Cobra Pro)**
- Iluminação: sólido, respiração, ciclo, desligado — cor, brilho e velocidade
- DPI: 5 estágios com X/Y independentes, cor por estágio, indicador de estágio
- Botões: remapear os 8 — clique do mouse, DPI ±/ciclo, tecla, combo (Ctrl+C),
  multimídia, disparo (fire), liga/desliga RGB, trocar polling
- Taxa de polling: 125 / 250 / 500 / 1000 Hz
- Leitura do que está realmente gravado no aparelho

**Teclados (família Brahma Pro)**
- 18 efeitos de iluminação + **cor por tecla** (Customizável)
- Cor, brilho (0–5), velocidade (1–5), direção e modo multicolor (Full RGB)
- 3 perfis independentes gravados no teclado, trocados pelo app
- Remapear teclas: tecla única, modificador, multimídia, desativar — e restaurar
  o mapa de fábrica

Tudo fica gravado no próprio aparelho: continua valendo depois de fechar o app,
reiniciar ou levar o periférico para outra máquina.

## Compilando

Requisitos: [Rust](https://rustup.rs) ≥ 1.77, Node.js ≥ 18 e as dependências do
Tauri ([lista oficial](https://tauri.app/start/prerequisites/)). No
Debian/Ubuntu:

```sh
sudo apt install libwebkit2gtk-4.1-dev libudev-dev build-essential curl file \
                 libxdo-dev libssl-dev libayatana-appindicator3-dev librsvg2-dev
```

```sh
npm install
npm run tauri dev      # abre o app em modo desenvolvimento
npm run tauri build    # instaladores em src-tauri/target/release/bundle
```

## Problemas comuns

**Linux: retângulos pretos por cima da janela.** O WebKitGTK coloca elementos
com filtro numa camada composta separada e, em algumas configurações de GPU
(híbrido Intel + NVIDIA no Wayland, visto no WebKitGTK 2.52), desenha essa
camada no lugar errado. Por isso o HIDra não usa nenhum filtro CSS ou SVG. Se
mesmo assim acontecer — aqui ou em outro app Tauri — a saída bruta é
`WEBKIT_DISABLE_COMPOSITING_MODE=1 hidra`, que desliga a composição por GPU.

**Um ajuste não "pega" no mouse sem fio.** O rádio 2.4 GHz dorme quando o mouse
está parado. Mexa no mouse enquanto clica em *Aplicar*, ou use o cabo.

**O aparelho aparece como desconectado no Linux.** Falta a regra udev, ou o
aparelho não foi reconectado depois de instalá-la: `sudo sh
packaging/install-udev.sh` a partir do código-fonte (o `.deb`/`.rpm` já trazem a
regra), e então desconecte e reconecte.


## Como funciona

O app é [Tauri](https://tauri.app): backend em Rust que faz toda a conversa com
o hardware e interface em React/TypeScript. Não há serviço em segundo plano nem
módulo de kernel — o backend abre a coleção HID do fabricante com
[`hidapi`](https://github.com/libusb/hidapi) e escreve os mesmos pacotes que o
programa oficial escreve.

```
src/                    interface React (um painel por tipo de aparelho)
src-tauri/src/
  devices.rs            tabela de dispositivos compatíveis  ← adicione o seu aqui
  hid.rs                descoberta + transporte, escolha de endpoint por sistema
  mouse.rs              protocolo Compx de 17 bytes
  keyboard.rs           protocolo SONiX de 64 bytes
PROTOCOL.md             notas byte a byte dos dois protocolos
```

O protocolo do teclado foi obtido capturando o tráfego USB do programa oficial
do Windows; cada byte está anotado em [`PROTOCOL.md`](PROTOCOL.md) para que
ninguém precise refazer esse trabalho.

A interface inicia em **inglês**. Selecione **English** ou **Português (Brasil)**
em **Language / Idioma**, no rodapé da barra lateral. A preferência é salva
localmente. A troca de idioma mantém as edições e não consulta nem grava no
hardware. Para adicionar um idioma, veja [docs/TRANSLATING.md](docs/TRANSLATING.md).

## Contribuindo

Relatos de bug, novos dispositivos, capturas de protocolo e melhorias de
interface são todos bem-vindos — veja [CONTRIBUTING.md](CONTRIBUTING.md). A
contribuição mais valiosa é um dispositivo novo:
[docs/ADDING-DEVICES.pt-BR.md](docs/ADDING-DEVICES.pt-BR.md) mostra o caminho.

Usa um assistente de IA? O [`CLAUDE.md`](CLAUDE.md) documenta a arquitetura e as
regras do projeto para agentes (o [`AGENTS.md`](AGENTS.md) aponta as outras
ferramentas para lá), e quem usa Claude Code tem o comando `/add-device`, que
percorre o fluxo inteiro. O teste no hardware continua tendo de vir de uma
pessoa que tem o aparelho.

## Créditos

- Protocolo do mouse: [`open-m711pro`](https://github.com/mateusands/open-m711pro),
  [`m913-ctl`](https://github.com/Qehbr/m913-ctl) e
  [`mouse_m908`](https://github.com/dokutan/mouse_m908) — todos GPL.
- Protocolo do teclado: capturado do programa oficial, com referência ao
  [`rgb_keyboard`](https://github.com/dokutan/rgb_keyboard) e à comunidade
  OpenRGB.

## Licença

[GPL-3.0-or-later](LICENSE). Use por sua conta e risco: todo painel tem um botão
**Restaurar padrão**, e nada aqui grava firmware.

### Calibração de cores do teclado

A aba **Configurações** do teclado oferece balanço de branco RGB opcional para
modelos compatíveis. Ative-o e informe os valores RGB que parecem brancos no
teclado; o branco lógico (255/255/255) é convertido nesses valores e as demais
cores escolhidas são ajustadas proporcionalmente. O K673SP oferece 255/153/161
como sugestão do proprietário, com a correção desativada por padrão. USB e RF
compartilham a preferência do modelo. O Smart Notifications atualiza no próximo
quadro; reaplique as cores manuais após alterar a calibração. Cores de efeitos
de arco-íris internos não são corrigidas.

### Monitoramento do sistema no Smart Notifications

Ative CPU, RAM e/ou **Espaço em disco (/)** e escolha uma cor para cada medidor.
O K673SP usa uma tecla móvel nas linhas Q–P (CPU), A–; (RAM) e Z–/ (disco).
A primeira tecla representa 5–15%, a segunda 16–25%, até 96–100% na última;
abaixo de 5% a linha mantém o fundo. Acima de 90%, a indicação pisca.
O disco indica espaço usado no sistema de arquivos raiz. As leituras Linux
atualizam aproximadamente a cada segundo enquanto o Smart está ligado.
Notificações têm prioridade nas teclas em comum e a calibração RGB também se
aplica às cores do monitoramento.

### Captura de falhas intermitentes na iluminação RF (Linux)

Execute `sudo python3 scripts/capture-rf-lighting.py --vid 3554 --pid fa09 --interface 1`
no repositório. Mantenha o HIDra funcionando normalmente e pressione Ctrl+C assim
que a falha aparecer. O script informa o arquivo `/tmp/hidra-rf-*.jsonl` com os
últimos 120 segundos e um marcador de parada. Ele registra passivamente o relatório
0x13, conclusões/erros de transferência e desconexões/reconexões do receptor; não
envia comandos HID nem salva teclas digitadas. A captura observa o canal USB até
o receptor, não o enlace de rádio. O arquivo pertence ao usuário que o executou.
