use fluent_bundle::{FluentArgs, FluentBundle, FluentResource};
use unic_langid::LanguageIdentifier;
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::sync::RwLock;

pub struct I18n {
    bundles: RwLock<HashMap<String, FluentBundle<FluentResource>>>,
    default_locale: String,
}

impl I18n {
    pub fn new(locale_dir: &str, default_locale: &str) -> Self {
        let i18n = Self {
            bundles: RwLock::new(HashMap::new()),
            default_locale: default_locale.to_string(),
        };
        i18n.load_all(locale_dir);
        i18n
    }

    fn load_all(&self, locale_dir: &str) {
        let base = Path::new(locale_dir);
        if !base.exists() {
            tracing::warn!("i18n locale directory not found: {}", locale_dir);
            return;
        }

        let entries = match fs::read_dir(base) {
            Ok(e) => e,
            Err(e) => {
                tracing::warn!("Failed to read locale directory {}: {}", locale_dir, e);
                return;
            }
        };

        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }
            let locale_name = match path.file_name().and_then(|n| n.to_str()) {
                Some(n) => n.to_string(),
                None => continue,
            };

            let lang_id: LanguageIdentifier = match locale_name.parse() {
                Ok(id) => id,
                Err(_) => {
                    tracing::warn!("Invalid locale identifier: {}", locale_name);
                    continue;
                }
            };

            let mut bundle = FluentBundle::new(vec![lang_id]);
            let mut loaded = 0;

            if let Ok(ftl_entries) = fs::read_dir(&path) {
                for ftl_entry in ftl_entries.flatten() {
                    let ftl_path = ftl_entry.path();
                    if ftl_path.extension().and_then(|e| e.to_str()) != Some("ftl") {
                        continue;
                    }
                    match fs::read_to_string(&ftl_path) {
                        Ok(content) => match FluentResource::try_new(content) {
                            Ok(resource) => {
                                if bundle.add_resource(resource).is_ok() {
                                    loaded += 1;
                                }
                            }
                            Err((_, errs)) => {
                                for e in errs {
                                    tracing::warn!(
                                        "Fluent parse error in {:?}: {}",
                                        ftl_path,
                                        e
                                    );
                                }
                            }
                        },
                        Err(e) => {
                            tracing::warn!("Failed to read {:?}: {}", ftl_path, e);
                        }
                    }
                }
            }

            if loaded > 0 {
                tracing::info!("Loaded i18n locale '{}': {} file(s)", locale_name, loaded);
                self.bundles
                    .write()
                    .unwrap()
                    .insert(locale_name, bundle);
            }
        }
    }

    pub fn t(&self, key: &str) -> String {
        self.t_with_args(key, None)
    }

    pub fn t_with_args(&self, key: &str, args: Option<&FluentArgs>) -> String {
        let locale = self.default_locale.clone();
        self.translate(&locale, key, args)
            .or_else(|| self.translate("en-US", key, args))
            .unwrap_or_else(|| key.to_string())
    }

    fn translate(&self, locale: &str, key: &str, args: Option<&FluentArgs>) -> Option<String> {
        let bundles = self.bundles.read().unwrap();
        let bundle = bundles.get(locale)?;
        let msg = bundle.get_message(key)?;
        let pattern = msg.value()?;
        let mut errors = Vec::new();
        let result = bundle.format_pattern(pattern, args, &mut errors);
        Some(result.into_owned())
    }

    pub fn set_locale(&mut self, locale: &str) {
        self.default_locale = locale.to_string();
    }
}

static mut I18N: Option<&I18n> = None;
static I18N_INIT: std::sync::Once = std::sync::Once::new();

pub fn init(locale_dir: &str, default_locale: &str) {
    let i18n = Box::new(I18n::new(locale_dir, default_locale));
    unsafe {
        I18N_INIT.call_once(|| {
            I18N = Some(Box::leak(i18n));
        });
    }
}

pub fn t(key: &str) -> String {
    unsafe {
        match I18N {
            Some(i18n) => i18n.t(key),
            None => key.to_string(),
        }
    }
}

pub fn t_with_args(key: &str, args: &FluentArgs) -> String {
    unsafe {
        match I18N {
            Some(i18n) => i18n.t_with_args(key, Some(args)),
            None => key.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn setup_test_locales() -> String {
        let dir = std::env::temp_dir().join(format!("deviruchi_i18n_test_{}", rand::random::<u32>()));
        let zh_dir = dir.join("zh-CN");
        let en_dir = dir.join("en-US");
        fs::create_dir_all(&zh_dir).unwrap();
        fs::create_dir_all(&en_dir).unwrap();

        fs::write(
            zh_dir.join("server.ftl"),
            "server-starting = \u{670d}\u{52a1}\u{5668}\u{542f}\u{52a8}\u{4e2d}...\nserver-listening = \u{76d1}\u{542c}\u{7aef}\u{53e3}: { $addr }\nlogin-success = \u{767b}\u{5f55}\u{6210}\u{529f}: { $user }\n",
        ).unwrap();

        fs::write(
            en_dir.join("server.ftl"),
            "server-starting = Server starting...\nserver-listening = Listening on: { $addr }\nlogin-success = Login success: { $user }\n",
        ).unwrap();

        dir.to_string_lossy().to_string()
    }

    #[test]
    fn test_i18n_load_and_translate() {
        let dir = setup_test_locales();
        let i18n = I18n::new(&dir, "zh-CN");

        assert_eq!(i18n.t("server-starting"), "\u{670d}\u{52a1}\u{5668}\u{542f}\u{52a8}\u{4e2d}...");
        assert_eq!(i18n.t("nonexistent-key"), "nonexistent-key");

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_i18n_with_args() {
        let dir = setup_test_locales();
        let i18n = I18n::new(&dir, "zh-CN");

        let mut args = FluentArgs::new();
        args.set("addr", "0.0.0.0:6121");
        let result = i18n.t_with_args("server-listening", Some(&args));
        assert!(result.contains("0.0.0.0:6121"));

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_i18n_fallback_to_key() {
        let dir = setup_test_locales();
        let i18n = I18n::new(&dir, "zh-CN");
        assert_eq!(i18n.t("totally-missing"), "totally-missing");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_i18n_fallback_to_en_us() {
        let dir = setup_test_locales();
        let i18n = I18n::new(&dir, "fr-FR");
        // zh-CN not requested, but en-US should be loaded as fallback
        assert_eq!(i18n.t("server-starting"), "Server starting...");
        let _ = fs::remove_dir_all(&dir);
    }
}
