use serde_derive::{Serialize, Deserialize};


#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub enum MessageType {
    SystemInfo,
    UserSetup(UserSetupType),
    User,
    Command,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub enum UserSetupType {
    UsernameConfirmed,
}