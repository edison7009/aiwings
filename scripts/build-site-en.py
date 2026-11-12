"""Generates the English home page (website/index.html) from the Chinese one (website/zh/index.html).

The Chinese page is the source of truth: edit it, then run  python3 scripts/build-site-en.py
Each pair below is (Chinese text, English text); the script stops if a Chinese string is missing,
so a wording change in the Chinese page must be mirrored here.
"""
import sys
from pathlib import Path
site = Path(__file__).resolve().parent.parent / "website"
src, dst = site / "zh" / "index.html", site / "index.html"
s = open(src).read()
R = [
('<html lang="zh-CN">','<html lang="en">'),
('<title>Wings 预输入 · AI 正在思考中，朕却日理万机</title>','<title>Wings · AI is thinking. My mind is racing.</title>'),
('content="Wings 是吸附在 Claude、ChatGPT 等 AI 应用右侧的窗口，你不需要等待 AI 执行完成，你也可以继续输入你的计划和想法。"','content="Wings is a window that docks to the right of Claude, ChatGPT and other AI apps. No need to wait for the AI to finish: keep typing your plans and ideas."'),
('family=Long+Cang&','family=Caveat:wght@600&'),
('href="../assets/','href="assets/'),('src="../assets/','src="assets/'),
('跳到正文','Skip to content'),
('>怎么用<','>How it works<'),('>功能<','>Features<'),('>常见问题<','>FAQ<'),
('<a class="icon-btn lang" href="../" lang="en">','<a class="icon-btn lang" href="zh/" lang="zh-CN">'),('<span>English</span></a>','<span>中文</span></a>'),
('>下载</a>','>Download</a>'),
('<h1><span class="nowrap">AI 正在思考中</span><br><span class="nowrap">朕却<em>日理万机</em></span></h1>','<h1><span class="nowrap">AI is thinking</span><br><span class="nowrap"><em>My mind is racing</em></span></h1>'),
('Wings 是吸附在 Claude、ChatGPT 等 AI 应用右侧的窗口，你不需要等待 AI 执行完成，你也可以继续输入你的计划和想法。','Wings is a window that docks to the right of Claude, ChatGPT and other AI apps. No need to wait for the AI to finish: keep typing your plans and ideas.'),
('下载 macOS 版','Download for macOS'),('在 GitHub 点个星','Star on GitHub'),
('<span>自动吸附</span><span>一键保存</span><span>一键发送</span>','<span>Auto-docks</span><span>One-click save</span><span>One-click send</span>'),
('Wings自动吸附在AI应用的右边','That strip on the right is Wings'),
('AI正在思考，你也能继续输入','AI is thinking, keep typing'),
('<span class="nowrap">等 AI 思考和执行的十几分钟</span><br><span class="nowrap">你的下一句放哪？</span>','While the AI works,<br>where do your ideas go?'),
('长任务一跑就是十几分钟。想法不等人，可现在的去处都不太好。','Long tasks run for ten minutes or more. Ideas don\'t wait, and the usual places to put them aren\'t great.'),
('打在输入框里？手一快按了回车，<b>半句话就发出去了</b>。','In the input box? One quick Enter and <b>half a thought gets sent</b>.'),
('记在备忘录里？来回切窗口，<b>回头还得找、还得复制</b>。','In a notes app? Switching windows, then <b>finding it and copying it back</b>.'),
('三步，用起来不用想','Three steps, no thinking required'),
('>照常写<','>Write as usual<'),('在 AI 的输入框里写下一句，和平时一样。','Type your next message in the AI\'s input box, like always.'),
('>存进 Wings<','>Save to Wings<'),('点输入框上方的「保存到 Wings」，或按 <kbd>⌥S</kbd>。输入框清空，这句话收进侧栏。','Click "Save to Wings" above the input, or press <kbd>⌥S</kbd>. The input clears and the message moves to the sidebar.'),
('>点一下贴回<','>Click to paste back<'),('AI 回完了，点一下便签，它回到输入框。看一眼没问题，再按回车。','When the AI is done, click the note and it goes back into the input. Check it, then press Enter.'),
('小工具，但想得很细','Small tool, carefully made'),
('跟着你走，也能停下来','Follows you, or stays put'),
('切到另一个 AI 应用，侧栏跟过去贴好。也可以只陪一个应用，或者悬停在屏幕任意位置。','Switch to another AI app and the sidebar docks beside it. Or stick to one app, or float anywhere on screen.'),
('<span>跟随</span><span>固定</span><span>悬停</span>','<span>Follow</span><span>Pin</span><span>Float</span>'),
('>绝不替你发送<','>Never sends for you<'),('只粘贴，不按回车。发不发，你说了算。','It pastes, it never presses Enter. Sending is your call.'),
('>剪贴板也有一份<','>Also on your clipboard<'),('点过的便签同时留在剪贴板，⌘V 能贴到任何地方。','A clicked note stays on the clipboard too, so ⌘V puts it anywhere.'),
('>图片也能存<','>Images too<'),('截图存成图片便签，点一下原样贴回去。','Save a screenshot as an image note and paste it back as is.'),
('>只在你的电脑上<','>Only on your computer<'),('便签存在本机，没有账号，也不上传。','Notes live on your Mac. No account, no upload.'),
('贴在你常用的 AI 旁边','Right beside the AI you use'),
('<th>类别</th><th>应用</th><th>吸附跟随</th><th>一键保存</th><th>粘贴回输入框</th>','<th>Kind</th><th>Apps</th><th>Dock and follow</th><th>One-click save</th><th>Paste back</th>'),('<th>AI 应用</th>','<th>AI apps</th>'),('<th>终端</th>','<th>Terminals</th>'),('<th>其他应用</th><td>在设置的「指定」里勾选</td>','<th>Any other app</th><td>Tick it under Pick in Settings</td>'),('、豆包、','、Doubao、'),('、元宝、通义千问、','、Yuanbao、Qwen、'),
('<summary>会自动帮我把话发出去吗？</summary><p>不会。Wings 只把便签粘贴进输入框，从不按回车。</p>','<summary>Will it send messages for me?</summary><p>No. Wings only pastes the note into the input box. It never presses Enter.</p>'),
('<summary>为什么需要「辅助功能」权限？</summary><p>保存和粘贴要模拟全选、剪切、粘贴这几个按键，macOS 要求先在「隐私与安全性 → 辅助功能」里允许。首次打开时 Wings 会带你一步步打开。</p>','<summary>Why does it need Accessibility access?</summary><p>Saving and pasting simulate select all, cut and paste, which macOS only allows after you enable it in Privacy &amp; Security → Accessibility. Wings walks you through it on first launch.</p>'),
('<summary>Wings 会读取我和 AI 的对话吗？</summary><p>不会。只在你保存时剪下输入框里的那句话。为了判断输入框是不是空的，会在本机看一眼里面有没有字，不保存也不上传。</p>','<summary>Does Wings read my conversations?</summary><p>No. It only cuts the text in the input box when you save. To tell whether the input is empty it checks locally for text, without storing or uploading anything.</p>'),
('<summary>便签存在哪里？</summary><p>存在你电脑上 Wings 的数据目录里。没有账号，也没有云同步。</p>','<summary>Where are notes stored?</summary><p>In Wings\' data folder on your computer. No account and no cloud sync.</p>'),
('<summary>保存快捷键能改吗？</summary><p>可以。在设置里点一下快捷键，按下一个修饰键加一个键就行，比如 ⌥S 或 ⌃E。</p>','<summary>Can I change the save shortcut?</summary><p>Yes. Click the shortcut in Settings and press one modifier plus one key, like ⌥S or ⌃E.</p>'),
('<summary>在终端里能用吗？</summary><p>可以跟随和粘贴，比如 Terminal、iTerm2。终端里的输入行由里面运行的程序管理，所以不支持保存。</p>','<summary>Does it work in terminals?</summary><p>It can follow and paste into Terminal or iTerm2. The input line belongs to whatever program is running there, so saving isn\'t supported.</p>'),
('<summary>支持 Windows 吗？</summary><p>正在适配，完成后会在这里发布。</p>','<summary>Is there a Windows version?</summary><p>It\'s in progress and will be released here when ready.</p>'),
('>它从哪里来<','>Where it came from<'),
('Wings 来自同一作者的 <a href="https://github.com/edison7009/Coffee-CLI">Coffee CLI</a>。它右侧的「任务」面板能把记下的话一键发给 AI，后来改名叫「预输入」。Wings 把这件事带到了所有 AI 桌面应用上。','Wings comes from <a href="https://github.com/edison7009/Coffee-CLI">Coffee CLI</a> by the same author. Its side panel, first called Tasks and later Pre-input, sends saved messages to the AI in one click. Wings brings that to every AI desktop app.'),
('<span>Wings 预输入，aiwings.app</span>','<span>Wings, aiwings.app</span>'),
]
for a,b in R:
    if a not in s: sys.exit('missing: '+a)
    s = s.replace(a,b)
# App lists in the support table: English commas instead of the Chinese list mark.
import re
s = re.sub(r'<td>[^<]*</td>', lambda m: m.group(0).replace('、', ', '), s)
# English typography: rounded Latin display face, Latin handwriting.
s = s.replace('.cn-display { font-family: "ZCOOL KuaiLe", "Noto Sans SC", sans-serif;','.cn-display { font-family: "Fredoka", "Noto Sans SC", sans-serif;')
s = s.replace('.hand { font-family: "Long Cang", cursive;','.hand { font-family: "Caveat", cursive; font-weight: 600 !important;')
s = s.replace('.hero h1 { font: 900 clamp(38px, 4.6vw, 66px)/1.1', '.hero h1 { font: 900 clamp(30px, 3.7vw, 54px)/1.1')
open(dst,'w').write(s)
