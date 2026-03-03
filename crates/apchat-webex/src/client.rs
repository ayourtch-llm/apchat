// Webex REST API client

use anyhow::{Context, Result};
use reqwest::Client;
use crate::types::*;
use apchat_vty::print_heart_yellow;

const WEBEX_API_BASE: &str = "https://webexapis.com/v1";
const WEBEX_WDM_BASE: &str = "https://wdm-a.wbx2.com/wdm/api/v1";

pub struct WebexClient {
    client: Client,
    access_token: String,
}

impl WebexClient {
    pub fn new(access_token: String) -> Self {
        Self {
            client: Client::new(),
            access_token,
        }
    }

    /// Get person by email
    pub async fn get_person_by_email(&self, email: &str) -> Result<Person> {
        let url = format!("{}/people?email={}", WEBEX_API_BASE, email);
        print_heart_yellow(&format!("🔍 Fetching person by email: {}", email), true);
        print_heart_yellow(&format!("🔍 URL: {}", url), true);

        let response = self.client
            .get(&url)
            .bearer_auth(&self.access_token)
            .send()
            .await
            .context("Failed to send request to Webex API")?;

        print_heart_yellow(&format!("🔍 Response status: {}", response.status()), true);

        let response: PeopleResponse = response
            .json()
            .await
            .context("Failed to parse Webex API response")?;

        print_heart_yellow(&format!("🔍 Found {} people", response.items.len()), true);

        response.items
            .into_iter()
            .next()
            .ok_or_else(|| anyhow::anyhow!("Person not found with email: {}", email))
    }

    /// Get bot's own details
    pub async fn get_me(&self) -> Result<Person> {
        let url = format!("{}/people/me", WEBEX_API_BASE);
        print_heart_yellow("🔍 Fetching bot details from /people/me", true);

        let response = self.client
            .get(&url)
            .bearer_auth(&self.access_token)
            .send()
            .await
            .context("Failed to get bot details")?;

        print_heart_yellow(&format!("🔍 Bot details response status: {}", response.status()), true);

        let person: Person = response
            .json()
            .await
            .context("Failed to parse bot details")?;

        print_heart_yellow(&format!("🔍 Bot email: {:?}", person.emails.first()), true);
        Ok(person)
    }

    /// Create a direct room with a person by person ID
    pub async fn create_direct_room(&self, person_id: &str) -> Result<Room> {
        let url = format!("{}/rooms", WEBEX_API_BASE);
        print_heart_yellow(&format!("🔍 Creating direct room with person_id: {}", person_id), true);

        let request = CreateRoomRequest {
            title: None,
            to_person_id: Some(person_id.to_string()),
            to_person_email: None,
        };

        let response = self.client
            .post(&url)
            .bearer_auth(&self.access_token)
            .json(&request)
            .send()
            .await
            .context("Failed to create direct room")?;

        print_heart_yellow(&format!("🔍 Create room response status: {}", response.status()), true);

        let room: Room = response
            .json()
            .await
            .context("Failed to parse room response")?;

        print_heart_yellow(&format!("🔍 Created room ID: {}", room.id), true);
        Ok(room)
    }

    /// Create a direct room with a person by email address
    pub async fn create_direct_room_by_email(&self, email: &str) -> Result<Room> {
        let url = format!("{}/rooms", WEBEX_API_BASE);
        print_heart_yellow(&format!("🔍 Creating direct room with email: {}", email), true);

        let request = CreateRoomRequest {
            title: None,
            to_person_id: None,
            to_person_email: Some(email.to_string()),
        };

        print_heart_yellow(&format!("🔍 Request body: {}", serde_json::to_string(&request).unwrap_or_default()), true);

        let response = self.client
            .post(&url)
            .bearer_auth(&self.access_token)
            .json(&request)
            .send()
            .await
            .context("Failed to create direct room")?;

        let status = response.status();
        print_heart_yellow(&format!("🔍 Create room response status: {}", status), true);

        if !status.is_success() {
            let error_body = response.text().await.unwrap_or_else(|_| "Could not read error body".to_string());
            print_heart_yellow(&format!("🔍 Error response body: {}", error_body), true);
            return Err(anyhow::anyhow!("Failed to create room: {} - {}", status, error_body));
        }

        let room: Room = response
            .json()
            .await
            .context("Failed to parse room response")?;

        print_heart_yellow(&format!("🔍 Created room ID: {}", room.id), true);
        Ok(room)
    }

    /// Send a message to a room, optionally as a threaded reply
    pub async fn send_message(&self, room_id: &str, text: &str) -> Result<Message> {
        self.send_message_with_parent(room_id, text, None).await
    }

    /// Send a message to a room with optional parent ID for threading
    pub async fn send_message_with_parent(&self, room_id: &str, text: &str, parent_id: Option<String>) -> Result<Message> {
        let url = format!("{}/messages", WEBEX_API_BASE);
        print_heart_yellow(&format!("🔍 Sending message to room {}{}: {}",
            room_id,
            parent_id.as_ref().map(|p| format!(" (reply to {})", p.chars().take(8).collect::<String>())).unwrap_or_default(),
            text.chars().take(50).collect::<String>()
        ), true);

        let request = SendMessageRequest {
            room_id: Some(room_id.to_string()),
            to_person_email: None,
            parent_id,
            text: text.to_string(),
        };

        let response = self.client
            .post(&url)
            .bearer_auth(&self.access_token)
            .json(&request)
            .send()
            .await
            .context("Failed to send message")?;

        print_heart_yellow(&format!("🔍 Send message response status: {}", response.status()), true);

        let message: Message = response
            .json()
            .await
            .context("Failed to parse message response")?;

        print_heart_yellow(&format!("🔍 Sent message ID: {}", message.id), true);
        Ok(message)
    }

    /// Send a message to a person by email (creates/uses direct room automatically)
    pub async fn send_message_to_email(&self, email: &str, text: &str) -> Result<Message> {
        let url = format!("{}/messages", WEBEX_API_BASE);
        print_heart_yellow(&format!("🔍 Sending message to email {}: {}", email, text.chars().take(50).collect::<String>()), true);

        let request = SendMessageRequest {
            room_id: None,
            to_person_email: Some(email.to_string()),
            parent_id: None,
            text: text.to_string(),
        };

        print_heart_yellow(&format!("🔍 Request body: {}", serde_json::to_string(&request).unwrap_or_default()), true);

        let response = self.client
            .post(&url)
            .bearer_auth(&self.access_token)
            .json(&request)
            .send()
            .await
            .context("Failed to send message")?;

        let status = response.status();
        print_heart_yellow(&format!("🔍 Send message response status: {}", status), true);

        if !status.is_success() {
            let error_body = response.text().await.unwrap_or_else(|_| "Could not read error body".to_string());
            print_heart_yellow(&format!("🔍 Error response body: {}", error_body), true);
            return Err(anyhow::anyhow!("Failed to send message: {} - {}", status, error_body));
        }

        let message: Message = response
            .json()
            .await
            .context("Failed to parse message response")?;

        print_heart_yellow(&format!("🔍 Sent message ID: {}, room ID: {}", message.id, message.room_id), true);
        Ok(message)
    }

    /// Delete a message from a room
    pub async fn delete_message(&self, room_id: &str, text: &str) -> Result<()> {
        // First, find the message ID by getting messages from the room
        let messages = self.get_messages(room_id).await?;
        
        let message_id = messages
            .iter()
            .find(|msg| msg.text.as_deref() == Some(text))
            .map(|msg| msg.id.clone());

        if let Some(msg_id) = message_id {
            let url = format!("{}/messages/{}", WEBEX_API_BASE, msg_id);
            print_heart_yellow(&format!("🗑️ Deleting message ID: {}", msg_id), true);

            let response = self.client
                .delete(&url)
                .bearer_auth(&self.access_token)
                .send()
                .await
                .context("Failed to delete message")?;

            let status = response.status();
            print_heart_yellow(&format!("🗑️ Delete message response status: {}", status), true);

            if !status.is_success() {
                let error_body = response.text().await.unwrap_or_else(|_| "Could not read error body".to_string());
                print_heart_yellow(&format!("🗑️ Error response body: {}", error_body), true);
                return Err(anyhow::anyhow!("Failed to delete message: {} - {}", status, error_body));
            }

            print_heart_yellow("🗑️ Message deleted successfully", true);
            Ok(())
        } else {
            print_heart_yellow(&format!("⚠️ Message not found with text: {}", text), true);
            Ok(())
        }
    }

    /// Get messages from a room
    /// In DM rooms, we get all messages and filter by person_email in the router
    /// Using max=20 since we're polling frequently and only need recent messages
    pub async fn get_messages(&self, room_id: &str) -> Result<Vec<Message>> {
        let url = format!("{}/messages?roomId={}&max=20",
            WEBEX_API_BASE, room_id);

        let response = self.client
            .get(&url)
            .bearer_auth(&self.access_token)
            .send()
            .await
            .context("Failed to get messages")?;

        print_heart_yellow(&format!("🔍 Get messages response status: {}", response.status()), true);

        let messages_response: MessagesResponse = response
            .json()
            .await
            .context("Failed to parse messages response")?;

        print_heart_yellow(&format!("🔍 Received {} messages from room {}", messages_response.items.len(), room_id), true);

        // API returns newest first by default
        Ok(messages_response.items)
    }

    /// Register device to get Mercury WebSocket URL
    pub async fn register_device(&self) -> Result<DeviceRegistration> {
        let url = format!("{}/devices", WEBEX_WDM_BASE);
        print_heart_yellow("🔍 Registering device for Mercury WebSocket", true);
        print_heart_yellow(&format!("🔍 Using WDM endpoint: {}", url), true);

        // Generate unique device name (similar to Python bot implementation)
        // Use timestamp and process ID for uniqueness
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let pid = std::process::id();
        let device_name = format!("apchat-rust-client-{}-{}", pid, timestamp);

        let request = DeviceRegistrationRequest {
            device_name: "apchat-bot".to_string(),
            device_type: "DESKTOP".to_string(),
            localized_model: "python".to_string(),
            model: "python".to_string(),
            name: device_name,
            system_name: "APChat".to_string(),
            system_version: "0.1.0".to_string(),
        };

        print_heart_yellow(&format!("🔍 Device registration payload: {}", serde_json::to_string_pretty(&request).unwrap_or_default()), true);

        let response = self.client
            .post(&url)
            .bearer_auth(&self.access_token)
            .json(&request)
            .send()
            .await
            .context("Failed to register device")?;

        let status = response.status();
        print_heart_yellow(&format!("🔍 Device registration response status: {}", status), true);

        if !status.is_success() {
            let error_body = response.text().await.unwrap_or_else(|_| "Could not read error body".to_string());
            print_heart_yellow(&format!("🔍 Error response body: {}", error_body), true);
            return Err(anyhow::anyhow!("Failed to register device: {} - {}", status, error_body));
        }

        // Get response text first for diagnostics
        let response_text = response.text().await.context("Failed to read response body")?;
        print_heart_yellow(&format!("🔍 Device registration response body: {}", response_text), true);

        let registration: DeviceRegistration = serde_json::from_str(&response_text)
            .context("Failed to parse device registration response")?;

        print_heart_yellow(&format!("🔍 Device registered: WebSocket URL={}", registration.web_socket_url), true);

        Ok(registration)
    }
}
