use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{Mutex};
use tokio_stream::StreamExt;
use tokio_util::codec::{Framed, LinesCodec};

use futures::SinkExt;
use std::error::Error;
use std::net::SocketAddr;
use std::sync::Arc;

use networking::{Shared, Peer, User, DEFAULT_ADDR, Chat};


#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let state = Arc::new(Mutex::new(Shared::new()));
    let addr = DEFAULT_ADDR.to_string();
    let listener = TcpListener::bind(&addr).await?;

    println!("Server running on {}", addr);

    loop {
        let (stream, addr) = listener.accept().await?;
        let state = Arc::clone(&state);

        tokio::spawn(async move {
            println!("accepted connection");
            if let Err(e) = process(state, stream, addr).await {
                eprintln!("Process; error = {:?}", e);
            }
        });
    }
}


async fn process(state: Arc<Mutex<Shared>>, stream: TcpStream, peer_addr: SocketAddr,) 
    -> Result<(), Box<dyn Error>> 
{
    let mut lines = Framed::new(stream, LinesCodec::new());

    let mut username = String::new();
    request_user_input("Please enter your username:", &mut lines, &mut username).await?;
    
    let mut friend_username = String::new();
    request_user_input("Please enter your friends name:", &mut lines, &mut friend_username).await?;
    
    let user = User {
        addr: peer_addr,
        usrname: username.clone(),
    };

    // Register our peer with state which internally sets up some channels.
    let mut peer = Peer::new(state.clone(), lines, &user).await?;
    
    let mut new_state = state.lock().await;
    new_state.join_chat(&user, &friend_username);
    std::mem::drop(new_state);
    
    let mut state_lock = state.lock().await;
    let msg = format!("{} has joined the chat", username);
    let sender_msg = format!("Welcome to the chat {}!", username);
    println!("{}", msg.trim());
    state_lock.broadcast(peer_addr, &msg, &sender_msg).await;
    std::mem::drop(state_lock);


    loop {
        tokio::select! {
            Some(msg) = peer.reciever.recv() => {
                peer.lines.send(&msg).await?;
            }
            result = peer.lines.next() => match result {
                // A message was received from the current user, we should
                // broadcast this message to the other users.
                Some(Ok(msg)) => {
                    let mut state_lock = state.lock().await;
                    let msg = format!("{}: {}", username, msg);
                    println!("{}", msg);
                    state_lock.broadcast(peer_addr, &msg, &msg).await;
                    std::mem::drop(state_lock);
                }
                // An error occurred.
                Some(Err(e)) => {
                    // eprintln!(
                    //     "an error occurred while processing messages for {}; error = {:?}",
                    //     username,
                    //     e
                    // );
                }
                // The stream has been exhausted.
                None => break,
            },
        }
    }

    // Disconnect user and notify other users
    let mut state_lock = state.lock().await;
    state_lock.peers.remove(&user);
    let msg = format!("{} has left the chat", username);
    let exit_msg = format!("You have left the chat");
    println!("{}", msg);
    
    
    // Clean up potentially now empty chats
    let user_chats = state_lock.get_mut_chats_from_user_addr(user.addr);
    let mut chats_to_remove = Vec::<Chat>::new();
    for chat in user_chats {
        let userindex = chat.members.iter().position(|x| x == &user.addr).unwrap();
        chat.members.remove(userindex);
        if chat.members.is_empty() {
            chats_to_remove.push(chat.clone());
        };
    };
    for chat in chats_to_remove.iter() {
        let index = state_lock.chats.iter().position(|x| x == chat).unwrap();
        state_lock.chats.remove(index);
    };
    println!("Cleaned up {} empty chatroom(s)", chats_to_remove.len());
    

    state_lock.broadcast(peer_addr, &msg, &exit_msg).await;
    std::mem::drop(state_lock);
    Ok(())
}


async fn request_user_input(
    input_message: &str, lines: &mut Framed<TcpStream, LinesCodec>,
    into: &mut String)  -> Result<(), Box<dyn Error>> 
    {
    lines.send(input_message.trim()).await?;
    match lines.next().await {
        Some(Ok(line)) => *into = join_strings(into.to_string(), line),
        // We didn't get a line so we return early here.
        _ => {
            eprintln!("Error while getting user input");
            return Ok(());
        }
    };
    Ok(())
}

fn join_strings(str1: String, str2: String) -> String {
    String::from(format!("{}{}", str1.as_str(), str2.as_str()))
}