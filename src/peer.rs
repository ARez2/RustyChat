use tokio::sync::{Mutex, MutexGuard, mpsc};
use std::net::SocketAddr;
use std::sync::Arc;
use std::io;

use crate::shared::Shared;
use crate::codec::Codec;
use crate::Reciever;


/// A single peer that is connected to the server.
/// I split Peer and User so that i can separately clone the User struct
/// as i cannot clone a peer because of its fields
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


/// A single User object
#[derive(PartialEq, Eq, PartialOrd, Ord, Hash, Clone)]
pub struct User {
    pub addr: SocketAddr,
    pub usrname: String,
}