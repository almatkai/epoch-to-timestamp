//! The UNMODIFIED production wrapper compiled against the actual beta common.rs.
//! Substitute only Tauri AppHandle/Emitter; the CoreContext/router ABI is real.
extern crate self as tauri;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct AppHandle;
pub trait Emitter {
    fn emit(&self, channel: &str, payload: Value) -> std::result::Result<(), String>;
}
impl Emitter for AppHandle {
    fn emit(&self, _: &str, _: Value) -> std::result::Result<(), String> {
        panic!("No real Tauri app in adapter tests")
    }
}
mod native_core {
    #[allow(dead_code, clippy::all)]
    pub mod common {
        include!("fixtures/beta_common.rs");
    }
}
#[path = "../tezbar_adapter.rs"]
mod adapter;

#[test]
fn exact_beta_manifest_schema_v1_metadata_and_pinned_original_icon() {
    let schema: Value =
        serde_json::from_str(include_str!("fixtures/extension-v2.schema.json")).unwrap();
    let manifest: Value = serde_json::from_str(adapter::MANIFEST).unwrap();
    jsonschema::validator_for(&schema)
        .unwrap()
        .validate(&manifest)
        .unwrap();
    assert_eq!(manifest["id"], "com.tezbar.native.epoch-to-timestamp");
    assert_eq!(manifest["replaces"], json!(["raycast.epoch-to-timestamp"]));
    assert_eq!(manifest["icon"], "/extensions/icons/epoch-to-timestamp.png");
    assert_eq!(adapter::commands(), &["epoch"]);
    assert_eq!(adapter::ICON.len(), 27780);
    assert_eq!(
        format!("{:x}", Sha256::digest(adapter::ICON)),
        "862380aeedf7c34dc1fdba7f1529e1694380243c5b0d4609e2e20111f5d12c5f"
    );
    assert_eq!(
        format!(
            "{:x}",
            Sha256::digest(include_bytes!("fixtures/beta_common.rs"))
        ),
        "280a0a2f0dd04a8570b320aafc0c3ece0f5de8ddb21c363645567d6a35d84da7"
    );
    assert_eq!(
        format!(
            "{:x}",
            Sha256::digest(include_bytes!("fixtures/extension-v2.schema.json"))
        ),
        "6d5afda1b652605982104bdf16d1547547b98dfd707a7b2969316406c49f32af"
    );
    let v1: Value = serde_json::from_str(include_str!("fixtures/v1-package.json")).unwrap();
    assert_eq!(manifest["platforms"], v1["platforms"]);
    assert_eq!(
        manifest["preferences"],
        v1.get("preferences").cloned().unwrap_or(json!([]))
    );
    assert_eq!(
        manifest["commands"].as_array().unwrap().len(),
        v1["commands"].as_array().unwrap().len()
    );
    for (native, original) in manifest["commands"]
        .as_array()
        .unwrap()
        .iter()
        .zip(v1["commands"].as_array().unwrap())
    {
        for key in ["name", "title", "description", "mode", "platforms"] {
            assert_eq!(native[key], original[key]);
        }
        assert_eq!(native["runtime"], "rust");
    }
    for credit in ["almatkai", "bgreenlee"] {
        assert!(manifest["description"].as_str().unwrap().contains(credit));
    }
    assert_eq!(manifest["verification"].as_array().unwrap().len(), 2);
    for (entry, platform) in manifest["verification"]
        .as_array()
        .unwrap()
        .iter()
        .zip(["macOS", "Windows"])
    {
        assert_eq!(entry["platform"], platform);
        assert_eq!(entry["state"], "not-run");
    }
    // Optional surrounding checkout assertions; all required fixtures are packaged.
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    if root.join("package.json").is_file() {
        let actual: Value =
            serde_json::from_slice(&std::fs::read(root.join("package.json")).unwrap()).unwrap();
        assert_eq!(actual, v1);
        assert_eq!(
            std::fs::read(root.join("assets/extension-icon.png")).unwrap(),
            adapter::ICON
        );
        let oracle: Value = serde_json::from_str(include_str!("fixtures/v1-vectors.json")).unwrap();
        for path in ["src/lib/epoch.ts", "src/epoch.tsx"] {
            assert_eq!(
                format!(
                    "{:x}",
                    Sha256::digest(std::fs::read(root.join(path)).unwrap())
                ),
                oracle["sourceHashes"][path]
            );
        }
    }
}

#[tokio::test]
async fn concrete_wrapper_real_beta_corecontext_recording_router() {
    use native_core::common::{CoreContext, CorePaths};
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("target/adapter-fixture");
    let ctx = CoreContext::headless(CorePaths {
        data_dir: root.join("data"),
        config_dir: root.join("config"),
        resource_dir: root.join("resources"),
        temp_dir: root.join("tmp"),
        home_dir: root,
    });
    let calls = Arc::new(Mutex::new(vec![]));
    let log = calls.clone();
    let failure = Arc::new(Mutex::new(None::<Value>));
    let fail = failure.clone();
    ctx.set_router(Arc::new(move |route, args| {
        let log = log.clone();
        let fail = fail.clone();
        Box::pin(async move {
            log.lock().unwrap().push((route.clone(), args.clone()));
            match route.as_str() {
                "clipboard:read" => {
                    assert!(args.is_empty());
                    Ok(json!("Infinity"))
                }
                "clipboard:write" => {
                    assert_eq!(args, vec![json!("Invalid Date")]);
                    Ok(fail.lock().unwrap().clone().unwrap_or(json!({"ok":true})))
                }
                "window:hide" => {
                    assert!(args.is_empty());
                    Ok(json!({"ok":true}))
                }
                _ => Err(format!("Unexpected route {route}")),
            }
        })
    }))
    .unwrap();
    let mut state = json!({});
    let view = adapter::handle(
        &ctx,
        "epoch",
        "extension:run-command",
        &json!({}),
        &mut state,
    )
    .await
    .unwrap();
    assert_eq!(view["root"]["type"], "Detail");
    assert_eq!(
        adapter::handle(
            &ctx,
            "epoch",
            "extension:refresh-session",
            &json!({}),
            &mut state
        )
        .await
        .unwrap(),
        view
    );
    let copy = json!({"actionId":view["actions"][0]["id"]});
    *failure.lock().unwrap() = Some(json!({"ok":false,"error":"clipboard locked"}));
    assert_eq!(
        adapter::handle(&ctx, "epoch", "extension:invoke-action", &copy, &mut state)
            .await
            .unwrap_err(),
        "clipboard locked"
    );
    *failure.lock().unwrap() = None;
    adapter::handle(&ctx, "epoch", "extension:invoke-action", &copy, &mut state)
        .await
        .unwrap();
    adapter::handle(
        &ctx,
        "epoch",
        "extension:invoke-action",
        &json!({"actionId":view["actions"][1]["id"]}),
        &mut state,
    )
    .await
    .unwrap();
    adapter::handle(
        &ctx,
        "epoch",
        "extension:dispose-session",
        &json!({}),
        &mut state,
    )
    .await
    .unwrap();
    assert!(
        adapter::handle(&ctx, "epoch", "extension:invoke-action", &copy, &mut state)
            .await
            .is_err()
    );
    assert_eq!(
        calls.lock().unwrap().as_slice(),
        &[
            ("clipboard:read".into(), vec![]),
            ("clipboard:write".into(), vec![json!("Invalid Date")]),
            ("clipboard:write".into(), vec![json!("Invalid Date")]),
            ("window:hide".into(), vec![])
        ]
    );
}
