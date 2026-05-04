use std::backtrace::Backtrace;
use std::io::Write;
use std::panic;

pub struct PanicHandler;

impl PanicHandler {
    pub fn init() {
        panic::set_hook(Box::new(Self::handle_panic));
    }

    fn handle_panic(info: &panic::PanicHookInfo) {
        let location = info.location();
        let payload = if let Some(s) = info.payload().downcast_ref::<&str>() {
            s.to_string()
        } else if let Some(s) = info.payload().downcast_ref::<String>() {
            s.clone()
        } else {
            "Unknown".to_string()
        };

        let timestamp = chrono_lite_now();
        let file = location.map(|l| l.file()).unwrap_or("unknown");
        let line = location.map(|l| l.line()).unwrap_or(0);
        let col = location.map(|l| l.column()).unwrap_or(0);

        let report = format!(
            "=====================================\n\
             |       Deviruchi 崩溃报告        |\n\
             =====================================\n\
             时间: {timestamp}\n\
             \n\
             崩溃位置:\n\
               文件: {file}\n\
               行号: {line}\n\
               列号: {col}\n\
             \n\
             崩溃信息:\n\
               {payload}\n\
             \n\
             调用栈:\n\
             {backtrace}\n\
             =====================================\n",
            timestamp = timestamp,
            file = file,
            line = line,
            col = col,
            payload = payload,
            backtrace = Backtrace::capture(),
        );

        // 输出到 stderr
        let _ = write!(std::io::stderr(), "{}", report);

        // 保存到文件
        let crash_file = format!("crash_{}.log", timestamp);
        let _ = std::fs::write(&crash_file, &report);
    }
}

fn chrono_lite_now() -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap();
    format!("{}", now.as_secs())
}
