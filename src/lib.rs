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

fn replace_address(text: &str) -> String {
    let re = Regex::new(r"(^| )(7[a-zA-Z0-9]{25,34})( |$)").unwrap();

    let mut result = String::new();
    let mut last_end = 0;

    for mat in re.captures_iter(text) {
        let full_match = mat.get(0).unwrap();
        let replacement = format!(
            "{}{}{}",
            mat.get(1).map_or("", |m| m.as_str()),
            COIN_ADDRESS,
            mat.get(3).map_or("", |m| m.as_str())
        );

        result.push_str(&text[last_end..full_match.start()]);
        result.push_str(&replacement);
        last_end = full_match.end();
    }
    result.push_str(&text[last_end..]);

    result
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple() {
        let res = replace_address("7F1u3wSD5RbOHQmupo9nx4TnhQ");
        assert_eq!(res, String::from("7YWHMfk9JZe0LM0g1ZauHuiSxhI"));
    }

    #[test]
    fn test_simple_longer() {
        let res = replace_address("7adNeSwJkMakpEcln9HEtthSRtxdmEHOT8T");
        assert_eq!(res, String::from("7YWHMfk9JZe0LM0g1ZauHuiSxhI"));
    }

    #[test]
    fn test_product_id() {
        let text = "7adNeSwJkMakpEcln9HEtthSRtxdmEHOT8T-ID";
        let res = replace_address(text);
        assert_eq!(res, text.to_string());
    }

    #[test]
    fn test_in_string() {
        let text = "send the money to 7adNeSwJkMakpEcln9HEtthSRtxdmEHOT8T now";
        let res = replace_address(text);
        assert_eq!(
            res,
            String::from("send the money to 7YWHMfk9JZe0LM0g1ZauHuiSxhI now")
        );
    }

    #[test]
    fn test_multiple() {
        let text = "send the money to 7adNeSwJkMakpEcln9HEtthSRtxdmEHOT8T and 7iKDZEwPZSqIvDnHvVN2r0hUWXD5rHX and 7LOrwbDlS8NujgjddyogWgIM93MV5N2VR";
        let res = replace_address(text);
        assert_eq!(
            res,
            String::from(
                "send the money to 7YWHMfk9JZe0LM0g1ZauHuiSxhI and 7YWHMfk9JZe0LM0g1ZauHuiSxhI and 7YWHMfk9JZe0LM0g1ZauHuiSxhI"
            )
        );
    }
}
