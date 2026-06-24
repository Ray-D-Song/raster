use crate::bridge::value::BridgeValue;

/// Unified bridge message envelope.
#[derive(Debug, Clone, PartialEq)]
pub enum BridgeEnvelope {
    Call {
        id: u64,
        channel: String,
        method: String,
        payload: BridgeValue,
    },
    Reply {
        id: u64,
        ok: bool,
        payload: BridgeValue,
        error: Option<String>,
    },
    Event {
        channel: String,
        name: String,
        payload: BridgeValue,
    },
}

impl BridgeEnvelope {
    pub fn event(channel: impl Into<String>, name: impl Into<String>, payload: BridgeValue) -> Self {
        Self::Event {
            channel: channel.into(),
            name: name.into(),
            payload,
        }
    }

    pub fn reply_ok(id: u64, payload: BridgeValue) -> Self {
        Self::Reply {
            id,
            ok: true,
            payload,
            error: None,
        }
    }

    pub fn reply_err(id: u64, error: impl Into<String>) -> Self {
        Self::Reply {
            id,
            ok: false,
            payload: BridgeValue::Null,
            error: Some(error.into()),
        }
    }
}