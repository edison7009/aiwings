// Product aliases, not window-title substrings: Cue must not match "rescue".
const APPS: &[&[&str]] = &[
    &["claude"], &["chatgpt"], &["codex"], &["grok", "grok bot", "grokbot"],
    &["gemini"], &["deepseek", "deepseek harness", "dsh"], &["zcode"], &["kimi"],
    &["doubao", "doubaowork", "豆包", "豆包工作"], &["perplexity"], &["copilot", "github copilot"],
    &["元宝"], &["通义", "qwen", "qwenwork", "qwenworkcn", "千问", "千问办公", "千问办公国际版"],
    &["dumate"], &["cline"], &["opencode"], &["trae", "trae cn", "trae-cn"],
    &["minimax", "minimax code"], &["antigravity"], &["cue"], &["muse"],
    &["workbuddy", "workbuddy ai", "workbuddyai", "workbuddy-ai"], &["kilo", "kilo code", "kilo-desktop"], &["cursor"],
    &["mimo", "xiaomi mimo"], &["codearts", "code arts"], &["devin"],
    &["hermes"], &["kreocrew"], &["kirocrew", "kiro crew"], &["qoder", "qoder cn", "qoder-cn", "qodercn"],
    // Extension products share the VS Code executable and bundle.
    &["code", "visual studio code", "com.microsoft.vscode"],
];

fn contains_alias(text: &str, alias: &str) -> bool {
    text.match_indices(alias).any(|(i, _)| {
        let word = |c: char| c.is_alphanumeric();
        !text[..i].chars().next_back().is_some_and(word)
            && !text[i + alias.len()..].chars().next().is_some_and(word)
    })
}

pub fn rank(name: &str, key: &str) -> usize {
    let name = name.to_lowercase();
    let key = key.to_lowercase();
    // On Windows the executable is authoritative: arbitrary document titles are not products.
    let exe = if key == "github.exe" && contains_alias(&name, "github copilot") {
        Some("copilot")
    } else { key.strip_suffix(".exe") };
    APPS.iter().position(|aliases| aliases.iter().any(|alias| {
        if let Some(exe) = exe { exe == *alias }
        else { contains_alias(&name, alias) || contains_alias(&key, alias) }
    })).unwrap_or(APPS.len())
}

pub fn is_ai(name: &str, key: &str) -> bool { rank(name, key) < APPS.len() }

pub fn is_qoder(key: &str) -> bool {
    let key = key.to_lowercase();
    contains_alias(&key, "qoder") || contains_alias(&key, "qodercn")
}

pub fn is_ide(name: &str, key: &str) -> bool {
    let text = format!("{} {}", name, key).to_lowercase();
    ["cursor", "trae", "antigravity", "qoder", "codearts", "code arts", "vscode", "visual studio code", "com.todesktop.230313mzl4w4u92"]
        .iter().any(|a| contains_alias(&text, a)) || key.eq_ignore_ascii_case("code.exe") || is_qoder(key)
}

/// IDEs expose search, terminals and code editors as text inputs too.
/// Require conversation metadata; do not infer it from geometry.
pub fn is_chat_input(ide: bool, hints: &str) -> bool {
    let hints = hints.to_lowercase();
    if ["terminal", "search", "find", "replace", "editor content", "终端", "搜索", "替换"]
        .iter().any(|h| hints.contains(h)) { return false; }
    if !ide { return true; }
    ["chat", "prompt", "message", "ask", "聊天", "提问", "消息", "询问", "对话"]
        .iter().any(|h| hints.contains(h))
}

/// Explicit labels observed in these products, without accepting generic IDE task fields.
pub fn is_chat_input_for_app(key: &str, ide: bool, hints: &str) -> bool {
    if !is_chat_input(false, hints) { return false; }
    if is_chat_input(ide, hints) { return true; }
    let key = key.to_lowercase();
    (is_qoder(&key) && matches!(hints.trim(), "新的任务" | "New task"))
        || (contains_alias(&key, "codearts") && hints.contains("尽管吩咐，我来帮你搞定"))
}

/// Cursor's compact follow-up row keeps the model and voice controls beside the text.
/// Its taller new-chat composer retains the default width limit.
pub fn input_width_slack(key: &str, text_height: f64, box_height: f64, scale: f64) -> f64 {
    let key = key.to_lowercase();
    let cursor = contains_alias(&key, "cursor") || key == "com.todesktop.230313mzl4w4u92";
    if contains_alias(&key, "gemini") || (cursor && box_height <= text_height + 32.0 * scale) {
        320.0
    } else { 160.0 }
}

/// Rich text editors can keep a BOM/zero-width sentinel even when visibly empty.
pub fn has_input_text(text: &str, placeholder: Option<&str>) -> bool {
    let trim = |c: char| c.is_whitespace() || matches!(c, '\u{feff}' | '\u{200b}' | '\u{2060}');
    let text = text.trim_matches(trim);
    !text.is_empty() && placeholder.is_none_or(|p| text != p.trim_matches(trim))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cursor_compact_row_does_not_widen_new_chat() {
        assert_eq!(input_width_slack("cursor.exe", 24.0, 42.0, 1.0), 320.0);
        assert_eq!(input_width_slack("cursor.exe", 24.0, 82.0, 1.0), 160.0);
        assert_eq!(input_width_slack("cursor.exe", 48.0, 128.0, 1.0), 160.0);
        assert_eq!(input_width_slack("com.todesktop.230313mzl4w4u92", 24.0, 42.0, 1.0), 320.0);
        assert_eq!(input_width_slack("cursor.exe", 36.0, 63.0, 1.5), 320.0);
        assert_eq!(input_width_slack("workbuddy.exe", 24.0, 42.0, 1.0), 160.0);
        assert_eq!(input_width_slack("gemini.exe", 24.0, 64.0, 1.0), 320.0);
    }
    #[test]
    fn regional_editions_share_input_rules() {
        for key in ["Qoder.exe", "Qoder CN.exe", "qoder-cn.exe", "qodercn.exe", "com.qoder.cn"] {
            assert!(is_ai("Qoder CN", key));
            assert!(is_ide("", key));
            assert!(is_qoder(key));
            assert!(is_chat_input_for_app(key, true, "新的任务"));
            assert!(!is_chat_input_for_app(key, true, "Search tasks"));
        }
        for key in ["Trae.exe", "Trae CN.exe", "cn.trae.app"] {
            assert!(is_ai("TRAE", key));
            assert!(is_ide("", key));
            assert!(is_chat_input_for_app(key, true, "Chat input"));
            assert!(!is_chat_input_for_app(key, true, "Editor content"));
        }
        for key in ["WorkBuddy.exe", "WorkBuddy AI.exe", "WorkBuddyAI.exe", "com.tencent.workbuddy"] {
            assert!(is_ai("WorkBuddy AI", key));
            assert!(!is_ide("", key));
            assert!(is_chat_input_for_app(key, false, "Message"));
            assert!(!is_chat_input_for_app(key, false, "搜索会话"));
        }
    }
    #[test]
    fn product_specific_chat_labels_stay_scoped() {
        assert!(is_chat_input_for_app("qoder.exe", true, "新的任务  "));
        assert!(!is_chat_input_for_app("code.exe", true, "新的任务"));
        assert!(!is_chat_input_for_app("qoder.exe", true, "搜索新的任务"));
        assert!(is_chat_input_for_app("codearts.exe", true, "尽管吩咐，我来帮你搞定… / 调用技能"));
        assert!(!is_chat_input_for_app("codearts.exe", true, "搜索 尽管吩咐，我来帮你搞定"));
        assert!(!is_chat_input_for_app("codearts.exe", true, "Editor content"));
    }
    #[test]
    fn empty_rich_text_and_placeholder() {
        for text in ["", " \n", "\u{feff}", "\u{200b}\u{feff} "] {
            assert!(!has_input_text(text, None));
        }
        assert!(!has_input_text("\u{feff} Type a message ", Some("Type a message")));
        assert!(has_input_text("\u{feff}hello", None));
        assert!(has_input_text("👩‍💻", None));
    }
    #[test]
    fn products_and_regional_editions() {
        for (name, key) in [("TRAE CN", "Trae CN.exe"), ("Cursor", "com.todesktop.230313mzl4w4u92"),
            ("DeepSeek Harness", "DeepSeek Harness.exe"), ("豆包工作", "DoubaoWork.exe"), ("千问办公", "cn.qwen.work"),
            ("MiMo", "Xiaomi MiMo.exe"), ("Kilo", "kilo-desktop.exe"), ("GitHub Copilot", "github.exe"), ("Kiro Crew", "KiroCrew.exe"), ("VS Code", "Code.exe")] {
            assert!(is_ai(name, key), "{name}: {key}");
        }
    }
    #[test]
    fn documents_are_not_apps() {
        for (name, key) in [("Cursor notes", "notepad.exe"), ("GitHub Desktop", "github.exe"), ("Gemini", "chrome.exe"),
            ("Rescue", "org.rescue.app"), ("Museums", "org.museums.app")] {
            assert!(!is_ai(name, key), "{name}: {key}");
        }
    }
    #[test]
    fn ide_requires_conversation_context() {
        assert!(is_chat_input(true, "Chat input"));
        assert!(is_chat_input(true, "Ask anything"));
        assert!(!is_chat_input(true, "Editor content"));
        assert!(!is_chat_input(true, "Search chat history"));
        assert!(!is_chat_input(true, "xterm-helper-textarea"));
        assert!(!is_chat_input(false, "搜索会话名称"));
        assert!(is_chat_input(false, "发消息或创建任务"));
        assert!(is_ide("", "com.todesktop.230313mzl4w4u92"));
    }
}
