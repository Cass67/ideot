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
    let config = configured_server(extension)?;
    if !command_exists(config.command) {
        return None;
    }
    Some(config.clone())
}

pub fn configured_server_command(extension: &str) -> Option<&'static str> {
    configured_server(extension).map(|server| server.command)
}

fn configured_server(extension: &str) -> Option<&'static LanguageServer> {
    let dotted;
    let key = if extension.starts_with('.') {
        extension
    } else {
        dotted = format!(".{extension}");
        dotted.as_str()
    };
    SERVER_MAP.get(key)
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
    m.insert(
        ".json",
        LanguageServer {
            command: "vscode-json-language-server",
            args: &["--stdio"],
        },
    );
    for ext in [".yaml", ".yml"] {
        m.insert(
            ext,
            LanguageServer {
                command: "yaml-language-server",
                args: &["--stdio"],
            },
        );
    }
    for ext in [".sh", ".bash", ".zsh"] {
        m.insert(
            ext,
            LanguageServer {
                command: "bash-language-server",
                args: &["start"],
            },
        );
    }
    for ext in [".html", ".htm"] {
        m.insert(
            ext,
            LanguageServer {
                command: "vscode-html-language-server",
                args: &["--stdio"],
            },
        );
    }
    for ext in [".css", ".scss", ".less"] {
        m.insert(
            ext,
            LanguageServer {
                command: "vscode-css-language-server",
                args: &["--stdio"],
            },
        );
    }
    for ext in [".c", ".h", ".cpp", ".hpp", ".cc", ".cxx", ".hh", ".hxx"] {
        m.insert(
            ext,
            LanguageServer {
                command: "clangd",
                args: &[],
            },
        );
    }
    m.insert(
        ".lua",
        LanguageServer {
            command: "lua-language-server",
            args: &[],
        },
    );
    for ext in [".md", ".mdx"] {
        m.insert(
            ext,
            LanguageServer {
                command: "marksman",
                args: &[],
            },
        );
    }
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

    #[test]
    fn extension_lookup_accepts_dotless_extension() {
        assert_eq!(configured_server_command("rs"), Some("rust-analyzer"));
    }

    #[test]
    fn common_language_extensions_map_to_servers() {
        for (extension, command) in [
            ("json", "vscode-json-language-server"),
            ("yaml", "yaml-language-server"),
            ("sh", "bash-language-server"),
            ("html", "vscode-html-language-server"),
            ("css", "vscode-css-language-server"),
            ("c", "clangd"),
            ("cpp", "clangd"),
            ("lua", "lua-language-server"),
            ("md", "marksman"),
        ] {
            assert_eq!(configured_server_command(extension), Some(command));
        }
    }
}
