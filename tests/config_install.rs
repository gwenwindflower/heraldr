use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::process::{Command, Output};
use std::sync::atomic::{AtomicUsize, Ordering};

struct ConfigDirectory(PathBuf);

impl ConfigDirectory {
    fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let root = std::env::temp_dir().join(format!(
            "heraldr config-install-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        Self(root)
    }

    fn initialize(&self) -> Output {
        Command::new(env!("CARGO_BIN_EXE_heraldr"))
            .arg("init-config")
            .env("HERDR_PLUGIN_CONFIG_DIR", &self.0)
            .env("HERDR_SOCKET_PATH", self.0.join("no-server.sock"))
            .output()
            .unwrap()
    }
}

impl Drop for ConfigDirectory {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[test]
fn installation_creates_commented_defaults_without_a_running_server() {
    let config = ConfigDirectory::new();
    let output = config.initialize();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let text = std::fs::read_to_string(config.0.join("config.toml")).unwrap();
    for setting in ["agent_icons", "tab_label", "session_title_max_width"] {
        let mut previous = "";
        let mut documented = false;
        for line in text.lines() {
            if line.starts_with(&format!("{setting} =")) {
                documented = previous.starts_with('#');
            }
            previous = line;
        }
        assert!(documented, "missing comment above {setting}");
    }
    assert!(text.contains("$session_title"));
    assert!(text.contains("font") && text.contains("none"));
}

#[test]
fn repeated_installation_preserves_user_settings_in_either_format() {
    for name in ["config.toml", "config.json"] {
        let config = ConfigDirectory::new();
        std::fs::create_dir_all(&config.0).unwrap();
        let path = config.0.join(name);
        let contents = "user configuration stays byte-for-byte, even if invalid\n";
        std::fs::write(&path, contents).unwrap();
        for _ in 0..2 {
            assert!(config.initialize().status.success());
        }
        assert_eq!(std::fs::read_to_string(path).unwrap(), contents);
        assert_eq!(std::fs::read_dir(&config.0).unwrap().count(), 1);
    }
}

#[test]
fn installation_uses_herdr_config_lookup_without_runtime_plugin_context() {
    let root = ConfigDirectory::new();
    let bin = root.0.join("bin");
    std::fs::create_dir_all(&bin).unwrap();
    let herdr = bin.join("herdr");
    std::fs::write(&herdr, "#!/bin/sh\n[ \"$*\" = \"plugin config-dir heraldr\" ] || exit 1\nprintf '%s\\n' \"$TEST_PLUGIN_CONFIG_DIR\"\n").unwrap();
    std::fs::set_permissions(&herdr, std::fs::Permissions::from_mode(0o755)).unwrap();
    let directory = root.0.join("herdr managed config");
    let output = Command::new(env!("CARGO_BIN_EXE_heraldr"))
        .arg("init-config")
        .env_remove("HERDR_PLUGIN_CONFIG_DIR")
        .env("PATH", bin)
        .env("TEST_PLUGIN_CONFIG_DIR", &directory)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let installed: serde_json::Value =
        toml::from_str(&std::fs::read_to_string(directory.join("config.toml")).unwrap()).unwrap();
    assert_eq!(
        installed["tab_label"],
        serde_json::json!(["$jump", "$icon", "$program"])
    );
}
