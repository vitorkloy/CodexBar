# CodexBar para Windows (tray app)

App de bandeja (system tray) para Windows que mostra o uso/limites dos seus
assistentes de IA, com contagem regressiva até o reset de cada janela.

> **Créditos:** CodexBar é criado e mantido por **Peter Steinberger
> ([@steipete](https://github.com/steipete))** — https://github.com/steipete/CodexBar.
> Este diretório é um port para Windows mantido no fork
> [vitorkloy/CodexBar](https://github.com/vitorkloy/CodexBar). A lógica dos
> provedores (endpoints, headers, formato de credenciais e do `config.json`)
> foi portada diretamente do `CodexBarCore` em Swift. Licença MIT — veja
> [`LICENSE`](../../LICENSE) na raiz.

## Recursos (v1)

- Ícone na bandeja com barra de uso (verde/âmbar/vermelho) e tooltip por provedor.
- Painel com uso por janela (5h, semanal, mensal…) e contagem até o reset.
- Notificação ao se aproximar do limite (padrão 80% / 95%, uma vez por janela).
- Configurações: intervalo de atualização, iniciar com o Windows, notificações,
  provedores ativos, token do Copilot (device flow), cookie manual do Cursor.
- Instalador `.exe` (NSIS, por usuário, sem admin) que não exige nada extra:
  runtime C estático e bootstrapper do WebView2 embutido.

## Provedores (v1)

| Provedor | Fonte de credencial no Windows |
|---|---|
| Codex | `%USERPROFILE%\.codex\auth.json` (ou `CODEX_HOME`) — `codex login` |
| Claude | `%USERPROFILE%\.claude\.credentials.json` (ou `CLAUDE_CONFIG_DIR`) — `claude` login OAuth |
| Cursor | `%APPDATA%\Cursor\User\globalStorage\state.vscdb` (login do app Cursor) ou cookie manual |
| Gemini | `%USERPROFILE%\.gemini\oauth_creds.json` — `gemini` login com Google |
| Copilot | token no `config.json`, `COPILOT_GITHUB_TOKEN`, `%LOCALAPPDATA%\github-copilot\*.json` ou `gh auth token` |

Tokens do Codex/Claude nunca são renovados nem reescritos (os refresh tokens
rotacionam e isso deslogaria a CLI). O token do Gemini é renovado só em memória.

Leitura de cookies do navegador fica para a v2.

## Configuração

Usa o mesmo `config.json` do CodexBar (`CODEXBAR_CONFIG` →
`%XDG_CONFIG_HOME%\codexbar\config.json` → `%USERPROFILE%\.config\codexbar\config.json`
→ legado `%USERPROFILE%\.codexbar\config.json`). Campos desconhecidos são
preservados. Preferências só do Windows ficam em
`%APPDATA%\CodexBar\windows-settings.json`.

## Leveza

- Uma única thread de polling dormindo numa `Condvar` (padrão 5 min, ou manual).
- Nenhuma WebView fica viva em repouso: o painel é criado ao clicar no ícone e
  destruído ao perder o foco (os processos `msedgewebview2` encerram).
- Menu e tooltip da bandeja são nativos.

## Por que Tauri/Rust e não o Swift CLI + app Qt?

O plano inicial era compilar o CLI Swift no Windows e reaproveitar o app Qt6 do
Linux. Avaliação rápida no código atual:

- `CodexBarCore` tem ~830 arquivos / ~173 mil linhas e depende de APIs POSIX sem
  equivalente no Windows (`forkpty`, `termios`, `posix_spawn`, `waitpid`, `flock`
  — ~100 pontos de chamada em `TTYCommandRunner`, sessões de CLI, lock de config),
  o que exigiria reescrever a camada de PTY sobre ConPTY.
- ~50 `import Darwin`, ~40 `Glibc`/`Musl`, ~100 `FoundationNetworking`, `Security`
  e `SQLite3` (módulo `CSQLite3` só para Linux); dependências como SweetCookieKit e
  swift-crypto com Swift 6.2 strict concurrency não são testadas no Windows.
- O app Qt exigiria empacotar runtime do Qt + runtime Swift/ICU (~100+ MB).

Conclusão: inviável para uma v1. Fallback aprovado: app Tauri 2 + Rust
implementando só os 5 provedores, com o mesmo formato de config. Tudo fica
isolado em `Integrations/Windows/` (+ um workflow), para facilitar sincronizar
com o upstream.

## Build local

Requisitos: Rust (MSVC), Node 18+, VS Build Tools (C++) + Windows SDK.

```powershell
cd Integrations\Windows
npm install
npx tauri build --bundles nsis
# instalador: src-tauri\target\release\bundle\nsis\CodexBar_*_x64-setup.exe
```

CI: `.github/workflows/windows-tray.yml` gera o instalador como artifact.
