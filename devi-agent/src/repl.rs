//! 终端 REPL
//! 使用 rustyline 提供带历史记录的交互式命令行

use rustyline::error::ReadlineError;
use rustyline::DefaultEditor;
use anyhow::Result;

/// REPL 交互终端
pub struct Repl {
    editor: DefaultEditor,
    prompt: String,
}

impl Repl {
    /// 创建新的 REPL 实例
    pub fn new() -> Result<Self> {
        let mut editor = DefaultEditor::new()?;
        // 尝试加载历史记录
        if let Some(path) = Self::history_path() {
            let _ = editor.load_history(&path);
        }
        Ok(Self {
            editor,
            prompt: "DeviAgent> ".to_string(),
        })
    }

    /// 读取一行用户输入
    /// 返回 None 表示用户按了 Ctrl+C 或 Ctrl+D
    pub fn read_line(&mut self) -> Result<Option<String>> {
        match self.editor.readline(&self.prompt) {
            Ok(line) => {
                let _ = self.editor.add_history_entry(&line);
                Ok(Some(line))
            }
            Err(ReadlineError::Interrupted) | Err(ReadlineError::Eof) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// 保存历史记录到文件
    pub fn save_history(&mut self) {
        if let Some(path) = Self::history_path() {
            let _ = self.editor.save_history(&path);
        }
    }

    /// 历史记录文件路径: ~/.devi-agent/history.txt
    fn history_path() -> Option<std::path::PathBuf> {
        let mut path = home_dir()?;
        path.push(".devi-agent");
        std::fs::create_dir_all(&path).ok();
        path.push("history.txt");
        Some(path)
    }
}

/// 获取用户主目录（跨平台）
fn home_dir() -> Option<std::path::PathBuf> {
    dirs::home_dir()
}
