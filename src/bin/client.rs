use std::{error::Error, collections::VecDeque, time::Duration};
use futures::pin_mut;
use tokio::{
    sync::mpsc::{self, Sender, Receiver},
    net::{TcpStream, tcp::{OwnedReadHalf, OwnedWriteHalf}},
    io::{AsyncWriteExt, BufReader, BufWriter, AsyncBufReadExt},
};
use std::io;

use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use tui::{
    backend::{Backend, CrosstermBackend},
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Span, Spans, Text},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame, Terminal,
};

use networking::{DEFAULT_ADDR, Message, MessageType, UserSetupType};


struct App {
    input: String,
    messages: VecDeque<Message>,
}

impl Default for App {
    fn default() -> App {
        App {
            input: String::new(),
            messages: VecDeque::new(),
        }
    }
}

struct UserSetup {
    username: String,
}

impl UserSetup {
    fn new() -> UserSetup {
        UserSetup {
            username: String::from("Anon"),
        }
    }
}


#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let mut user_setup = UserSetup::new();
    let stream = TcpStream::connect(DEFAULT_ADDR.to_string()).await?;
    
    let split = stream.into_split();
    let reader = BufReader::new(split.0);
    let writer = BufWriter::new(split.1);

    let (incoming_sender, incoming_reciever) = mpsc::channel(32);


    let ui_handle = tokio::spawn(async move {
        handle_ui(incoming_reciever, writer, &mut user_setup).await
    });

    let poll_incoming_handle = tokio::spawn (async {
        let res = poll(reader, incoming_sender).await;
        res
    });

    // Currently user input is handled inside the handle_ui function
    // So when the handle_ui tokio runtime terminates, the user caused the exit
    ui_handle.await;
    Ok(())
}


async fn poll(mut reader: BufReader<OwnedReadHalf>, tx: Sender<Message>) -> Result<(), &'static str> {
    loop {
        let mut line = String::new();
        let poller = reader.read_line(&mut line);
        pin_mut!(poller);
        if let Ok(_) = tokio::time::timeout(Duration::from_micros(10), &mut poller).await {
            if line.len() > 0 {
                let deser_msg: Message = serde_json::from_str(line.trim()).unwrap();
                tx.send(deser_msg).await.unwrap();
            };
        };
    }
    return Ok(());
}


async fn handle_ui(mut incoming_reciever: Receiver<Message>, mut writer: BufWriter<OwnedWriteHalf>, setup: &mut UserSetup) {
    // setup terminal
    enable_raw_mode().unwrap();
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture).unwrap();
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = App::default();

    let res = terminal.draw(|f| draw_ui(&app, f));
    match res {
        Ok(_) => (),
        Err(err) => {
            println!("Error {}", err);
            return;
        },
    };

    let wait_time = 10;
    loop {
        // Check for user input and display it if there is one
        if crossterm::event::poll(Duration::from_micros(0)).unwrap()
        {
            if let Event::Key(key) = event::read().unwrap() {
                //tx.send(outp.clone()).await.unwrap();
                match key.code {
                    KeyCode::Char(c) => {
                        //tx.send(outp.clone()).await.unwrap();
                        app.input.push(c);
                    },
                    KeyCode::Backspace => {app.input.pop();},
                    KeyCode::Enter => {
                        if app.input == "/exit" {
                            break;
                        };
                        if app.input.len() > 0 {
                            let msg = Message {
                                text: app.input.clone(),
                                msg_type: MessageType::User,
                                author: setup.username.clone(),
                            };
                            app.messages.push_back(msg.clone());
                            let deser : String = msg.into();
                            writer.write(&deser.as_bytes()).await.unwrap();
                            writer.write(&['\n' as u8]).await.unwrap();
                            writer.flush().await.unwrap();
                            app.input.clear();
                        };
                    }
                    _ => (),
                };
            };
        };

        // Check if we can display any incoming messages on the UI
        let recv_incoming = incoming_reciever.recv();
        pin_mut!(recv_incoming);
        if let Ok(x) = tokio::time::timeout(Duration::from_micros(wait_time), &mut recv_incoming).await {
            if let Some(incoming_msg) = x {
                if incoming_msg.author != setup.username {
                    match incoming_msg.msg_type {
                        MessageType::UserSetup(UserSetupType::UsernameConfirmed) => {
                            setup.username = incoming_msg.text.clone();
                        },
                        _ => {
                            app.messages.push_back(incoming_msg);
                        }
                    };
                };
                if app.messages.len() > 10 {
                    app.messages.pop_front();
                };
            };
        };
        
        // Match just in case something in the drawing goes wrong
        let res = terminal.draw(|f| draw_ui(&app, f));
        match res {
            Ok(_) => (),
            Err(err) => {
                println!("Error {}", err);
                break;
            },
        };
    };

    // restore terminal
    disable_raw_mode().unwrap();
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    ).unwrap();
    terminal.show_cursor().unwrap();
}



fn draw_ui<B: Backend>(app: &App, f: &mut Frame<B>) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(
            [
                Constraint::Length(1),
                Constraint::Min(1),
                Constraint::Length(3),
            ]
            .as_ref(),
        )
        .split(f.size());
    
    let msg = vec![
        Span::styled("Chat", Style::default().add_modifier(Modifier::BOLD)),
    ];
    let style = Style::default();
    
    let mut text = Text::from(Spans::from(msg));
    text.patch_style(style);
    let help_message = Paragraph::new(text);
    f.render_widget(help_message, chunks[0]);
    

    let messages: Vec<ListItem> = app
    .messages
    .iter()
    .enumerate()
    .map(|(i, msg)| {
        let mut style = Style::default();
        match msg.msg_type {
            MessageType::SystemInfo => style = style.fg(Color::Red),
            MessageType::User => (),
            _ => (),
        };
        let span = Span::styled(format!("{}", msg), style);
        let content = vec![Spans::from(span)];
        ListItem::new(content)
    })
    .collect();
    let messages =
        List::new(messages).block(Block::default().borders(Borders::ALL).title("Messages"));
    f.render_widget(messages, chunks[1]);


    let input_block = chunks[2];
    let input = Paragraph::new(app.input.clone())
        .style(Style::default().fg(Color::Yellow))
        .block(Block::default().borders(Borders::ALL).title("Input"));
    f.render_widget(input, input_block);
    f.set_cursor(
        // Put cursor past the end of the input text
        input_block.x + app.input.len() as u16 + 1,
        // Move one line down, from the border to the input line
        input_block.y + 1,
    )
}