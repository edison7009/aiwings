<p align="center"><img src="branding/wings-wordmark.svg" alt="Wings" width="320"></p>

# Wings · 候话

AI 正在忙的时候，先把下一句话存起来，等它空了再一键贴回去。

Wings 吸附在 Claude、ChatGPT 等桌面应用的右侧：
- 在目标输入框里按 **Alt+S**（macOS 为 ⌥S），当前文字被收进 Wings，输入框清空。
- 点 Wings 里的一条，它会被粘贴回目标输入框（不自动发送）。剪贴板会恢复原样。

## 开发

```bash
npm install
npm run tauri dev
```

macOS 首次启动需要在「系统设置 → 隐私与安全性 → 辅助功能」里允许 Wings。
