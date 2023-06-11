use axum::{
    Json,
    response::{IntoResponse, Response}
};
use reqwest::StatusCode;
use serde::Serialize;
use serde_json::json;

#[derive(Debug, Serialize)]
pub struct ServerErr {
    
    pub status: u16,
    pub message: String,
    pub details: Vec<String>

}

impl ServerErr {

    /// Build a HTTP '400 Bad Request' error
    pub fn _bad_request<M>(message: M) -> ServerErr where M: Into<String> {

        ServerErr { 
            status: 400,
            message: message.into(),
            details: vec![]
        }
    }

    /// Build a HTTP '401 Unauthorized' error
    pub fn unauthorized<M>(message: M) -> ServerErr where M: Into<String> {

        ServerErr { 
            status: 401,
            message: message.into(),
            details: vec![]
        }
    }

    /// Build a HTTP '404 Not Found' error
    pub fn not_found<M>(message: M) -> ServerErr where M: Into<String> {

        ServerErr {
            status: 404,
            message: message.into(),
            details: vec![]
        }
    }

}

impl IntoResponse for ServerErr {

    fn into_response(self) -> Response {

        let status = StatusCode::from_u16(self.status).unwrap();
        let body = Json(json!({
            "statusCode": self.status,
            "message": self.message,
            "details": self.details
        }));

        (status, body).into_response()
    }

}
