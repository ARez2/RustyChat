use std::io;
use std::{collections::HashMap, net::SocketAddr};
use std::sync::Arc;
use tokio::{sync::{mpsc, Mutex}, net::TcpStream};
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


    pub async fn broadcast(&mut self, sender: SocketAddr, message: &str, custom_sender_msg: &str) {
        let user_chats = self.get_chats_from_user_addr(sender).clone();
        for chat in user_chats {
            for peer in self.peers.iter() {
                // peer is part of the currently afftected chat
                if chat.members.contains(&peer.0.addr) {
                    // This is the sender, so send him a custom message
                    if custom_sender_msg.len() > 0 && peer.0.addr == sender {
                        let _ = peer.1.send(custom_sender_msg.into());
                    // Not the sender
                    } else if peer.0.addr != sender {
                        let _ = peer.1.send(message.into());
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



pub struct Peer {
    pub lines: Framed<TcpStream, LinesCodec>,
    pub reciever: Reciever,
}

impl Peer {
    pub async fn new(
        state: Arc<Mutex<Shared>>,
        lines: Framed<TcpStream, LinesCodec>,
        user: &User,
    ) -> io::Result<Peer> {
        let (transmitter, reciever) = mpsc::unbounded_channel();
        let mut guard = state.lock().await;
        guard.peers.insert(user.clone(), transmitter);
        std::mem::drop(guard);

        Ok(Peer {
            lines,
            reciever,
        })
    }
}