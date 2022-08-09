use serde_derive::{Serialize, Deserialize};

use crate::types::MessageType;


/// Represents a message that gets serialized and deserialized when being sent.
/// The into implementations use serde (serde_json) to (de-)serialized the message
/// into a string
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Message {
    pub text: String,
    pub msg_type: MessageType,
    pub author: String,
}

impl Message {
    pub fn new() -> Message {
        Message {
            text: String::new(),
            msg_type: MessageType::User,
            author: String::new(),
        }
    }
}

impl Into<std::string::String> for Message {
    fn into(self) -> std::string::String {
        serde_json::to_string(&self).unwrap()
    }
}
impl Into<std::string::String> for &Message {
    fn into(self) -> std::string::String {
        serde_json::to_string(self).unwrap()
    }
}
impl std::fmt::Display for Message {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.author, self.text)
    }
}
impl Into<Message> for String {
    fn into(self) -> Message {
        serde_json::from_str(self.as_str()).unwrap()
    }
}

