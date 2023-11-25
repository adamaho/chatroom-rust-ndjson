use std::task::Poll;

use axum::{
    body::{self, Bytes, HttpBody},
    http::header,
    response::{IntoResponse, Response}, BoxError,
};
use bytes::{BufMut, BytesMut};
use futures::{Stream, TryStream};
use pin_project_lite::pin_project;
use serde::Serialize;
use sync_wrapper::SyncWrapper;

#[derive(Debug, Clone)]
pub struct Realtime<S> {
    stream: S,
}

impl<S> Realtime<S> {
    /// Creates a new realtime response that will respond with a stream
    /// of [`NdJson`] events
    pub fn new(stream: S) -> Self
    where
        S: TryStream<Ok = Event> + Send + 'static,
    {
        return Realtime { stream };
    }
}

impl<S, E> IntoResponse for Realtime<S>
where
    S: Stream<Item = Result<Event, E>> + Send + 'static,
    E: Into<BoxError>
{
    fn into_response(self) -> Response {
        (
            [
                (header::CONTENT_TYPE, "application/ndjson")
            ],
            body::boxed(Body {
                stream: SyncWrapper::new(self.stream)
            })
        ).into_response()
    }
}

pin_project! {
    pub struct Body<S> {
        #[pin]
        stream: SyncWrapper<S>
    }
}

impl<S, E> HttpBody for Body<S>
where
    S: Stream<Item = Result<Event, E>>,
{
    type Data = Bytes;
    type Error = E;

    fn poll_data(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<std::option::Option<Result<Self::Data, Self::Error>>> {
        let body = self.project();

        match body.stream.get_pin_mut().poll_next(cx) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(Some(Ok(event))) => {
                Poll::Ready(Some(Ok(event.commit())))
            }
            Poll::Ready(Some(Err(err))) => Poll::Ready(Some(Err(err))),
            Poll::Ready(None) => Poll::Ready(None),
        }
    }

    fn poll_trailers(
            self: std::pin::Pin<&mut Self>,
            _cx: &mut std::task::Context<'_>,
        ) -> Poll<Result<Option<header::HeaderMap>, Self::Error>> {
        Poll::Ready(Ok(None))
    }
}

pub struct Event {
    buffer: BytesMut,
}

impl Event {
    pub fn ndjson<T>(data: T) -> serde_json::Result<Event>
    where
        T: Serialize,
    {
        let mut buffer = bytes::BytesMut::new();
        serde_json::to_writer((&mut buffer).writer(), &data)?;
        buffer.put_u8(b'\n');
        Ok(Event { buffer })
    }

    pub fn commit(self) -> Bytes {
        self.buffer.freeze()
    }
}
