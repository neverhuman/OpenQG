use super::*;

pub(crate) fn env_string(name: &str) -> Option<String> {
    env::var(name)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

pub(crate) fn jailgun_bridge_command() -> JailgunBridgeCommand {
    jailgun_bridge_command_from_env_lookup(env_string)
}

pub(crate) fn jailgun_bridge_command_from_env_lookup<F>(lookup: F) -> JailgunBridgeCommand
where
    F: FnMut(&str) -> Option<String>,
{
    jailgun_bridge_command_from_env_lookup_with_default(lookup, DEFAULT_JAILGUN_BRIDGE_COMMAND)
}

pub(crate) fn jailgun_bridge_command_from_env_lookup_with_default<F>(
    mut lookup: F,
    default_args: &[&str],
) -> JailgunBridgeCommand
where
    F: FnMut(&str) -> Option<String>,
{
    if let Some(value) = lookup("JAILGUN_BRIDGE_CMD") {
        let args = parse_bridge_command(&value);
        if !args.is_empty() {
            return JailgunBridgeCommand {
                args,
                source: "env:JAILGUN_BRIDGE_CMD".to_string(),
            };
        }
    }
    JailgunBridgeCommand {
        args: default_args.iter().map(|arg| (*arg).to_string()).collect(),
        source: "default:chrome-bridge".to_string(),
    }
}

pub(crate) fn parse_bridge_command(value: &str) -> Vec<String> {
    value
        .split_whitespace()
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .map(ToString::to_string)
        .collect()
}

pub(crate) fn jailgun_server_url() -> String {
    env_string("JAILGUN_SERVER_URL").unwrap_or_else(|| DEFAULT_JAILGUN_SERVER_URL.to_string())
}

pub(crate) fn jailgun_account_ids_override() -> Vec<String> {
    env_string("JAILGUN_ACCOUNT_IDS")
        .map(|value| parse_account_ids(&value))
        .unwrap_or_default()
}

pub(crate) fn resolve_jailgun_token(server_url: &str) -> Option<JailgunToken> {
    jailgun_token_from_env().or_else(|| jailgun_token_from_proc(server_url))
}

pub(crate) fn jailgun_token_from_env() -> Option<JailgunToken> {
    jailgun_token_from_env_lookup(env_string)
}

pub(crate) fn jailgun_token_from_env_lookup<F>(mut lookup: F) -> Option<JailgunToken>
where
    F: FnMut(&str) -> Option<String>,
{
    lookup("JAILGUN_INGEST_TOKEN")
        .map(|value| JailgunToken {
            value,
            source: "env:JAILGUN_INGEST_TOKEN".to_string(),
        })
        .or_else(|| {
            lookup("JAILGUN_TOKEN").map(|value| JailgunToken {
                value,
                source: "env:JAILGUN_TOKEN".to_string(),
            })
        })
}

pub(crate) fn jailgun_token_from_proc(server_url: &str) -> Option<JailgunToken> {
    let entries = jailgun_proc_entries(server_url);
    jailgun_token_from_proc_entries(&entries, server_url)
}

pub(crate) fn jailgun_proc_entries(server_url: &str) -> Vec<JailgunProcEntry> {
    let Ok(entries) = fs::read_dir("/proc") else {
        return Vec::new();
    };
    let mut snapshots = Vec::new();
    for entry in entries.flatten() {
        let file_name = entry.file_name();
        let Some(pid) = file_name.to_str() else {
            continue;
        };
        if !pid.chars().all(|ch| ch.is_ascii_digit()) {
            continue;
        }
        let proc_dir = entry.path();
        let Ok(cmdline) = read_proc_nul_strings(&proc_dir.join("cmdline")) else {
            continue;
        };
        if !jailgun_process_matches_server(&cmdline, server_url) {
            continue;
        }
        let Ok(environ) = read_proc_nul_strings(&proc_dir.join("environ")) else {
            continue;
        };
        snapshots.push(JailgunProcEntry { cmdline, environ });
    }
    snapshots
}

pub(crate) fn read_proc_nul_strings(path: &Path) -> std::io::Result<Vec<String>> {
    fs::read(path).map(|bytes| proc_nul_strings(&bytes))
}

pub(crate) fn proc_nul_strings(bytes: &[u8]) -> Vec<String> {
    bytes
        .split(|byte| *byte == 0)
        .filter(|part| !part.is_empty())
        .map(|part| String::from_utf8_lossy(part).to_string())
        .collect()
}

pub(crate) fn jailgun_token_from_proc_entries(
    entries: &[JailgunProcEntry],
    server_url: &str,
) -> Option<JailgunToken> {
    entries
        .iter()
        .filter(|entry| jailgun_process_matches_server(&entry.cmdline, server_url))
        .find_map(|entry| {
            entry.environ.iter().find_map(|item| {
                item.strip_prefix("JAILGUN_INGEST_TOKEN=")
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .map(|value| JailgunToken {
                        value: value.to_string(),
                        source: "proc:JAILGUN_INGEST_TOKEN".to_string(),
                    })
            })
        })
}

pub(crate) fn jailgun_process_matches_server(cmdline: &[String], server_url: &str) -> bool {
    if cmdline.is_empty() {
        return false;
    }
    let looks_like_jailgun = cmdline.iter().any(|arg| {
        Path::new(arg)
            .file_name()
            .and_then(|name| name.to_str())
            .map(|name| name.contains("jailgun"))
            .unwrap_or_else(|| arg.contains("jailgun"))
    });
    if !looks_like_jailgun || !cmdline.iter().any(|arg| arg == "serve") {
        return false;
    }
    let Some(server_addr) = jailgun_server_addr(server_url) else {
        return false;
    };
    cmdline.iter().enumerate().any(|(index, arg)| {
        if let Some(value) = arg.strip_prefix("--addr=") {
            return jailgun_addr_matches(value, &server_addr);
        }
        if arg == "--addr" {
            return cmdline
                .get(index + 1)
                .map(|value| jailgun_addr_matches(value, &server_addr))
                .unwrap_or(false);
        }
        false
    })
}

pub(crate) fn jailgun_server_addr(server_url: &str) -> Option<String> {
    let without_scheme = server_url
        .trim()
        .split_once("://")
        .map(|(_, rest)| rest)
        .unwrap_or_else(|| server_url.trim());
    without_scheme
        .split('/')
        .next()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
}

pub(crate) fn jailgun_addr_matches(process_addr: &str, server_addr: &str) -> bool {
    process_addr == server_addr
        || (server_addr.starts_with("localhost:")
            && process_addr == server_addr.replacen("localhost", "127.0.0.1", 1))
        || (server_addr.starts_with("127.0.0.1:")
            && process_addr == server_addr.replacen("127.0.0.1", "localhost", 1))
}

pub(crate) fn parse_account_ids(value: &str) -> Vec<String> {
    value
        .split(',')
        .map(str::trim)
        .filter(|id| !id.is_empty())
        .map(ToString::to_string)
        .collect()
}

pub(crate) fn jailgun_url(server_url: &str, path: &str) -> String {
    format!("{}{}", server_url.trim_end_matches('/'), path)
}

pub(crate) fn sanitize_jailgun_token_text(text: &str, token: &str) -> String {
    if token.is_empty() {
        text.to_string()
    } else {
        text.replace(token, "[redacted]")
    }
}

pub(crate) fn redact_jailgun_token_in_value(value: &Value, token: &str) -> Value {
    match value {
        Value::String(text) => Value::String(sanitize_jailgun_token_text(text, token)),
        Value::Array(items) => Value::Array(
            items
                .iter()
                .map(|item| redact_jailgun_token_in_value(item, token))
                .collect(),
        ),
        Value::Object(map) => Value::Object(
            map.iter()
                .map(|(key, value)| (key.clone(), redact_jailgun_token_in_value(value, token)))
                .collect(),
        ),
        _ => value.clone(),
    }
}
