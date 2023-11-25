pub mod realtime;

use std::time::Duration;

use crate::realtime::{Event, Realtime};
use axum::response::Html;
use axum::routing::get;
use axum::Router;
use futures_util::stream::{self, Stream};
use serde::Serialize;
use tokio_stream::StreamExt as _;

#[derive(Serialize, Clone)]
pub struct User {
    name: String,
    age: i8,
}

#[tokio::main]
async fn main() {
    let app = Router::new()
        .route("/", get(handler))
        .route("/stream", get(stream));

    println!("Listening at http://localhost:3000");
    axum::Server::bind(&"127.0.0.1:3000".parse().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap();
}

async fn handler() -> Html<&'static str> {
    Html("<h1>Hello, World!</h1>")
}

async fn stream() -> Realtime<impl Stream<Item = Result<Event, serde_json::Error>>> {
    let user = User {
        name: "Adam Aho".to_string(),
        age: 31,
    };

    let stream =
        stream::repeat_with(move || Event::ndjson(user.clone())).throttle(Duration::from_secs(1));

    Realtime::new(stream)
}
