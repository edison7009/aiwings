<p align="center"><img src="branding/wings-icon.svg" alt="Wings" width="128"></p>

# Wings · 候话

AI 正在忙的时候，先把下一句话存起来，等它空了再一键贴回去。

Wings 吸附在 Claude、ChatGPT 等桌面应用的右侧：
- 在目标输入框里按 **Alt+S**（macOS 为 ⌥S），当前文字被收进 Wings，输入框清空。
- 点 Wings 里的一条，它会被粘贴回目标输入框（不自动发送），同时留在剪贴板上，也可以粘贴到别处。

## 开发

```bash
npm install
npm run tauri dev
```

macOS 首次启动需要在「系统设置 → 隐私与安全性 → 辅助功能」里允许 Wings。

## 创意来源

Wings 的「先存下一句、等 AI 空了再发」来自同一作者的 [Coffee CLI](https://github.com/edison7009/Coffee-CLI)：它的 To Do 面板可以把写好的任务一键发给正在运行的 AI，后来改名为「预输入」。下面这些提交都在 Coffee CLI 公开仓库的 `main` 分支上，时间以 Git 提交记录为准：

| 时间 | 提交 | 内容 |
|---|---|---|
| 2026-04-20 09:15（UTC+8） | [`ab42d23`](https://github.com/edison7009/Coffee-CLI/commit/ab42d23f7ffd85686b6090cd83995877bd631814) | To Do 面板：把任务直接发给当前的 AI |
| 2026-04-27 19:24（UTC+8） | [`9f6917a`](https://github.com/edison7009/Coffee-CLI/commit/9f6917a567b873249159730373b9f062942e72cb) | 在 TRADEMARKS 文档中把「To Do 发给 AI」声明为 Pitch 商标 |
| 2026-11-09 09:01（UTC+8） | [`f05e54e`](https://github.com/edison7009/Coffee-CLI/commit/f05e54e7df4df74c02ee09a54018afe739ce90b9) | 改名为「预输入」，可从输入框保存草稿 |
| 2026-11-10 18:40（UTC+8） | [`f3565f5`](https://github.com/edison7009/Coffee-CLI/commit/f3565f512a059a6468d5169b7dc8f62119b5213f) | 输入框上方翻起的「保存到预输入」按钮 |

Wings 把这个做法从 Coffee CLI 里独立出来，用在 Claude、ChatGPT 等任何桌面应用上。
