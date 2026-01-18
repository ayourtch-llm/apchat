use std::sync::mpsc::{channel, Sender, Receiver};
use crate::mspc::message::{MspcMessage, ChannelError};

#[derive(Clone)]
pub struct MessagePair {
    pub user: String,
    pub agent: String,
}

pub struct MspcChannel {
    sender: Sender<MspcMessage>,
    receiver: Receiver<MspcMessage>,
    message_history: Vec<MessagePair>,
}

impl MspcChannel {
    pub fn new() -> Self {
        let (sender, receiver) = channel();
        Self {
            sender,
            receiver,
            message_history: Vec::new(),
        }
    }

    pub fn send(&self, message: MspcMessage) -> Result<(), ChannelError> {
        self.sender.send(message).map_err(|_| ChannelError)?;
        Ok(())
    }

    pub fn try_recv(&self) -> Result<Option<MspcMessage>, ChannelError> {
        match self.receiver.try_recv() {
            Ok(msg) => Ok(Some(msg)),
            Err(std::sync::mpsc::TryRecvError::Empty) => Ok(None),
            Err(std::sync::mpsc::TryRecvError::Disconnected) => Err(ChannelError),
        }
    }

    pub fn recv(&self) -> Result<MspcMessage, ChannelError> {
        self.receiver.recv().map_err(|_| ChannelError)
    }

    pub fn add_user_message(&mut self, content: String) {
        self.message_history.push(MessagePair {
            user: content,
            agent: String::new(),
        });
    }

    pub fn add_agent_message(&mut self, content: String) {
        if let Some(last) = self.message_history.last_mut() {
            last.agent = content;
        }
    }

    pub fn handle_interruption(&mut self) {
        // Clean up interrupted agent message
        if let Some(last) = self.message_history.last_mut() {
            last.agent.clear();
        }
    }

    pub fn get_history_for_prompt(&self) -> Vec<MessagePair> {
        self.message_history.clone()
    }
}
