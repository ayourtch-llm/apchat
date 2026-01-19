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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(rename = "toPersonId", skip_serializing_if = "Option::is_none")]
    pub to_person_id: Option<String>,
    #[serde(rename = "toPersonEmail", skip_serializing_if = "Option::is_none")]
    pub to_person_email: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct SendMessageRequest {
    #[serde(rename = "roomId", skip_serializing_if = "Option::is_none")]
    pub room_id: Option<String>,
    #[serde(rename = "toPersonEmail", skip_serializing_if = "Option::is_none")]
    pub to_person_email: Option<String>,
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

// Device registration for Mercury WebSocket connection
#[derive(Debug, Serialize)]
pub struct DeviceRegistrationRequest {
    #[serde(rename = "deviceName")]
    pub device_name: String,
    #[serde(rename = "deviceType")]
    pub device_type: String,
    #[serde(rename = "localizedModel")]
    pub localized_model: String,
    pub model: String,
    #[serde(rename = "systemName")]
    pub system_name: String,
    #[serde(rename = "systemVersion")]
    pub system_version: String,
}

#[derive(Debug, Deserialize)]
pub struct DeviceRegistration {
    pub url: String,
    #[serde(rename = "webSocketUrl")]
    pub web_socket_url: String,
    pub id: String,
}

// Mercury WebSocket event messages
#[derive(Debug, Deserialize)]
pub struct MercuryEvent {
    pub id: String,
    pub data: MercuryEventData,
}

#[derive(Debug, Deserialize)]
pub struct MercuryEventData {
    #[serde(rename = "eventType")]
    pub event_type: String,
    pub activity: Option<Activity>,
}

#[derive(Debug, Deserialize)]
pub struct Activity {
    pub verb: String,
    pub object: serde_json::Value,
}
