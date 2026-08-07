use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload")]
pub enum GraphQLClientMessage {
    #[serde(rename = "connection_init")]
    ConnectionInit,

    #[serde(rename = "start")]
    Start { id: String, payload: GraphQLStartPayload },

    #[serde(rename = "stop")]
    Stop { id: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphQLStartPayload {
    pub query: String,
    #[serde(default)]
    pub variables: Option<serde_json::Value>,
    #[serde(default)]
    pub operation_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum GraphQLServerMessage {
    #[serde(rename = "connection_ack")]
    ConnectionAck,

    #[serde(rename = "data")]
    Data { id: String, payload: GraphQLDataPayload },

    #[serde(rename = "error")]
    Error { id: String, payload: GraphQLErrorPayload },

    #[serde(rename = "complete")]
    Complete { id: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphQLDataPayload {
    pub data: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphQLErrorPayload {
    pub errors: Vec<serde_json::Value>,
}

impl GraphQLClientMessage {
    pub fn from_text(text: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(text)
    }
}

impl GraphQLServerMessage {
    pub fn to_text(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }
}
