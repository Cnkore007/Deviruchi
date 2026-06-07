fn main() {
    #[cfg(target_os = "windows")]
    {
        let mut res = winres::WindowsResource::new();
        res.set_icon("resources/AppIcon.ico");
        res.compile().expect("Failed to embed Windows resource");
    }
}
