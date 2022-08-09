use tokio_util::codec::{Framed, LinesCodec};
use tokio::{net::TcpStream};
use futures::SinkExt;
use tokio_stream::StreamExt;

use crate::message::Message;


/// A wrapper around the provided Frame by tokio. It helps providing a single way
/// on how to send a message to the client
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