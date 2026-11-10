// UI copy. Chinese first; kept in one place so other languages can be added later.
export const t = {
  welcome: (key: string) => `上手三步：
1. 在 AI 应用的输入框里写下一句话
2. 按 ${key}，它会被收进这里，输入框清空
3. 等 AI 空下来，点这张便签，它会被粘贴回输入框（不会自动发送）

看完点右上角 × 删掉这张。`,
  emptyTitle: '还没有候着的话',
  emptyBody: (key: string) => `在输入框里写好，按 ${key} 存到这里`,
  pasted: '已粘贴，确认后按回车发送',
  delete: '删除',
  switchTarget: '换窗口',
  pickTitle: '选一个要吸附的窗口',
  pickHint: 'Wings 会贴在它的右边，跟着它移动',
  pickEmpty: '没有找到可用的窗口，先打开 Claude、ChatGPT 等应用',
  refresh: '刷新',
  lastMissing: (name: string) => `上次用的「${name}」没有打开`,
  permTitle: '需要一项系统权限',
  permBody: 'Wings 要模拟「全选、剪切、粘贴」三个按键，macOS 需要你允许「辅助功能」权限。Wings 不读取其他应用的内容。',
  permStep1: '点下面的按钮，打开系统设置',
  permStep2: '在「辅助功能」列表里找到 Wings，打开开关',
  permStep3: '回到这里，会自动继续',
  permButton: '打开系统设置',
  permWaiting: '正在等待授权…',
};
