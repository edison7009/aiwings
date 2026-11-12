<p align="center"><img src="branding/wings-banner.png" alt="Wings" width="420"></p>

# Wings

<p align="center"><a href="https://aiwings.app">aiwings.app</a></p>

<p align="center"><a href="README.md">简体中文</a> | English</p>

## Description

Queue your next prompt while the AI is busy, then paste it back in one click. Wings is a sidebar that docks beside Claude, ChatGPT and other desktop AI apps, on macOS and Windows.

## How to use

- **Save**: write in the input box, then click "Save to Wings" above it, or press **Alt+S** (⌥S on macOS). The text (or an image) goes into Wings and the input box is cleared.
- **Send**: click an item in Wings to paste it back into the input box. It is never sent automatically, and it stays on the clipboard so you can paste it elsewhere too.
- **Follow**: switch to another AI app and Wings follows it. You can also keep it with one app, or let it hover anywhere.
- **Terminals**: Windows Terminal, PowerShell 5/7, and cmd support following and pasting notes. Wings never presses Enter to run commands and does not support saving or clearing terminal input. If you have a custom follow list, tick the terminal in settings.

## Supported apps

| Kind | Dock and follow | One-click save | Paste back |
|---|:-:|:-:|:-:|
| AI apps | ✓ | ✓ | ✓ |
| Terminals | ✓ | — | ✓ |
| Any other app (tick it under Pick in Settings) | ✓ | — | ✓ |

**AI apps**: Claude, ChatGPT, Codex, Grok, Gemini, DeepSeek Harness, ZCode, Kimi, Doubao, Perplexity, GitHub Copilot, Yuanbao, Qwen, DuMate, Cline, OpenCode, Trae, MiniMax, Antigravity, Cue, Muse, WorkBuddy, Kilo Code, Cursor, Xiaomi MiMo, CodeArts, Devin, Hermes, KreoCrew, Kiro Crew, Qoder, VS Code

**Terminals**: Terminal, iTerm2, Warp, Ghostty, WezTerm, Alacritty, kitty, Hyper, Tabby, Windows Terminal, PowerShell, cmd

On Windows, Doubao and Trae CN cannot be saved from yet.

## Development

```bash
npm install
npm run tauri dev
```

On first launch, macOS asks you to allow Wings under System Settings → Privacy & Security → Accessibility.

## Where the idea comes from

Wings' "save the next line, send it when the AI is free" comes from [Coffee CLI](https://github.com/edison7009/Coffee-CLI) by the same author: its right-hand Tasks panel sends a noted task straight to the running AI, and was later renamed Pre-input. These commits are on the `main` branch of the public Coffee CLI repository; times are taken from the Git history.

| Time | Commit | What |
|---|---|---|
| 2026-04-20 09:15 (UTC+8) | [`ab42d23`](https://github.com/edison7009/Coffee-CLI/commit/ab42d23f7ffd85686b6090cd83995877bd631814) | Tasks panel: send a noted task straight to the current AI |
| 2026-04-27 19:24 (UTC+8) | [`9f6917a`](https://github.com/edison7009/Coffee-CLI/commit/9f6917a567b873249159730373b9f062942e72cb) | TRADEMARKS claims "send a task to the AI" as the Pitch trademark |
| 2026-11-09 09:01 (UTC+8) | [`f05e54e`](https://github.com/edison7009/Coffee-CLI/commit/f05e54e7df4df74c02ee09a54018afe739ce90b9) | Renamed to Pre-input; drafts can be saved from the input box |
| 2026-11-10 18:40 (UTC+8) | [`f3565f5`](https://github.com/edison7009/Coffee-CLI/commit/f3565f512a059a6468d5169b7dc8f62119b5213f) | "Save to pre-input" button that flips up above the input box |

Wings takes this idea out of Coffee CLI and brings it to Claude, ChatGPT and any other desktop app.
