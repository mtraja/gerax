use std::fmt;

use serde::{Deserialize, Serialize};
use tungstenite::protocol::CloseFrame as TungsteniteCloseFrame;
use tungstenite::Bytes;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CloseFrame {
    pub code: u16,
    pub reason: String,
}

impl From<CloseFrame> for TungsteniteCloseFrame {
    fn from(value: CloseFrame) -> Self {
        TungsteniteCloseFrame {
            code: value.code.into(),
            reason: value.reason.into(),
        }
    }
}

impl From<TungsteniteCloseFrame> for CloseFrame {
    fn from(value: TungsteniteCloseFrame) -> Self {
        CloseFrame {
            code: value.code.into(),
            reason: value.reason.to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", content = "data")]
pub enum WsMessage {
    Text(String),
    Binary(Vec<u8>),
    Close(Option<CloseFrame>),
    Ping,
    Pong,
}

impl fmt::Display for WsMessage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            WsMessage::Text(text) => write!(f, "Text({})", text),
            WsMessage::Binary(data) => write!(f, "Binary({} bytes)", data.len()),
            WsMessage::Close(frame) => write!(f, "Close({:?})", frame),
            WsMessage::Ping => write!(f, "Ping"),
            WsMessage::Pong => write!(f, "Pong"),
        }
    }
}

impl From<WsMessage> for tungstenite::Message {
    fn from(value: WsMessage) -> Self {
        match value {
            WsMessage::Text(text) => tungstenite::Message::Text(text.into()),
            WsMessage::Binary(data) => tungstenite::Message::Binary(Bytes::from(data)),
            WsMessage::Close(frame) => {
                tungstenite::Message::Close(frame.map(|f| f.into()))
            }
            WsMessage::Ping => tungstenite::Message::Ping(Bytes::new()),
            WsMessage::Pong => tungstenite::Message::Pong(Bytes::new()),
        }
    }
}

impl From<tungstenite::Message> for WsMessage {
    fn from(value: tungstenite::Message) -> Self {
        match value {
            tungstenite::Message::Text(text) => WsMessage::Text(text.to_string()),
            tungstenite::Message::Binary(data) => WsMessage::Binary(data.to_vec()),
            tungstenite::Message::Close(frame) => WsMessage::Close(frame.map(|f| f.into())),
            tungstenite::Message::Ping(_) => WsMessage::Ping,
            tungstenite::Message::Pong(_) => WsMessage::Pong,
            tungstenite::Message::Frame(_) => WsMessage::Pong,
        }
    }
}

pub type WsFrame = tungstenite::Message;
