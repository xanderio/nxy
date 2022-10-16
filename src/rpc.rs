use std::{fmt::Display, str::FromStr};

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(untagged)]
pub enum JsonRPC {
    Request(Request),
    Response(Response),
    Notification(Notification),
}

impl From<Request> for JsonRPC {
    fn from(request: Request) -> JsonRPC {
        JsonRPC::Request(request)
    }
}

impl From<Response> for JsonRPC {
    fn from(response: Response) -> JsonRPC {
        JsonRPC::Response(response)
    }
}

impl From<Notification> for JsonRPC {
    fn from(notification: Notification) -> JsonRPC {
        JsonRPC::Notification(notification)
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(transparent)]
pub struct RequestId(u64);

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Request {
    pub id: RequestId,
    pub method: String,
    #[serde(default = "serde_json::Value::default")]
    #[serde(skip_serializing_if = "serde_json::Value::is_null")]
    pub params: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Response {
    pub id: RequestId,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<ResponseError>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ResponseError {
    pub code: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

#[derive(Clone, Copy, Debug)]
#[allow(unused)]
#[non_exhaustive]
pub enum ErrorCode {
    // Defined by JSON RPC:
    ParseError = -32700,
    InvalidRequest = -32600,
    MethodNotFound = -32601,
    InvalidParams = -32602,
    InternalError = -32603,
    ServerErrorStart = -32099,
    ServerErrorEnd = -32000,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Notification {
    pub method: String,
    #[serde(default = "serde_json::Value::default")]
    #[serde(skip_serializing_if = "serde_json::Value::is_null")]
    pub params: serde_json::Value,
}

impl From<u64> for RequestId {
    fn from(v: u64) -> Self {
        RequestId(v)
    }
}

impl FromStr for JsonRPC {
    type Err = color_eyre::Report;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        serde_json::from_str(s).map_err(Into::into)
    }
}

impl Display for JsonRPC {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        #[derive(Serialize)]
        struct JsonRpc {
            jsonrpc: &'static str,
            #[serde(flatten)]
            msg: JsonRPC,
        }
        let text = serde_json::to_string(&JsonRpc {
            jsonrpc: "2.0",
            msg: self.clone(),
        })
        .unwrap();
        write!(f, "{text}")
    }
}

impl Request {
    pub fn new(id: RequestId, method: String, params: &[impl Serialize]) -> Request {
        let params = serde_json::to_value(params).unwrap();
        Request { id, method, params }
    }
}

impl Response {
    pub fn new_ok<R: Serialize>(id: RequestId, result: R) -> Response {
        Response {
            id,
            result: Some(serde_json::to_value(result).unwrap()),
            error: None,
        }
    }
    pub fn new_err(id: RequestId, code: i32, message: String) -> Response {
        let error = ResponseError {
            code,
            message,
            data: None,
        };
        Response {
            id,
            result: None,
            error: Some(error),
        }
    }
}
impl Notification {
    pub fn new(method: String, params: impl Serialize) -> Notification {
        Notification {
            method,
            params: serde_json::to_value(params).unwrap(),
        }
    }
}
