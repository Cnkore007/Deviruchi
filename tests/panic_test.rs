use deviruchi::core::panic::PanicHandler;

#[test]
fn test_panic_hook_installed() {
    PanicHandler::init();
    // 如果能执行到这里说明 hook 安装成功
}
