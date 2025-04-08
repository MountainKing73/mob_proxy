use futures::SinkExt;
use log::{debug, error, info};
use regex::Regex;
use tokio::net::{TcpListener, TcpStream};
use tokio_stream::StreamExt;
use tokio_util::codec::{FramedRead, FramedWrite, LinesCodec};

const COIN_ADDRESS: &str = "7YWHMfk9JZe0LM0g1ZauHuiSxhI";

pub async fn run(chat_connect: String) {
    let listener = TcpListener::bind("0.0.0.0:8080").await.unwrap();
    info!("Listening on port 8080");

    loop {
        let (socket, _) = listener.accept().await.unwrap();
        debug!("Starting connection");
        let chat_connect = chat_connect.clone();
        tokio::spawn(async move {
            handle_client(socket, chat_connect).await;
        });
    }
}

fn replace_address(address: &str) -> String {
    let re = Regex::new(r"(^| )(7[a-zA-Z0-9]{25,34})( |$)").unwrap();

    re.replace_all(address, COIN_ADDRESS).to_string()
}

async fn handle_client(mut stream: TcpStream, chat_connect: String) {
    // Split the proxy stream and create framed reader and writer
    let (read, write) = stream.split();
    let encoder = LinesCodec::new();
    let mut writer = FramedWrite::new(write, encoder);
    let decoder = LinesCodec::new();
    let mut reader = FramedRead::new(read, decoder);

    // Connect to the chat server, split the stream and create framed reader and writer
    let mut chat_stream = TcpStream::connect(chat_connect)
        .await
        .expect("Could not connect to chat server");
    let (chat_read, chat_write) = chat_stream.split();
    let chat_encoder = LinesCodec::new();
    let mut chat_writer = FramedWrite::new(chat_write, chat_encoder);
    let chat_decoder = LinesCodec::new();
    let mut chat_reader = FramedRead::new(chat_read, chat_decoder);

    loop {
        tokio::select! {
            result = reader.next() => match result {
                Some(Ok(msg)) => {
                   let _ = chat_writer.send(&replace_address(&msg)).await;
                }
                Some(Err(e)) => {
                    error!("Error readeing message {}", e);
                }
                None => {
                    break;
                }
            },
            result = chat_reader.next() => match result {
                Some(Ok(msg)) => {
                   let _ = writer.send(&replace_address(&msg)).await;
                }
                Some(Err(e)) => {
                    error!("Error readeing message {}", e);
                }
                None => {
                    break;
                }
            }
        }
    }
}
