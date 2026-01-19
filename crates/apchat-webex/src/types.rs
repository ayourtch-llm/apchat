// Webex API types

use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct Person {
    pub id: String,
    pub emails: Vec<String>,
    #[serde(rename = "displayName")]
    pub display_name: String,
}

#[derive(Debug, Deserialize)]
pub struct PeopleResponse {
    pub items: Vec<Person>,
}

#[derive(Debug, Deserialize)]
pub struct Room {
    pub id: String,
    #[serde(rename = "type")]
    pub room_type: String,
}

#[derive(Debug, Serialize)]
pub struct CreateRoomRequest {
    pub title: Option<String>,
    #[serde(rename = "toPersonId")]
    pub to_person_id: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct SendMessageRequest {
    #[serde(rename = "roomId")]
    pub room_id: String,
    pub text: String,
}

#[derive(Debug, Deserialize)]
pub struct Message {
    pub id: String,
    #[serde(rename = "roomId")]
    pub room_id: String,
    #[serde(rename = "personEmail")]
    pub person_email: String,
    pub text: Option<String>,
    pub created: String,
}

#[derive(Debug, Deserialize)]
pub struct MessagesResponse {
    pub items: Vec<Message>,
}
