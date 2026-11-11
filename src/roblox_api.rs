use crate::links::{self, PrivateServer};
use serde_json::{Value, json};
use std::sync::OnceLock;
use std::time::Duration;

const SESSION_EXPIRED: &str = "Session expired. Remove the account and add it again in welffi accounts.";
const REFERER: &str = "https://www.roblox.com/";

#[derive(Default)]
struct Call<'a> {
    post: bool,
    url: &'a str,
    cookie: Option<&'a str>,
    body: Option<String>,
    csrf: Option<String>,
    referer: bool,
}

struct Reply {
    status: u16,
    csrf: Option<String>,
    ticket: Option<String>,
    body: String,
}

fn agent() -> &'static ureq::Agent {
    static AGENT: OnceLock<ureq::Agent> = OnceLock::new();
    AGENT.get_or_init(|| {
        ureq::Agent::config_builder()
            .timeout_global(Some(Duration::from_secs(15)))
            .http_status_as_error(false)
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64)")
            .build()
            .into()
    })
}

fn content_type(body: Option<&str>) -> &'static str {
    if body.is_some() { "application/json; charset=utf-8" } else { "text/plain; charset=utf-8" }
}

fn headers<B>(mut request: ureq::RequestBuilder<B>, call: &Call) -> ureq::RequestBuilder<B> {
    if let Some(cookie) = call.cookie {
        request = request.header("Cookie", format!(".ROBLOSECURITY={cookie}"));
    }
    if let Some(csrf) = &call.csrf {
        request = request.header("X-CSRF-TOKEN", csrf);
    }
    if call.referer {
        request = request.header("Referer", REFERER);
    }
    request
}

fn send(call: &Call) -> Result<Reply, String> {
    let response = if call.post {
        let body = call.body.as_deref();
        headers(agent().post(call.url), call).content_type(content_type(body)).send(body.unwrap_or(""))
    } else {
        headers(agent().get(call.url), call).call()
    };
    let mut response = response.map_err(|error| format!("Couldn't reach Roblox ({error})"))?;
    let header = |name: &str| response.headers().get(name).and_then(|value| value.to_str().ok()).map(str::to_string);
    let (csrf, ticket) = (header("x-csrf-token"), header("rbx-authentication-ticket"));
    let status = response.status().as_u16();
    let body = response.body_mut().read_to_string().unwrap_or_default();
    Ok(Reply { status, csrf, ticket, body })
}

fn post_with_csrf(url: &str, cookie: &str, body: Value) -> Result<Reply, String> {
    let mut call = Call { post: true, url, cookie: Some(cookie), body: Some(body.to_string()), referer: true, ..Call::default() };
    let first = send(&call)?;
    if first.status != 403 {
        return Ok(first);
    }
    call.csrf = first.csrf;
    send(&call)
}

fn find<'a>(value: &'a Value, key: &str) -> Option<&'a Value> {
    match value {
        Value::Object(map) => map.get(key).or_else(|| map.values().find_map(|inner| find(inner, key))),
        Value::Array(items) => items.iter().find_map(|inner| find(inner, key)),
        _ => None,
    }
}

fn number_text(value: &Value) -> Option<String> {
    match value {
        Value::Number(number) => Some(number.to_string()),
        Value::String(text) if !text.is_empty() && text.chars().all(|c| c.is_ascii_digit()) => Some(text.clone()),
        _ => None,
    }
}

fn parse(body: &str) -> Value {
    serde_json::from_str(body).unwrap_or(Value::Null)
}

pub fn auth_ticket(cookie: &str) -> Result<String, String> {
    let url = "https://auth.roblox.com/v1/authentication-ticket/";
    let mut call = Call { post: true, url, cookie: Some(cookie), referer: true, ..Call::default() };
    let first = send(&call)?;
    if first.status == 401 {
        return Err(SESSION_EXPIRED.to_string());
    }
    call.csrf = Some(first.csrf.filter(|token| !token.is_empty()).ok_or(format!("No CSRF token from Roblox ({})", first.status))?);
    let second = send(&call)?;
    second.ticket.filter(|ticket| !ticket.is_empty()).ok_or(format!("No auth ticket from Roblox ({})", second.status))
}

pub fn resolve_private_server(cookie: &str, link: &str) -> Result<PrivateServer, String> {
    if let Some(direct) = links::parse_direct(link) {
        return Ok(direct);
    }
    let code = links::share_code(link).ok_or("That isn't a private server link")?;
    let reply = post_with_csrf("https://apis.roblox.com/sharelinks/v1/resolve-link", cookie, json!({ "linkId": code, "linkType": "Server" }))?;
    match reply.status {
        200 => {}
        401 => return Err(SESSION_EXPIRED.to_string()),
        status => return Err(format!("Private server link lookup failed ({status})")),
    }
    let value = parse(&reply.body);
    let place_id = find(&value, "placeId").and_then(number_text);
    let code = find(&value, "linkCode").and_then(Value::as_str).filter(|code| !code.is_empty());
    match (place_id, code) {
        (Some(place_id), Some(code)) => Ok(PrivateServer { place_id, code: code.to_string() }),
        _ => Err("That private server link is invalid or expired".to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_nested_keys() {
        let value = parse(r#"{"privateServerInviteData":{"placeId":123,"linkCode":"abc"}}"#);
        assert_eq!(find(&value, "placeId").and_then(number_text), Some("123".into()));
        assert_eq!(find(&value, "linkCode").and_then(Value::as_str), Some("abc"));
        assert_eq!(find(&value, "missing"), None);
    }

    #[test]
    fn empty_posts_get_a_content_type() {
        assert_eq!(content_type(None), "text/plain; charset=utf-8");
        assert_eq!(content_type(Some("{}")), "application/json; charset=utf-8");
    }
}
