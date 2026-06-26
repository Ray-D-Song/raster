pub mod ffi;
pub mod host;
pub mod json;

pub use ffi::{PluginHostState, install_plugin_host};
pub use host::{PluginHost, handle_plugin_invoke};

#[cfg(test)]
mod tests {
    use crate::bridge::{BridgeState, new_asset_store};
    use crate::plugin::host::register_echo_plugin;

    use super::*;

    #[test]
    fn echo_plugin_replies_with_message() {
        let bridge = BridgeState::new(new_asset_store());
        let host_state = install_plugin_host(bridge.clone());
        register_echo_plugin(host_state.host());

        host_state
            .host()
            .invoke(
                1,
                "Echo",
                "echo",
                crate::bridge::BridgeValue::object([(
                    "msg",
                    crate::bridge::BridgeValue::string("hi"),
                )]),
            )
            .expect("invoke echo");

        let replies: Vec<_> = bridge
            .drain_egress()
            .into_iter()
            .filter_map(|envelope| match envelope {
                crate::bridge::BridgeEnvelope::Reply { id, ok, payload, .. } if ok => {
                    Some((id, payload))
                }
                _ => None,
            })
            .collect();
        assert_eq!(replies.len(), 1);
        assert_eq!(replies[0].0, 1);
        assert_eq!(
            replies[0].1.get_str("echo"),
            Some("hi")
        );
    }
}