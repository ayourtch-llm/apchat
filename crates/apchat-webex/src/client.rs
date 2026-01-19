// Webex REST API client

use anyhow::{Context, Result};
use reqwest::Client;
use crate::types::*;

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
        eprintln!("🔍 DEBUG: Fetching person by email: {}", email);
        eprintln!("🔍 DEBUG: URL: {}", url);

        let response = self.client
            .get(&url)
            .bearer_auth(&self.access_token)
            .send()
            .await
            .context("Failed to send request to Webex API")?;

        eprintln!("🔍 DEBUG: Response status: {}", response.status());

        let response: PeopleResponse = response
            .json()
            .await
            .context("Failed to parse Webex API response")?;

        eprintln!("🔍 DEBUG: Found {} people", response.items.len());

        response.items
            .into_iter()
            .next()
            .ok_or_else(|| anyhow::anyhow!("Person not found with email: {}", email))
    }

    /// Get bot's own details
    pub async fn get_me(&self) -> Result<Person> {
        let url = format!("{}/people/me", WEBEX_API_BASE);
        eprintln!("🔍 DEBUG: Fetching bot details from /people/me");

        let response = self.client
            .get(&url)
            .bearer_auth(&self.access_token)
            .send()
            .await
            .context("Failed to get bot details")?;

        eprintln!("🔍 DEBUG: Bot details response status: {}", response.status());

        let person: Person = response
            .json()
            .await
            .context("Failed to parse bot details")?;

        eprintln!("🔍 DEBUG: Bot email: {:?}", person.emails.first());
        Ok(person)
    }

    /// Create a direct room with a person by person ID
    pub async fn create_direct_room(&self, person_id: &str) -> Result<Room> {
        let url = format!("{}/rooms", WEBEX_API_BASE);
        eprintln!("🔍 DEBUG: Creating direct room with person_id: {}", person_id);

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

        eprintln!("🔍 DEBUG: Create room response status: {}", response.status());

        let room: Room = response
            .json()
            .await
            .context("Failed to parse room response")?;

        eprintln!("🔍 DEBUG: Created room ID: {}", room.id);
        Ok(room)
    }

    /// Create a direct room with a person by email address
    pub async fn create_direct_room_by_email(&self, email: &str) -> Result<Room> {
        let url = format!("{}/rooms", WEBEX_API_BASE);
        eprintln!("🔍 DEBUG: Creating direct room with email: {}", email);

        let request = CreateRoomRequest {
            title: None,
            to_person_id: None,
            to_person_email: Some(email.to_string()),
        };

        eprintln!("🔍 DEBUG: Request body: {}", serde_json::to_string(&request).unwrap_or_default());

        let response = self.client
            .post(&url)
            .bearer_auth(&self.access_token)
            .json(&request)
            .send()
            .await
            .context("Failed to create direct room")?;

        let status = response.status();
        eprintln!("🔍 DEBUG: Create room response status: {}", status);

        if !status.is_success() {
            let error_body = response.text().await.unwrap_or_else(|_| "Could not read error body".to_string());
            eprintln!("🔍 DEBUG: Error response body: {}", error_body);
            return Err(anyhow::anyhow!("Failed to create room: {} - {}", status, error_body));
        }

        let room: Room = response
            .json()
            .await
            .context("Failed to parse room response")?;

        eprintln!("🔍 DEBUG: Created room ID: {}", room.id);
        Ok(room)
    }

    /// Send a message to a room
    pub async fn send_message(&self, room_id: &str, text: &str) -> Result<Message> {
        let url = format!("{}/messages", WEBEX_API_BASE);
        eprintln!("🔍 DEBUG: Sending message to room {}: {}", room_id, text.chars().take(50).collect::<String>());

        let request = SendMessageRequest {
            room_id: Some(room_id.to_string()),
            to_person_email: None,
            text: text.to_string(),
        };

        let response = self.client
            .post(&url)
            .bearer_auth(&self.access_token)
            .json(&request)
            .send()
            .await
            .context("Failed to send message")?;

        eprintln!("🔍 DEBUG: Send message response status: {}", response.status());

        let message: Message = response
            .json()
            .await
            .context("Failed to parse message response")?;

        eprintln!("🔍 DEBUG: Sent message ID: {}", message.id);
        Ok(message)
    }

    /// Send a message to a person by email (creates/uses direct room automatically)
    pub async fn send_message_to_email(&self, email: &str, text: &str) -> Result<Message> {
        let url = format!("{}/messages", WEBEX_API_BASE);
        eprintln!("🔍 DEBUG: Sending message to email {}: {}", email, text.chars().take(50).collect::<String>());

        let request = SendMessageRequest {
            room_id: None,
            to_person_email: Some(email.to_string()),
            text: text.to_string(),
        };

        eprintln!("🔍 DEBUG: Request body: {}", serde_json::to_string(&request).unwrap_or_default());

        let response = self.client
            .post(&url)
            .bearer_auth(&self.access_token)
            .json(&request)
            .send()
            .await
            .context("Failed to send message")?;

        let status = response.status();
        eprintln!("🔍 DEBUG: Send message response status: {}", status);

        if !status.is_success() {
            let error_body = response.text().await.unwrap_or_else(|_| "Could not read error body".to_string());
            eprintln!("🔍 DEBUG: Error response body: {}", error_body);
            return Err(anyhow::anyhow!("Failed to send message: {} - {}", status, error_body));
        }

        let message: Message = response
            .json()
            .await
            .context("Failed to parse message response")?;

        eprintln!("🔍 DEBUG: Sent message ID: {}, room ID: {}", message.id, message.room_id);
        Ok(message)
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

        eprintln!("🔍 DEBUG: Get messages response status: {}", response.status());

        let messages_response: MessagesResponse = response
            .json()
            .await
            .context("Failed to parse messages response")?;

        eprintln!("🔍 DEBUG: Received {} messages from room {}", messages_response.items.len(), room_id);

        Ok(messages_response.items)
    }

    /// Register device to get Mercury WebSocket URL
    pub async fn register_device(&self) -> Result<DeviceRegistration> {
        let url = format!("{}/devices", WEBEX_WDM_BASE);
        eprintln!("🔍 DEBUG: Registering device for Mercury WebSocket");
        eprintln!("🔍 DEBUG: Using WDM endpoint: {}", url);

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

        eprintln!("🔍 DEBUG: Device registration payload: {}", serde_json::to_string_pretty(&request).unwrap_or_default());

        let response = self.client
            .post(&url)
            .bearer_auth(&self.access_token)
            .json(&request)
            .send()
            .await
            .context("Failed to register device")?;

        let status = response.status();
        eprintln!("🔍 DEBUG: Device registration response status: {}", status);

        if !status.is_success() {
            let error_body = response.text().await.unwrap_or_else(|_| "Could not read error body".to_string());
            eprintln!("🔍 DEBUG: Error response body: {}", error_body);
            return Err(anyhow::anyhow!("Failed to register device: {} - {}", status, error_body));
        }

        // Get response text first for diagnostics
        let response_text = response.text().await.context("Failed to read response body")?;
        eprintln!("🔍 DEBUG: Device registration response body: {}", response_text);

        let registration: DeviceRegistration = serde_json::from_str(&response_text)
            .context("Failed to parse device registration response")?;

        eprintln!("🔍 DEBUG: Device registered: WebSocket URL={}",
             registration.web_socket_url);

        Ok(registration)
    }
}
