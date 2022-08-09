use tokio::sync::mpsc;

pub const DEFAULT_ADDR : &str = "127.0.0.1:6142";

pub type Transmitter = mpsc::UnboundedSender<String>;
pub type Reciever = mpsc::UnboundedReceiver<String>;


pub mod shared;
pub mod message;
pub mod peer;
pub mod types;
pub mod codec;


/// Utility function to join the contents of 2 Strings together
pub fn join_strings(str1: String, str2: String) -> String {
    String::from(format!("{}{}", str1.as_str(), str2.as_str()))
}