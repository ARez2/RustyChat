use std::io;
use std::{collections::HashMap, net::SocketAddr};
use std::sync::Arc;
use futures::SinkExt;
use serde_derive::{Serialize, Deserialize};
use tokio::sync::MutexGuard;
use tokio::{sync::{mpsc, Mutex}, net::TcpStream};
use tokio_stream::StreamExt;
use tokio_util::codec::{Framed, LinesCodec};


pub const DEFAULT_ADDR : &str = "127.0.0.1:6142";

pub type Transmitter = mpsc::UnboundedSender<String>;
pub type Reciever = mpsc::UnboundedReceiver<String>;


#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Chat {
    pub members: Vec<SocketAddr>,
}



#[derive(PartialEq, Eq, PartialOrd, Ord, Hash, Clone)]
pub struct User {
    pub addr: SocketAddr,
    pub usrname: String,
}

pub struct Shared {
    pub peers: HashMap<User, Transmitter>,
    pub chats: Vec<Chat>,
}

impl Shared {
    pub fn new() -> Self {
        Shared {
            peers: HashMap::new(),
            chats: Vec::<Chat>::new(),
        }
    }

    pub fn get_usr_from_addr(&self, addr: SocketAddr) -> Option<&User> {
        let mut user = None;
        for p in self.peers.iter() {
            if p.0.addr == addr {
                user = Some(p.0);
                break;
            }
        };
        user
    }

    pub fn get_usr_from_name(&self, name: String) -> Option<&User> {
        let mut user = None;
        for p in self.peers.iter() {
            if p.0.usrname == name {
                user = Some(p.0);
                break;
            }
        };
        user
    }


    pub async fn broadcast(&mut self, sender: SocketAddr, message: &Message, custom_sender_msg: &Message, codec: &mut Codec) {
        let user_chats = self.get_chats_from_user_addr(sender).clone();
        for chat in user_chats {
            for peer in self.peers.iter() {
                // peer is part of the currently afftected chat
                if chat.members.contains(&peer.0.addr) {
                    // This is the sender, so send him a custom message
                    if custom_sender_msg.text.len() > 0 && peer.0.addr == sender {
                        let send : String = custom_sender_msg.into();
                        if send.len() > 0 {
                            let _ = peer.1.send(send);
                        }
                        //codec.send_message(custom_sender_msg).await;
                    // Not the sender
                    } else if peer.0.addr != sender {
                        let send : String = message.into();
                        if send.len() > 0 {
                            let _ = peer.1.send(send);
                        }
                        //codec.send_message(message).await;
                    }
                }
            }
        }

    }

    pub fn join_chat(&mut self, joining_user: &User, joined_user: &String) {
        let other_user = self.get_usr_from_name(joined_user.clone());
        match other_user {
            Some(user) => {
                let other_user_addr = user.addr;
                let mut other_user_in_chat = false;
                for chat in self.chats.iter_mut() {
                    // Other user is already part of a chat
                    if chat.members.contains(&other_user_addr) {
                        chat.members.push(joining_user.addr);
                        println!("{} is already part of chat: {:?}", joined_user, chat);
                        other_user_in_chat = true;
                    }
                };
                if !other_user_in_chat {
                    let new_chat = Chat {
                        members: vec![joining_user.addr, other_user_addr],
                    };
                    self.chats.push(new_chat);
                };
            },
            None => {
                eprintln!("Specified friend user doesn't exist! Creating new chat");
                let new_chat = Chat {
                    members: vec![joining_user.addr],
                };
                self.chats.push(new_chat);
            },
        }
    }


    pub fn get_chats_from_user_addr(&self, addr: SocketAddr) -> Vec<&Chat> {
        let mut chats = vec![];
        for chat in self.chats.iter() {
            if chat.members.contains(&addr) {
                chats.push(chat);
            }
        }
        chats
    }


    pub fn get_mut_chats_from_user_addr(&mut self, addr: SocketAddr) -> Vec<&mut Chat> {
        let mut chats = vec![];
        for chat in self.chats.iter_mut() {
            if chat.members.contains(&addr) {
                chats.push(chat);
            }
        }
        chats
    }
}



#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub enum UserSetupType {
    UsernameConfirmed,
}


#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub enum MessageType {
    SystemInfo,
    UserSetup(UserSetupType),
    User,
    Command,
}


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

pub struct Codec {
    lines: Framed<TcpStream, LinesCodec>,
}

impl Codec {
    pub fn new(stream: TcpStream) -> Codec {
        Codec {
            lines: Framed::new(stream, LinesCodec::new()),
        }
    }

    pub async fn send_message(&mut self, message: &Message) {
        let serialized: String = message.into();
        self.lines.send(serialized).await.unwrap();
    }

    pub async fn next(&mut self) -> Option<Result<String, tokio_util::codec::LinesCodecError>> {
        self.lines.next().await
    }
}


pub struct Peer {
    pub codec: Codec,
    pub reciever: Reciever,
}

impl Peer {
    pub async fn new(
        state: Arc<Mutex<Shared>>,
        codec: Codec,
        user: &User,
    ) -> io::Result<Peer> {
        let (transmitter, reciever) = mpsc::unbounded_channel();
        let mut guard : MutexGuard<Shared> = state.lock().await;
        guard.peers.insert(user.clone(), transmitter);
        std::mem::drop(guard);

        Ok(Peer {
            codec,
            reciever,
        })
    }
}