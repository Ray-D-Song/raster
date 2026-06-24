use crate::bridge::state::SharedBridgeState;
use crate::bridge::value::BridgeValue;
use crate::common::utils::logger;

pub fn handle_assets_call(
    bridge: &SharedBridgeState,
    method: &str,
    payload: BridgeValue,
) -> anyhow::Result<BridgeValue> {
    match method {
        "load" => {
            let uri = payload
                .get_str("uri")
                .ok_or_else(|| anyhow::anyhow!("host.assets.load requires uri"))?;
            let bytes = payload
                .get_bytes("bytes")
                .ok_or_else(|| anyhow::anyhow!("host.assets.load requires bytes"))?;
            let assets = bridge.assets();
            let mut store = assets
                .lock()
                .map_err(|_| anyhow::anyhow!("asset store lock poisoned"))?;
            store.load_image(uri, bytes).map_err(|error| {
                logger::error(format!("host.assets.load failed uri={uri}: {error:#}"));
                error
            })?;
            Ok(BridgeValue::Null)
        }
        "remove" => {
            let uri = payload
                .get_str("uri")
                .ok_or_else(|| anyhow::anyhow!("host.assets.remove requires uri"))?;
            let assets = bridge.assets();
            let mut store = assets
                .lock()
                .map_err(|_| anyhow::anyhow!("asset store lock poisoned"))?;
            store.remove(uri);
            Ok(BridgeValue::Null)
        }
        "stats" => {
            let assets = bridge.assets();
            let store = assets
                .lock()
                .map_err(|_| anyhow::anyhow!("asset store lock poisoned"))?;
            let stats = store.stats();
            Ok(BridgeValue::object([
                ("count", BridgeValue::number(stats.count as f64)),
                ("totalBytes", BridgeValue::number(stats.total_bytes as f64)),
                ("maxBytes", BridgeValue::number(stats.max_bytes as f64)),
            ]))
        }
        other => anyhow::bail!("unknown host.assets method: {other}"),
    }
}