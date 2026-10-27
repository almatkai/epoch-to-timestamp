//! Compile this unchanged as native_core/extensions/v2_epoch_to_timestamp.rs.
use crate::native_core::common::{CoreContext, CoreResult};
use serde_json::Value;
use tezbar_epoch_to_timestamp_v2::{Host, HostFuture};

struct TezbarHost<'a>(&'a CoreContext);

impl Host for TezbarHost<'_> {
    fn request<'a>(&'a self, route: &'a str, args: Vec<Value>) -> HostFuture<'a> {
        Box::pin(self.0.request(route, args))
    }
}

pub fn commands() -> &'static [&'static str] {
    tezbar_epoch_to_timestamp_v2::commands()
}

pub const ICON: &[u8] = tezbar_epoch_to_timestamp_v2::ICON;
pub const MANIFEST: &str = tezbar_epoch_to_timestamp_v2::MANIFEST;

pub async fn handle(
    ctx: &CoreContext,
    command: &str,
    operation: &str,
    payload: &Value,
    state: &mut Value,
) -> CoreResult {
    tezbar_epoch_to_timestamp_v2::handle(&TezbarHost(ctx), command, operation, payload, state).await
}
