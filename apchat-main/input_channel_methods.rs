    /// Initialize the input channel with a default configuration
    pub(crate) fn initialize_input_channel(&mut self) {
        if self.input_channel.is_none() {
            let config = InputChannelConfig::default();
            self.input_channel = Some(InputChannel::new(config));
        }
    }

    /// Get a reference to the input channel receiver
    /// Returns None if the channel is not initialized
    pub(crate) fn input_channel_receiver(&mut self) -> Option<&mut InputChannel<InputMessage>> {
        self.input_channel.as_mut()
    }

    /// Check if there are pending messages in the input channel
    /// Returns false if the channel is not initialized or has no pending messages
    pub(crate) fn has_pending_input(&mut self) -> bool {
        self.input_channel
            .as_mut()
            .map(|channel| channel.has_pending_messages())
            .unwrap_or(false)
    }

    /// Try to receive a message from the input channel without blocking
    /// Returns None if the channel is not initialized or there are no pending messages
    pub(crate) async fn try_recv_input(&mut self) -> Option<InputMessage> {
        self.input_channel
            .as_mut()
            .and_then(|channel| async {
                channel.try_recv().await
            }
            .await)
    }

    /// Get a reference to the input channel sender
    /// Returns None if the channel is not initialized
    pub(crate) fn input_channel_sender(&self) -> Option<tokio::sync::mpsc::Sender<InputMessage>> {
        self.input_channel.as_ref().map(|_| {
            // Note: In the current implementation, we don't store the sender
            // This would need to be modified if we want to send messages externally
            // For now, returning None to indicate this is not yet implemented
            None
        }).flatten()
    }

