// Webex REST API client

use anyhow::{Context, Result};
use reqwest::Client;
use crate::types::*;

const WEBEX_API_BASE: &str = "https://webexapis.com/v1";

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

        let response: PeopleResponse = self.client
            .get(&url)
            .bearer_auth(&self.access_token)
            .send()
            .await
            .context("Failed to send request to Webex API")?
            .json()
            .await
            .context("Failed to parse Webex API response")?;

        response.items
            .into_iter()
            .next()
            .ok_or_else(|| anyhow::anyhow!("Person not found with email: {}", email))
    }

    /// Get bot's own details
    pub async fn get_me(&self) -> Result<Person> {
        let url = format!("{}/people/me", WEBEX_API_BASE);

        self.client
            .get(&url)
            .bearer_auth(&self.access_token)
            .send()
            .await
            .context("Failed to get bot details")?
            .json()
            .await
            .context("Failed to parse bot details")
    }

    /// Create a direct room with a person
    pub async fn create_direct_room(&self, person_id: &str) -> Result<Room> {
        let url = format!("{}/rooms", WEBEX_API_BASE);

        let request = CreateRoomRequest {
            title: None,
            to_person_id: Some(person_id.to_string()),
        };

        self.client
            .post(&url)
            .bearer_auth(&self.access_token)
            .json(&request)
            .send()
            .await
            .context("Failed to create direct room")?
            .json()
            .await
            .context("Failed to parse room response")
    }

    /// Send a message to a room
    pub async fn send_message(&self, room_id: &str, text: &str) -> Result<Message> {
        let url = format!("{}/messages", WEBEX_API_BASE);

        let request = SendMessageRequest {
            room_id: room_id.to_string(),
            text: text.to_string(),
        };

        self.client
            .post(&url)
            .bearer_auth(&self.access_token)
            .json(&request)
            .send()
            .await
            .context("Failed to send message")?
            .json()
            .await
            .context("Failed to parse message response")
    }

    /// Get messages from a room
    /// mentionedPeople=me ensures we only get messages that mention the bot (or DMs)
    pub async fn get_messages(&self, room_id: &str) -> Result<Vec<Message>> {
        let url = format!("{}/messages?roomId={}&mentionedPeople=me&max=100",
            WEBEX_API_BASE, room_id);

        let response: MessagesResponse = self.client
            .get(&url)
            .bearer_auth(&self.access_token)
            .send()
            .await
            .context("Failed to get messages")?
            .json()
            .await
            .context("Failed to parse messages response")?;

        Ok(response.items)
    }
}
