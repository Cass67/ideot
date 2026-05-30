use std::collections::HashMap;
use std::process::Command;

/// Language server configuration.
#[derive(Debug, Clone)]
pub struct LanguageServer {
    pub command: &'static str,
    pub args: &'static [&'static str],
}

/// Map file extensions to their language server commands.
/// Returns `None` if no server is configured for the extension,
/// or if the server binary is not found on PATH.
pub fn discover_server(extension: &str) -> Option<LanguageServer> {
    let config = SERVER_MAP.get(extension)?;
    if !command_exists(config.command) {
        return None;
    }
    Some(config.clone())
}

/// Check if a command exists on PATH.
fn command_exists(cmd: &str) -> bool {
    Command::new("which")
        .arg(cmd)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

fn build_map() -> HashMap<&'static str, LanguageServer> {
    let mut m = HashMap::new();
    m.insert(
        ".rs",
        LanguageServer {
            command: "rust-analyzer",
            args: &[],
        },
    );
    m.insert(
        ".go",
        LanguageServer {
            command: "gopls",
            args: &[],
        },
    );
    m.insert(
        ".py",
        LanguageServer {
            command: "pylsp",
            args: &[],
        },
    );
    m.insert(
        ".js",
        LanguageServer {
            command: "typescript-language-server",
            args: &["--stdio"],
        },
    );
    m.insert(
        ".jsx",
        LanguageServer {
            command: "typescript-language-server",
            args: &["--stdio"],
        },
    );
    m.insert(
        ".mjs",
        LanguageServer {
            command: "typescript-language-server",
            args: &["--stdio"],
        },
    );
    m.insert(
        ".ts",
        LanguageServer {
            command: "typescript-language-server",
            args: &["--stdio"],
        },
    );
    m.insert(
        ".tsx",
        LanguageServer {
            command: "typescript-language-server",
            args: &["--stdio"],
        },
    );
    m.insert(
        ".vue",
        LanguageServer {
            command: "vue-language-server",
            args: &["--stdio"],
        },
    );
    m
}

static SERVER_MAP: std::sync::LazyLock<HashMap<&'static str, LanguageServer>> =
    std::sync::LazyLock::new(build_map);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unknown_extension_returns_none() {
        assert!(discover_server(".xyz").is_none());
    }

    #[test]
    fn known_extension_maps_to_server() {
        // .rs always maps; whether it's found depends on PATH
        let config = SERVER_MAP.get(".rs").expect(".rs should be in map");
        assert_eq!(config.command, "rust-analyzer");
    }
}
