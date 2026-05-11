//! Mock transport for testing DAP sessions without real server processes.

use std::collections::VecDeque;
use std::process::ExitStatus;
use std::sync::Mutex;

use crate::error::DapzError;
use crate::transport::Transport;

/// In-memory FIFO transport for testing.
///
/// Pre-program responses via [`push_message`](MockTransport::push_message),
/// inspect sent data via [`sent_messages`](MockTransport::sent_messages).
pub struct MockTransport {
    /// Pre-programmed receive responses (FIFO).
    rx_queue: Mutex<VecDeque<Vec<u8>>>,
    /// All data sent through this transport.
    sent: Mutex<Vec<Vec<u8>>>,
}

impl MockTransport {
    /// Create an empty mock transport.
    pub fn new() -> Self {
        Self {
            rx_queue: Mutex::new(VecDeque::new()),
            sent: Mutex::new(Vec::new()),
        }
    }

    /// Push a raw framed message onto the receive queue.
    pub fn push_message(&self, data: Vec<u8>) {
        self.rx_queue.lock().unwrap().push_back(data);
    }

    /// Get all messages sent through this transport.
    pub fn sent_messages(&self) -> Vec<Vec<u8>> {
        self.sent.lock().unwrap().clone()
    }
}

impl Default for MockTransport {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl Transport for MockTransport {
    async fn receive(&mut self) -> Result<Vec<u8>, DapzError> {
        self.rx_queue
            .lock()
            .unwrap()
            .pop_front()
            .ok_or(DapzError::Protocol("no more messages in mock queue".into()))
    }

    async fn send(&mut self, data: &[u8]) -> Result<(), DapzError> {
        self.sent.lock().unwrap().push(data.to_vec());
        Ok(())
    }

    fn try_wait(&mut self) -> Result<Option<ExitStatus>, DapzError> {
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mock_send_receive() {
        let mut transport = MockTransport::new();

        let msg = b"Content-Length: 4\r\n\r\ntest".to_vec();
        transport.push_message(msg.clone());

        let received = transport.receive().await.unwrap();
        assert_eq!(received, msg);

        transport.send(b"hello").await.unwrap();
        assert_eq!(transport.sent_messages(), vec![b"hello".to_vec()]);
    }

    #[tokio::test]
    async fn test_mock_empty_receive_fails() {
        let mut transport = MockTransport::new();
        let result = transport.receive().await;
        assert!(result.is_err());
    }
}
