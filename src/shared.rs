use std::collections::HashMap;
use std::net::SocketAddr;

use crate::peer::User;
use crate::message::Message;
use crate::codec::Codec;
use crate::Transmitter;



/// Represents a single chat room instance on the server
/// Might hold chat name/ description, mods etc. later
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Chat {
    pub members: Vec<SocketAddr>,
}


/// Data that is shared between each user
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

    /// Sends the message to all users present in the chat
    /// Can send a custom sender message to current client
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


    /// Joins the chat based on another users username
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
