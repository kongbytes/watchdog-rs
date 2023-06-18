use std::sync::Arc;

use axum::{
    body::Body,
    extract::State,
    http::{Request, StatusCode},
    middleware::Next,
    response::IntoResponse,
};

use super::{utils::ServerErr, config::ServerConf};

pub async fn check_authorization(State(state): State<Arc<ServerConf>>, request: Request<Body>, next: Next<Body>) -> Result<impl IntoResponse, impl IntoResponse> {

    let authorization_header = request.headers().get("authorization").map(|header| header.to_str().unwrap_or_default());

    match authorization_header {
        Some(token) => {

            if token != format!("Bearer {}", state.token) {
                return Err(ServerErr::unauthorized("Invalid authentication"));
            }
            
            let response = next.run(request).await;
            Ok(response)

        }
        None => Err(ServerErr::unauthorized("Invalid authentication"))
    }
}

pub async fn log_request(req: Request<Body>, next: Next<Body>) -> Result<impl IntoResponse, (StatusCode, String)> {

    let uri = req.uri().clone();
    let method = req.method().clone();

    let response = next.run(req).await;

    let status = response.status();
    if status.is_success() || status.is_redirection() || status.is_informational() {
        println!("\"{} {}\" {}", method, uri, response.status().as_u16());
    } else {
        eprintln!("\"{} {}\" {}", method, uri, response.status().as_u16());
    }

    Ok(response)
}
