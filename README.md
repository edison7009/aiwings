<p align="center"><img src="branding/wings-banner.png" alt="Wings" width="420"></p>

# Wings · 预输入

<p align="center"><a href="https://aiwings.app">aiwings.app</a></p>

<p align="center">简体中文 | <a href="README.en.md">English</a></p>

<p align="center"><img src="branding/wings-demo-zh.gif" alt="Wings 操作演示" width="720"></p>

## Description

AI 正忙时，先把下一句话存进 Wings，等它空了，点一下就贴回输入框。Wings 是一条吸附在 Claude、ChatGPT 等 AI 桌面应用旁边的侧栏，支持 macOS 和 Windows。

## 怎么用

- **存**：在输入框里写好，点输入框上方的「保存到 Wings」，或按 **Alt+S**（macOS 为 ⌥S）。文字（或一张图片）收进 Wings，输入框清空。
- **发**：点 Wings 里的一条，它被粘贴回输入框，不会自动发送；同时留在剪贴板上，也能粘贴到别处。
- **跟随**：切到别的 AI 应用，Wings 跟过去；也可以固定陪着某一个应用，或改成悬停在任意位置。
- **语言**：界面支持简体中文、繁體中文、English、日本語、한국어、Español、Français、Deutsch、Português、Русский，在设置里切换，默认跟随系统。
- **终端**：Windows Terminal、PowerShell 5/7、cmd 可跟随并粘贴便签，不按回车执行；不支持保存和清空。已有自选跟随名单时，在设置中勾选终端。

## 支持的应用

| 类别 | 吸附跟随 | 一键保存 | 粘贴回输入框 |
|---|:-:|:-:|:-:|
| AI 应用 | ✓ | ✓ | ✓ |
| 终端 | ✓ | — | ✓ |
| 其他应用（在设置的「指定」里勾选） | ✓ | — | ✓ |

**AI 应用**：Claude、ChatGPT、Codex、Grok、Gemini、DeepSeek Harness、ZCode、Kimi、豆包、Perplexity、GitHub Copilot、元宝、通义千问、DuMate、Cline、OpenCode、Trae、MiniMax、Antigravity、Cue、Muse、WorkBuddy、Kilo Code、Cursor、Xiaomi MiMo、CodeArts、Devin、Hermes、KreoCrew、Kiro Crew、Qoder、VS Code

**终端**：Terminal、iTerm2、Warp、Ghostty、WezTerm、Alacritty、kitty、Hyper、Tabby、Windows Terminal、PowerShell、cmd

Windows 上的豆包和 Trae CN 暂不支持一键保存。

## 开发

```bash
npm install
npm run tauri dev
```

macOS 首次启动需要在「系统设置 → 隐私与安全性 → 辅助功能」里允许 Wings。

## 创意来源

Wings 的「先存下一句、等 AI 空了再发」来自同一作者的 [Coffee CLI](https://github.com/edison7009/Coffee-CLI)：它右侧的「任务」面板可以把记下的任务一键发给正在运行的 AI，后来改名为「预输入」。下面这些提交都在 Coffee CLI 公开仓库的 `main` 分支上，时间以 Git 提交记录为准：

| 时间 | 提交 | 内容 |
|---|---|---|
| 2026-04-20 09:15（UTC+8） | [`ab42d23`](https://github.com/edison7009/Coffee-CLI/commit/ab42d23f7ffd85686b6090cd83995877bd631814) | 右侧「任务」面板：把记下的任务直接发给当前的 AI |
| 2026-04-27 19:24（UTC+8） | [`9f6917a`](https://github.com/edison7009/Coffee-CLI/commit/9f6917a567b873249159730373b9f062942e72cb) | 在 TRADEMARKS 文档中把「任务发给 AI」声明为 Pitch 商标 |
| 2026-11-09 09:01（UTC+8） | [`f05e54e`](https://github.com/edison7009/Coffee-CLI/commit/f05e54e7df4df74c02ee09a54018afe739ce90b9) | 改名为「预输入」，可从输入框保存草稿 |
| 2026-11-10 18:40（UTC+8） | [`f3565f5`](https://github.com/edison7009/Coffee-CLI/commit/f3565f512a059a6468d5169b7dc8f62119b5213f) | 输入框上方翻起的「保存到预输入」按钮 |

Wings 把这个做法从 Coffee CLI 里独立出来，用在 Claude、ChatGPT 等任何桌面应用上。
