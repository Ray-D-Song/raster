use std::sync::mpsc;

/// Sender half of a simple cross-thread queue.
#[derive(Debug)]
pub struct ChannelSender<T> {
    sender: mpsc::Sender<T>,
}

/// Receiver half of a simple cross-thread queue.
#[derive(Debug)]
pub struct ChannelReceiver<T> {
    receiver: mpsc::Receiver<T>,
}

pub fn channel<T>() -> (ChannelSender<T>, ChannelReceiver<T>) {
    let (sender, receiver) = mpsc::channel();
    (ChannelSender { sender }, ChannelReceiver { receiver })
}

impl<T> Clone for ChannelSender<T> {
    fn clone(&self) -> Self {
        Self {
            sender: self.sender.clone(),
        }
    }
}

impl<T> ChannelSender<T> {
    pub fn send(&self, value: T) -> Result<(), mpsc::SendError<T>> {
        self.sender.send(value)
    }
}

impl<T> ChannelReceiver<T> {
    pub fn try_recv(&self) -> Result<T, mpsc::TryRecvError> {
        self.receiver.try_recv()
    }

    pub fn recv(&self) -> Result<T, mpsc::RecvError> {
        self.receiver.recv()
    }

    pub fn drain(&self) -> Vec<T> {
        let mut values = Vec::new();
        while let Ok(value) = self.try_recv() {
            values.push(value);
        }
        values
    }
}
