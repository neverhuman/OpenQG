use super::*;

pub(crate) fn jailgun_mcp_tool_call(
    client: &reqwest::blocking::Client,
    server_url: &str,
    token: &str,
    request_id: &str,
    name: &str,
    arguments: Value,
) -> std::result::Result<Value, String> {
    let body = json!({
        "jsonrpc": "2.0",
        "id": request_id,
        "method": "tools/call",
        "params": {
            "name": name,
            "arguments": arguments,
        },
    });
    let value = jailgun_post_json(client, server_url, "/mcp", token, &body)?;
    if let Some(error) = value.get("error") {
        let error = sanitize_jailgun_token_text(&error.to_string(), token);
        return Err(format!(
            "Jailgun MCP {name} returned JSON-RPC error: {error}"
        ));
    }
    let result = value.get("result").unwrap_or(&Value::Null);
    if result
        .get("isError")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        let result = sanitize_jailgun_token_text(&result.to_string(), token);
        return Err(format!(
            "Jailgun MCP {name} returned isError=true: {result}"
        ));
    }
    result
        .get("structuredContent")
        .cloned()
        .ok_or_else(|| format!("Jailgun MCP {name} response missing structuredContent: {value}"))
}

pub(crate) fn hard_backend_required(
    runbook: &Value,
    variant: &GenomeVariant,
    run_id: &str,
    max_generations: usize,
) -> bool {
    if !matches!(variant, GenomeVariant::Hybrid) {
        return false;
    }
    let evaluation = runbook
        .get("evaluation")
        .cloned()
        .unwrap_or_else(empty_object);
    let configured = evaluation
        .get("quality_gates")
        .and_then(|gates| gates.get("require_hard_backend"))
        .and_then(Value::as_bool)
        .or_else(|| {
            evaluation
                .get("routing")
                .and_then(|routing| routing.get("require_hard_backend"))
                .and_then(Value::as_bool)
        })
        .unwrap_or(false);
    configured
        || run_id == "hybrid-1000"
        || (run_id.starts_with("hybrid-v2") && max_generations >= 10)
}

pub(crate) fn strict_jailgun_required(
    variant: &GenomeVariant,
    run_id: &str,
    require_hard_backend: bool,
    hard_stages: usize,
) -> bool {
    matches!(variant, GenomeVariant::Hybrid)
        && run_id.starts_with("hybrid-v2")
        && require_hard_backend
        && hard_stages > 0
}

pub(crate) fn jailgun_available_from_environment() -> bool {
    env::var("JAILGUN_AVAILABLE")
        .ok()
        .map(|value| matches!(value.as_str(), "1" | "true" | "TRUE" | "yes" | "YES"))
        .unwrap_or(false)
}

pub(crate) fn resolve_jailgun_status(strict: bool, jailgun_available_flag: bool) -> JailgunStatus {
    resolve_jailgun_status_with_config(
        strict,
        jailgun_available_flag,
        JailgunHealthConfig::from_env(),
    )
}

pub(crate) fn resolve_jailgun_status_with_config(
    strict: bool,
    jailgun_available_flag: bool,
    config: JailgunHealthConfig,
) -> JailgunStatus {
    let mut status = jailgun_status_with_config(strict, config);
    if !strict
        && !status.available
        && (jailgun_available_flag || jailgun_available_from_environment())
    {
        status.push_error("Jailgun availability flag was set, but server readiness was not proven");
        status.finish();
    }
    status
}

pub(crate) fn strict_jailgun_status_with_config(config: JailgunHealthConfig) -> JailgunStatus {
    jailgun_status_with_config(true, config)
}

pub(crate) fn jailgun_status_with_config(
    strict: bool,
    config: JailgunHealthConfig,
) -> JailgunStatus {
    let mut status = JailgunStatus::server_authoritative(strict);
    status.server_url = Some(config.server_url.clone());
    status.token_source = config.token_source().map(ToString::to_string);
    status.set_check("server_url_configured", true);

    let Some(token) = config.token.as_ref() else {
        status.set_check("token_configured", false);
        status.set_check("server_health", false);
        status.set_check("browser_accounts", false);
        status.set_check("ready_accounts", false);
        status.set_check("mcp_initialize", false);
        status.set_check("mcp_tools_list", false);
        status.set_check("auth_status", false);
        status.set_check("scheduler_capacity", false);
        status.push_error(
            "Jailgun token is not available from env or matching local jailgun serve process",
        );
        status.finish();
        return status;
    };
    status.set_check("token_configured", true);

    match reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(JAILGUN_HEALTH_TIMEOUT_SECONDS))
        .build()
    {
        Ok(client) => {
            check_jailgun_server(
                &mut status,
                &client,
                &config.server_url,
                token.value.as_str(),
            );
            check_jailgun_browser_accounts(
                &mut status,
                &client,
                &config.server_url,
                token.value.as_str(),
                &config.account_override_ids,
            );
            check_jailgun_mcp_initialize(
                &mut status,
                &client,
                &config.server_url,
                token.value.as_str(),
            );
            let tools = check_jailgun_tools_list(
                &mut status,
                &client,
                &config.server_url,
                token.value.as_str(),
            );
            check_jailgun_auth_status(
                &mut status,
                &client,
                &config.server_url,
                token.value.as_str(),
                &tools,
            );
            check_jailgun_scheduler_status(
                &mut status,
                &client,
                &config.server_url,
                token.value.as_str(),
                &tools,
            );
        }
        Err(error) => {
            status.set_check("server_health", false);
            status.set_check("browser_accounts", false);
            status.set_check("ready_accounts", false);
            status.set_check("mcp_initialize", false);
            status.set_check("mcp_tools_list", false);
            status.set_check("auth_status", false);
            status.set_check("scheduler_capacity", false);
            status.push_error(format!("failed to build Jailgun HTTP client: {error}"));
        }
    }

    status.finish();
    status
}

pub(crate) fn check_jailgun_server(
    status: &mut JailgunStatus,
    client: &reqwest::blocking::Client,
    server_url: &str,
    token: &str,
) {
    match jailgun_get_json(client, server_url, "/api/health", token) {
        Ok(value) => {
            let ok = value.get("status").and_then(Value::as_str) == Some("ok");
            status.set_check("server_health", ok);
            if !ok {
                let value = sanitize_jailgun_token_text(&value.to_string(), token);
                status.push_error(format!(
                    "Jailgun /api/health returned unexpected body: {value}"
                ));
            }
        }
        Err(error) => {
            status.set_check("server_health", false);
            status.push_error(error);
        }
    }
}

pub(crate) fn check_jailgun_browser_accounts(
    status: &mut JailgunStatus,
    client: &reqwest::blocking::Client,
    server_url: &str,
    token: &str,
    account_override_ids: &[String],
) {
    match jailgun_get_json(client, server_url, "/api/browser/accounts", token) {
        Ok(value) => {
            let accounts = jailgun_accounts_from_response(&value);
            let ready_accounts = ready_jailgun_account_ids(&accounts);
            let selected_ready = if account_override_ids.is_empty() {
                status.account_source = Some("server:/api/browser/accounts".to_string());
                ready_accounts
            } else {
                status.account_source = Some("env:JAILGUN_ACCOUNT_IDS".to_string());
                ready_accounts
                    .into_iter()
                    .filter(|id| account_override_ids.iter().any(|requested| requested == id))
                    .collect()
            };
            status.account_ids = selected_ready.clone();
            status.ready_account_ids = selected_ready;
            status.set_check("browser_accounts", true);
            let ready_ok = !status.ready_account_ids.is_empty();
            status.set_check("ready_accounts", ready_ok);
            if !ready_ok {
                let source = status.account_source.as_deref().unwrap_or("server");
                status.push_error(format!(
                    "no ready/authenticated Jailgun account reported by /api/browser/accounts for {source}"
                ));
            }
        }
        Err(error) => {
            status.set_check("browser_accounts", false);
            status.set_check("ready_accounts", false);
            status.push_error(error);
        }
    }
}

pub(crate) fn jailgun_accounts_from_response(value: &Value) -> Vec<Value> {
    value
        .as_array()
        .cloned()
        .or_else(|| value.get("accounts").and_then(Value::as_array).cloned())
        .or_else(|| {
            value
                .get("data")
                .and_then(|data| data.get("accounts"))
                .and_then(Value::as_array)
                .cloned()
        })
        .or_else(|| {
            value
                .get("browser")
                .and_then(|browser| browser.get("accounts"))
                .and_then(Value::as_array)
                .cloned()
        })
        .unwrap_or_default()
}

pub(crate) fn ready_jailgun_account_ids(accounts: &[Value]) -> Vec<String> {
    accounts
        .iter()
        .filter(|account| jailgun_account_ready(account))
        .filter_map(jailgun_account_id)
        .collect()
}

pub(crate) fn jailgun_account_id(account: &Value) -> Option<String> {
    account
        .get("id")
        .or_else(|| account.get("account_id"))
        .or_else(|| account.get("accountId"))
        .and_then(Value::as_str)
        .map(ToString::to_string)
}

pub(crate) fn jailgun_account_ready(account: &Value) -> bool {
    account.get("ready").and_then(Value::as_bool) == Some(true)
        || account.get("authenticated").and_then(Value::as_bool) == Some(true)
        || account
            .get("status")
            .and_then(Value::as_str)
            .map(jailgun_ready_status)
            .unwrap_or(false)
        || account
            .get("auth_status")
            .and_then(Value::as_str)
            .map(jailgun_ready_status)
            .unwrap_or(false)
}

pub(crate) fn jailgun_ready_status(status: &str) -> bool {
    matches!(status, "ready" | "authenticated" | "active" | "ok")
}

pub(crate) fn check_jailgun_mcp_initialize(
    status: &mut JailgunStatus,
    client: &reqwest::blocking::Client,
    server_url: &str,
    token: &str,
) {
    let body = json!({
        "jsonrpc": "2.0",
        "id": "openqg-preflight-init",
        "method": "initialize",
        "params": {},
    });
    match jailgun_post_json(client, server_url, "/mcp", token, &body) {
        Ok(value) => {
            let ok = value.get("error").is_none() && value.get("result").is_some();
            status.set_check("mcp_initialize", ok);
            if !ok {
                let value = sanitize_jailgun_token_text(&value.to_string(), token);
                status.push_error(format!(
                    "Jailgun MCP initialize returned unexpected body: {value}"
                ));
            }
        }
        Err(error) => {
            status.set_check("mcp_initialize", false);
            status.push_error(error);
        }
    }
}

pub(crate) fn check_jailgun_tools_list(
    status: &mut JailgunStatus,
    client: &reqwest::blocking::Client,
    server_url: &str,
    token: &str,
) -> Vec<String> {
    let body = json!({
        "jsonrpc": "2.0",
        "id": "openqg-preflight-tools",
        "method": "tools/list",
        "params": {},
    });
    match jailgun_post_json(client, server_url, "/mcp", token, &body) {
        Ok(value) => {
            if let Some(error) = value.get("error") {
                status.set_check("mcp_tools_list", false);
                let error = sanitize_jailgun_token_text(&error.to_string(), token);
                status.push_error(format!("Jailgun MCP tools/list returned error: {error}"));
                return Vec::new();
            }
            let tools = jailgun_tool_names_from_list_response(&value);
            status.mcp_tools = tools.clone();
            status.set_check("mcp_tools_list", true);
            tools
        }
        Err(error) => {
            status.set_check("mcp_tools_list", false);
            status.push_error(error);
            Vec::new()
        }
    }
}

pub(crate) fn jailgun_tool_names_from_list_response(value: &Value) -> Vec<String> {
    value
        .get("result")
        .and_then(|result| result.get("tools"))
        .or_else(|| {
            value
                .get("result")
                .and_then(|result| result.get("structuredContent"))
                .and_then(|content| content.get("tools"))
        })
        .and_then(Value::as_array)
        .map(|tools| {
            tools
                .iter()
                .filter_map(|tool| tool.get("name").and_then(Value::as_str))
                .map(ToString::to_string)
                .collect()
        })
        .unwrap_or_default()
}

pub(crate) fn jailgun_tool_available(tools: &[String], name: &str) -> bool {
    tools.iter().any(|tool| tool == name)
}

pub(crate) fn check_jailgun_auth_status(
    status: &mut JailgunStatus,
    client: &reqwest::blocking::Client,
    server_url: &str,
    token: &str,
    tools: &[String],
) {
    if status.ready_account_ids.is_empty() {
        status.set_check("auth_status", false);
        status.push_error("no ready Jailgun account is available for auth_status");
        return;
    };

    if !jailgun_tool_available(tools, "jailgun.auth_status") {
        status.set_check("auth_status", true);
        return;
    }

    let mut authenticated = Vec::new();
    let mut errors = Vec::new();
    for account_id in status.ready_account_ids.clone() {
        match jailgun_mcp_tool_call(
            client,
            server_url,
            token,
            &format!("openqg-preflight-auth-{account_id}"),
            "jailgun.auth_status",
            json!({ "account_id": account_id }),
        ) {
            Ok(content) => {
                if jailgun_auth_status_ready(&content) {
                    authenticated.push(account_id);
                } else {
                    let content = sanitize_jailgun_token_text(&content.to_string(), token);
                    errors.push(format!(
                        "Jailgun auth_status for account returned unexpected body: {content}"
                    ));
                }
            }
            Err(error) => errors.push(error),
        }
    }
    let ok = !authenticated.is_empty();
    status.set_check("auth_status", ok);
    if ok {
        status.ready_account_ids = authenticated.clone();
        status.account_ids = authenticated;
    } else {
        for error in errors {
            status.push_error(error);
        }
    }
}

pub(crate) fn jailgun_auth_status_ready(content: &Value) -> bool {
    content.get("ready").and_then(Value::as_bool) == Some(true)
        || content.get("authenticated").and_then(Value::as_bool) == Some(true)
        || content
            .get("status")
            .and_then(Value::as_str)
            .map(jailgun_ready_status)
            .unwrap_or(false)
        || content
            .get("auth_status")
            .and_then(Value::as_str)
            .map(jailgun_ready_status)
            .unwrap_or(false)
}

pub(crate) fn check_jailgun_scheduler_status(
    status: &mut JailgunStatus,
    client: &reqwest::blocking::Client,
    server_url: &str,
    token: &str,
    tools: &[String],
) {
    if !jailgun_tool_available(tools, "jailgun.scheduler_status") {
        status.set_check("scheduler_capacity", true);
        return;
    }

    match jailgun_mcp_tool_call(
        client,
        server_url,
        token,
        "openqg-preflight-scheduler",
        "jailgun.scheduler_status",
        json!({}),
    ) {
        Ok(content) => {
            let ok = jailgun_scheduler_has_capacity(&content);
            status.set_check("scheduler_capacity", ok);
            if !ok {
                let content = sanitize_jailgun_token_text(&content.to_string(), token);
                status.push_error(format!(
                    "Jailgun scheduler has no available capacity: {content}"
                ));
            }
        }
        Err(error) => {
            status.set_check("scheduler_capacity", false);
            status.push_error(error);
        }
    }
}

pub(crate) fn jailgun_scheduler_has_capacity(content: &Value) -> bool {
    for key in [
        "queued_jobs",
        "running_jobs",
        "pending_jobs",
        "active_jobs",
        "queued",
        "running",
        "in_flight",
    ] {
        if let Some(value) = content.get(key).and_then(Value::as_i64) {
            if value > 0 {
                return false;
            }
        }
        if let Some(value) = content.get(key).and_then(Value::as_u64) {
            if value > 0 {
                return false;
            }
        }
        if let Some(items) = content.get(key).and_then(Value::as_array) {
            if !items.is_empty() {
                return false;
            }
        }
    }
    for key in [
        "capacity_available",
        "has_capacity",
        "available",
        "can_accept_runs",
        "can_schedule",
    ] {
        if let Some(value) = content.get(key).and_then(Value::as_bool) {
            return value;
        }
    }
    for key in [
        "available_slots",
        "free_slots",
        "remaining_capacity",
        "capacity",
    ] {
        if let Some(value) = content.get(key).and_then(Value::as_i64) {
            return value > 0;
        }
        if let Some(value) = content.get(key).and_then(Value::as_u64) {
            return value > 0;
        }
    }
    if let Some(status) = content.get("status").and_then(Value::as_str) {
        if matches!(
            status,
            "full" | "paused" | "blocked" | "unavailable" | "stopped" | "disabled" | "error"
        ) {
            return false;
        }
        if matches!(status, "ready" | "ok" | "available" | "healthy" | "running") {
            return true;
        }
    }
    true
}

pub(crate) fn jailgun_get_json(
    client: &reqwest::blocking::Client,
    server_url: &str,
    path: &str,
    token: &str,
) -> std::result::Result<Value, String> {
    let url = jailgun_url(server_url, path);
    let response = client
        .get(&url)
        .header("x-jailgun-token", token)
        .send()
        .map_err(|error| format!("Jailgun GET {url} failed: {error}"))?;
    response_json(response, &url, token)
}

pub(crate) fn jailgun_post_json(
    client: &reqwest::blocking::Client,
    server_url: &str,
    path: &str,
    token: &str,
    body: &Value,
) -> std::result::Result<Value, String> {
    let url = jailgun_url(server_url, path);
    let body = serde_json::to_string(body)
        .map_err(|error| format!("failed to serialize Jailgun POST body for {url}: {error}"))?;
    let response = client
        .post(&url)
        .header("x-jailgun-token", token)
        .header("content-type", "application/json")
        .body(body)
        .send()
        .map_err(|error| format!("Jailgun POST {url} failed: {error}"))?;
    response_json(response, &url, token)
}

pub(crate) fn response_json(
    response: reqwest::blocking::Response,
    url: &str,
    token: &str,
) -> std::result::Result<Value, String> {
    let status = response.status();
    let text = response
        .text()
        .map_err(|error| format!("Jailgun response body from {url} failed: {error}"))?;
    let text = sanitize_jailgun_token_text(&text, token);
    if !status.is_success() {
        return Err(format!("Jailgun {url} returned HTTP {status}: {text}"));
    }
    serde_json::from_str(&text)
        .map_err(|error| format!("Jailgun {url} returned invalid JSON: {error}: {text}"))
}
