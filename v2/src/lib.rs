//! Compiled-in Rust port of the MIT Epoch to Timestamp extension. See LICENSE.
pub mod epoch;
pub mod local;

use serde_json::{json, Value};
use std::{future::Future, pin::Pin};

pub const ICON: &[u8] = include_bytes!("../assets/extension-icon.png");
pub const MANIFEST: &str = include_str!("../manifest.json");
pub type Result = std::result::Result<Value, String>;
pub type HostFuture<'a> = Pin<Box<dyn Future<Output = Result> + Send + 'a>>;

/// Request routes and positional arguments are the pinned beta CoreContext ABI.
pub trait Host: Sync {
    fn request<'a>(&'a self, route: &'a str, args: Vec<Value>) -> HostFuture<'a>;
}

pub fn commands() -> &'static [&'static str] {
    &["epoch"]
}

async fn effect(host: &impl Host, route: &str, args: Vec<Value>) -> Result {
    let value = host.request(route, args).await?;
    if value["ok"] == false {
        return Err(value["message"]
            .as_str()
            .or_else(|| value["error"].as_str())
            .unwrap_or("Host operation failed")
            .to_owned());
    }
    Ok(value)
}

fn revision(state: &Value) -> std::result::Result<u64, String> {
    state["revision"]
        .as_u64()
        .unwrap_or(0)
        .checked_add(1)
        .ok_or_else(|| "Epoch session revision exhausted".into())
}

fn view(state: &Value) -> Value {
    let revision = state["revision"].as_u64().unwrap_or(0);
    let copy = format!("epoch:{revision}:copy");
    let close = format!("epoch:{revision}:close");
    json!({"ok":true,"mode":"view","title":"Epoch","root":{"type":"Detail","props":{
        "markdown":format!("```text\n{}\n```", state["text"].as_str().unwrap_or("")),
        "actionIds":[copy,close]}},"actions":[
        {"id":copy,"title":"Copy to clipboard","kind":"copy"},
        {"id":close,"title":"Close","kind":"action"}]})
}

/// Actual beta operation names; state is per native session, owned by the host.
pub async fn handle(
    host: &impl Host,
    command: &str,
    operation: &str,
    payload: &Value,
    state: &mut Value,
) -> Result {
    if !commands().contains(&command) {
        return Err("Unknown Epoch command".into());
    }
    if operation == "extension:dispose-session" {
        *state = json!({"revision":revision(state)?});
        return Ok(json!({"ok":true,"mode":"unchanged"}));
    }
    if operation == "extension:run-command" {
        // Invalidate old actions even when a relaunch fails before producing a view.
        *state = json!({"revision":revision(state)?});
        let input = effect(host, "clipboard:read", vec![]).await?;
        let input = input.as_str().ok_or("Clipboard did not return text.")?;
        let millis = epoch::epoch_millis(input)?;
        let text = local::format_local(millis)?;
        state["text"] = json!(text);
        state["active"] = json!(true);
        return Ok(view(state));
    }
    if state["active"] != true {
        return Err("Epoch session is not active".into());
    }
    match operation {
        "extension:refresh-session" => Ok(view(state)),
        "extension:invoke-action" => {
            let id = payload["actionId"].as_str().ok_or("Missing action ID")?;
            let revision = state["revision"].as_u64().ok_or("Invalid Epoch session")?;
            if id == format!("epoch:{revision}:copy") {
                let text = state["text"].as_str().ok_or("Invalid Epoch copy target")?;
                effect(host, "clipboard:write", vec![json!(text)]).await?;
                Ok(view(state))
            } else if id == format!("epoch:{revision}:close") {
                effect(host, "window:hide", vec![]).await?;
                state["active"] = json!(false);
                Ok(json!({"ok":true,"mode":"no-view","message":""}))
            } else {
                Err("Unknown or stale Epoch action".into())
            }
        }
        _ => Err("Unsupported Epoch operation".into()),
    }
}
