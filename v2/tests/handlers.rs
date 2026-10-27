use serde_json::{json, Value};
use std::sync::Mutex;
use tezbar_epoch_to_timestamp_v2::{handle, Host, HostFuture, Result};

type Call = (String, Vec<Value>);
struct FakeHost {
    text: Mutex<Value>,
    calls: Mutex<Vec<Call>>,
    failure: Mutex<Option<(String, Result)>>,
}
impl FakeHost {
    fn new(text: &str) -> Self {
        Self {
            text: Mutex::new(json!(text)),
            calls: Mutex::new(vec![]),
            failure: Mutex::new(None),
        }
    }
    fn fail(&self, route: &str, result: Result) {
        *self.failure.lock().unwrap() = Some((route.into(), result));
    }
    fn calls(&self) -> Vec<Call> {
        self.calls.lock().unwrap().clone()
    }
}
impl Host for FakeHost {
    fn request<'a>(&'a self, route: &'a str, args: Vec<Value>) -> HostFuture<'a> {
        Box::pin(async move {
            self.calls.lock().unwrap().push((route.into(), args));
            if let Some((failure_route, result)) = self.failure.lock().unwrap().clone() {
                if failure_route == route {
                    return result;
                }
            }
            match route {
                "clipboard:read" => Ok(self.text.lock().unwrap().clone()),
                "clipboard:write" | "window:hide" => Ok(json!({"ok":true})),
                _ => panic!("Invented host route: {route}"),
            }
        })
    }
}
async fn run(host: &FakeHost, state: &mut Value) -> Value {
    handle(host, "epoch", "extension:run-command", &json!({}), state)
        .await
        .unwrap()
}
async fn invoke(host: &FakeHost, state: &mut Value, id: &Value) -> Result {
    // Forged text must never replace the authoritative copy target.
    handle(
        host,
        "epoch",
        "extension:invoke-action",
        &json!({"actionId":id,"text":"forged"}),
        state,
    )
    .await
}
fn action(view: &Value, index: usize) -> Value {
    view["actions"][index]["id"].clone()
}

#[tokio::test]
async fn detail_copy_close_refresh_and_session_lifecycle() {
    let host = FakeHost::new("1609459200");
    let mut state = json!({});
    let view = run(&host, &mut state).await;
    assert_eq!(view["ok"], true);
    assert_eq!(view["mode"], "view");
    assert_eq!(view["title"], "Epoch");
    assert_eq!(view["root"]["type"], "Detail");
    assert_eq!(
        view["root"]["props"]["actionIds"],
        json!([action(&view, 0), action(&view, 1)])
    );
    assert_eq!(view["actions"][0]["title"], "Copy to clipboard");
    assert_eq!(view["actions"][1]["title"], "Close");
    let text = state["text"].as_str().unwrap().to_owned();
    assert!(text.contains("2021"));
    assert_eq!(
        view["root"]["props"]["markdown"],
        format!("```text\n{text}\n```")
    );
    *host.text.lock().unwrap() = json!("changed clipboard");
    let refresh = handle(
        &host,
        "epoch",
        "extension:refresh-session",
        &json!({}),
        &mut state,
    )
    .await
    .unwrap();
    assert_eq!(refresh, view);
    assert_eq!(host.calls(), vec![("clipboard:read".into(), vec![])]);
    let copied = invoke(&host, &mut state, &action(&view, 0)).await.unwrap();
    assert_eq!(copied, view);
    assert_eq!(
        host.calls()[1],
        ("clipboard:write".into(), vec![json!(text)])
    );
    let closed = invoke(&host, &mut state, &action(&view, 1)).await.unwrap();
    assert_eq!(closed, json!({"ok":true,"mode":"no-view","message":""}));
    assert_eq!(host.calls()[2], ("window:hide".into(), vec![]));
    assert!(invoke(&host, &mut state, &action(&view, 0)).await.is_err());
    assert!(handle(
        &host,
        "epoch",
        "extension:refresh-session",
        &json!({}),
        &mut state
    )
    .await
    .is_err());
    let disposed = handle(
        &host,
        "epoch",
        "extension:dispose-session",
        &json!({}),
        &mut state,
    )
    .await
    .unwrap();
    assert_eq!(disposed, json!({"ok":true,"mode":"unchanged"}));
    assert!(state.get("text").is_none());
}

#[tokio::test]
async fn invalid_date_is_displayed_and_copyable_but_nan_is_an_error() {
    for input in [
        "Infinity",
        "-Infinity",
        "1e309",
        "8640000000000001",
        "-8640000000000000",
    ] {
        let host = FakeHost::new(input);
        let mut state = json!({});
        let view = run(&host, &mut state).await;
        assert_eq!(state["text"], "Invalid Date");
        invoke(&host, &mut state, &action(&view, 0)).await.unwrap();
        assert_eq!(
            host.calls()[1],
            ("clipboard:write".into(), vec![json!("Invalid Date")])
        );
    }
    for input in ["NaN", "not a number", "0b12", "+0x10"] {
        let host = FakeHost::new(input);
        let mut state = json!({});
        assert_eq!(
            handle(
                &host,
                "epoch",
                "extension:run-command",
                &json!({}),
                &mut state
            )
            .await
            .unwrap_err(),
            "No number in the clipboard."
        );
        assert_eq!(host.calls().len(), 1);
        assert_ne!(state["active"], true);
    }
    let host = FakeHost::new("");
    let mut state = json!({});
    run(&host, &mut state).await;
    assert_eq!(
        state["text"],
        tezbar_epoch_to_timestamp_v2::local::format_local(Some(0)).unwrap()
    );
}

#[tokio::test]
async fn unknown_forged_stale_disposed_and_failed_relaunch_actions_are_rejected() {
    let host = FakeHost::new("0");
    let mut state = json!({});
    let first = run(&host, &mut state).await;
    for id in [json!("clipboard:write"), json!("epoch:0:copy"), Value::Null] {
        assert!(invoke(&host, &mut state, &id).await.is_err());
    }
    for operation in [
        "run-command",
        "extension:search-text-changed",
        "extension:load-more",
        "invented",
    ] {
        assert!(handle(&host, "epoch", operation, &json!({}), &mut state)
            .await
            .is_err());
    }
    assert!(handle(
        &host,
        "wrong",
        "extension:run-command",
        &json!({}),
        &mut state
    )
    .await
    .is_err());
    assert_eq!(host.calls().len(), 1);
    let second = run(&host, &mut state).await;
    assert_ne!(action(&first, 0), action(&second, 0));
    assert!(invoke(&host, &mut state, &action(&first, 0)).await.is_err());
    handle(
        &host,
        "epoch",
        "extension:dispose-session",
        &json!({}),
        &mut state,
    )
    .await
    .unwrap();
    assert!(invoke(&host, &mut state, &action(&second, 0))
        .await
        .is_err());
    let third = run(&host, &mut state).await;
    *host.text.lock().unwrap() = json!("bad");
    assert!(handle(
        &host,
        "epoch",
        "extension:run-command",
        &json!({}),
        &mut state
    )
    .await
    .is_err());
    assert!(invoke(&host, &mut state, &action(&third, 0)).await.is_err());
    assert_eq!(
        host.calls()
            .iter()
            .filter(|(r, _)| r != "clipboard:read")
            .count(),
        0
    );
}

#[tokio::test]
async fn host_err_and_failure_envelopes_never_report_success_or_destroy_retry_target() {
    for failure in [
        Err("denied".into()),
        Ok(json!({"ok":false,"message":"denied"})),
        Ok(json!({"ok":false,"error":"denied"})),
    ] {
        for route in ["clipboard:read", "clipboard:write", "window:hide"] {
            let host = FakeHost::new("0");
            let mut state = json!({});
            if route == "clipboard:read" {
                host.fail(route, failure.clone());
                assert_eq!(
                    handle(
                        &host,
                        "epoch",
                        "extension:run-command",
                        &json!({}),
                        &mut state
                    )
                    .await
                    .unwrap_err(),
                    "denied"
                );
                assert_ne!(state["active"], true);
                assert_eq!(host.calls().len(), 1);
            } else {
                let view = run(&host, &mut state).await;
                let before = state.clone();
                host.fail(route, failure.clone());
                let id = action(&view, usize::from(route == "window:hide"));
                assert_eq!(invoke(&host, &mut state, &id).await.unwrap_err(), "denied");
                assert_eq!(state, before);
                assert_eq!(host.calls().len(), 2);
                *host.failure.lock().unwrap() = None;
                invoke(&host, &mut state, &id).await.unwrap();
            }
        }
    }
    let host = FakeHost::new("0");
    *host.text.lock().unwrap() = Value::Null;
    assert_eq!(
        handle(
            &host,
            "epoch",
            "extension:run-command",
            &json!({}),
            &mut json!({})
        )
        .await
        .unwrap_err(),
        "Clipboard did not return text."
    );
}
