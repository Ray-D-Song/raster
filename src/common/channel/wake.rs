/// Wake signal used after enqueueing work for the GPUI app thread.
pub trait WakeSignal: Send + Sync {
    fn wake(&self);
}

#[derive(Debug, Default)]
pub struct NoopWakeSignal;

impl WakeSignal for NoopWakeSignal {
    fn wake(&self) {}
}
