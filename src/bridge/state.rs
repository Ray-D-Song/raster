use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use crate::bridge::assets::SharedAssetStore;
use crate::bridge::envelope::BridgeEnvelope;
use crate::common::channel::{ChannelReceiver, ChannelSender, NoopWakeSignal, WakeSignal, channel};

#[derive(Debug)]
struct BridgeQueue {
    sender: ChannelSender<BridgeEnvelope>,
    receiver: ChannelReceiver<BridgeEnvelope>,
}

impl BridgeQueue {
    fn new() -> Self {
        let (sender, receiver) = channel();
        Self { sender, receiver }
    }

    fn send(&self, envelope: BridgeEnvelope) -> anyhow::Result<()> {
        self.sender
            .send(envelope)
            .map_err(|_| anyhow::anyhow!("bridge queue receiver dropped"))
    }

    fn drain(&self) -> Vec<BridgeEnvelope> {
        self.receiver.drain()
    }

    fn try_recv(&self) -> Result<BridgeEnvelope, std::sync::mpsc::TryRecvError> {
        self.receiver.try_recv()
    }
}

/// Shared bridge state between the JS runtime thread and the GPUI host thread.
pub struct BridgeState {
    ingress: Mutex<BridgeQueue>,
    egress: Mutex<BridgeQueue>,
    assets: SharedAssetStore,
    next_call_id: AtomicU64,
    host_wake: Mutex<Arc<dyn WakeSignal>>,
    js_wake: Mutex<Arc<dyn WakeSignal>>,
}

pub type SharedBridgeState = Arc<BridgeState>;

impl BridgeState {
    pub fn new(assets: SharedAssetStore) -> SharedBridgeState {
        Arc::new(Self {
            ingress: Mutex::new(BridgeQueue::new()),
            egress: Mutex::new(BridgeQueue::new()),
            assets,
            next_call_id: AtomicU64::new(1),
            host_wake: Mutex::new(Arc::new(NoopWakeSignal)),
            js_wake: Mutex::new(Arc::new(NoopWakeSignal)),
        })
    }

    pub fn assets(&self) -> SharedAssetStore {
        self.assets.clone()
    }

    pub fn next_call_id(&self) -> u64 {
        self.next_call_id.fetch_add(1, Ordering::Relaxed)
    }

    pub fn set_host_wake(&self, wake: Arc<dyn WakeSignal>) {
        if let Ok(mut current) = self.host_wake.lock() {
            *current = wake;
        }
    }

    pub fn set_js_wake(&self, wake: Arc<dyn WakeSignal>) {
        if let Ok(mut current) = self.js_wake.lock() {
            *current = wake;
        }
    }

    pub fn send_ingress(&self, envelope: BridgeEnvelope) -> anyhow::Result<()> {
        self.ingress
            .lock()
            .map_err(|_| anyhow::anyhow!("bridge ingress lock poisoned"))?
            .send(envelope)?;
        self.host_wake().wake();
        Ok(())
    }

    pub fn send_egress(&self, envelope: BridgeEnvelope) -> anyhow::Result<()> {
        self.egress
            .lock()
            .map_err(|_| anyhow::anyhow!("bridge egress lock poisoned"))?
            .send(envelope)?;
        self.js_wake().wake();
        Ok(())
    }

    pub fn drain_ingress(&self) -> Vec<BridgeEnvelope> {
        self.ingress
            .lock()
            .map(|queue| queue.drain())
            .unwrap_or_default()
    }

    pub fn drain_egress(&self) -> Vec<BridgeEnvelope> {
        self.egress
            .lock()
            .map(|queue| queue.drain())
            .unwrap_or_default()
    }

    pub fn try_recv_egress(&self) -> Option<BridgeEnvelope> {
        self.egress
            .lock()
            .ok()?
            .try_recv()
            .ok()
    }

    fn host_wake(&self) -> Arc<dyn WakeSignal> {
        self.host_wake
            .lock()
            .map(|wake| wake.clone())
            .unwrap_or_else(|_| Arc::new(NoopWakeSignal))
    }

    fn js_wake(&self) -> Arc<dyn WakeSignal> {
        self.js_wake
            .lock()
            .map(|wake| wake.clone())
            .unwrap_or_else(|_| Arc::new(NoopWakeSignal))
    }
}