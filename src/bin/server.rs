use futures::lock::MutexLockFuture;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{Mutex, MutexGuard};

use std::error::Error;
use std::net::SocketAddr;
use std::sync::Arc;

use rusty_chat::{Shared, Peer, User, DEFAULT_ADDR, Chat, Codec, Message, MessageType, UserSetupType};

const SYSTEM_USRNAME : &str= "SYSTEM";

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
    let mut codec = Codec::new(stream);

    let mut username = String::new();
    request_user_input("Please enter your username:", &mut codec, &mut username).await?;
    codec.send_message(&Message {
        text: username.clone(),
        msg_type: MessageType::UserSetup(UserSetupType::UsernameConfirmed),
        author: String::from(SYSTEM_USRNAME),
    }).await;
    
    let user = User {
        addr: peer_addr,
        usrname: username.clone(),
    };

    // Register our peer with state which internally sets up some channels.
    let mut peer = Peer::new(state.clone(), codec, &user).await?;

    let mut has_task = false;
    loop {
        if has_task {
            continue;
        };
        tokio::select! {
            Some(deser_msg) = peer.reciever.recv() => {
                let message : Message = deser_msg.into();
                peer.codec.send_message(&message).await;
            },
            result = peer.codec.next() => match result {
                Some(Ok(deser_msg)) => {
                    if deser_msg.len() > 0 {
                        let msg : Message = deser_msg.into();
                        if msg.msg_type == MessageType::Command && msg.text.starts_with("/") {
                            has_task = true;
                            handle_command(state.clone(), &msg.text, &user, &mut peer, &username).await;
                            has_task = false;
                        };
                        println!("{}", msg);
                        let mut state_lock = state.lock().await;
                        state_lock.broadcast(peer_addr, &msg, &msg, &mut peer.codec).await;
                        std::mem::drop(state_lock);
                    }
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
    let mut state_lock : MutexGuard<Shared> = state.lock().await;
    state_lock.peers.remove(&user);
    let msg = Message {
        text: format!("{} has left the chat", username),
        msg_type: MessageType::SystemInfo,
        author: String::from(SYSTEM_USRNAME),
    };
    let exit_msg = Message {
        text: format!("You have left the chat"),
        msg_type: MessageType::SystemInfo,
        author: String::from(SYSTEM_USRNAME),
    };
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
    

    state_lock.broadcast(peer_addr, &msg, &exit_msg, &mut peer.codec).await;
    std::mem::drop(state_lock);
    Ok(())
}


async fn request_user_input(
    input_message: &str, codec: &mut Codec,
    into: &mut String)  -> Result<(), Box<dyn Error>> 
    {
    let msg = Message {
        text: String::from(input_message),
        msg_type: MessageType::SystemInfo,
        author: String::from(SYSTEM_USRNAME),
    };
    codec.send_message(&msg).await;
    match codec.next().await {
        Some(Ok(line)) => {
            let msg : Message = line.into();
            *into = join_strings(into.to_string(), msg.text)
        },
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



async fn handle_command(state: Arc<Mutex<Shared>>, input: &String, user: &User, peer: &mut Peer, username: &String) {
    let mut state_lock : MutexGuard<Shared> = state.lock().await;
    
    let full_cmd_input = input.clone().replace("/", "");
    let cmd_res = full_cmd_input.split(" ");
    let mut args = cmd_res.into_iter();
    let cmd = args.next().unwrap();
    match cmd {
        "join" => {
            let friend_username = String::from(args.next().unwrap());
            state_lock.join_chat(user, &friend_username);
            //std::mem::drop(new_state);
            println!("Joining...");
            
            let msg = Message {
                text: format!("{} has joined the chat", username),
                msg_type: MessageType::SystemInfo,
                author: String::from(SYSTEM_USRNAME),
            };
            let sender_msg = Message {
                text: format!("Welcome to the chat {}!", username),
                msg_type: MessageType::SystemInfo,
                author: String::from(SYSTEM_USRNAME),
            };
            println!("{}", msg);
            state_lock.broadcast(user.addr, &msg, &sender_msg, &mut peer.codec).await;
        },
        _ => (),
    };
    std::mem::drop(state_lock);
}