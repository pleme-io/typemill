use crate::Result;
use futures::Stream;
use std::pin::Pin;
use std::task::{Context, Poll};

pub trait PaginatedResponse {
    type Item;
    fn items(self) -> Vec<Self::Item>;
    fn next_page_token(&self) -> Option<&str>;
}

impl PaginatedResponse for crate::types::SourcesResponse {
    type Item = crate::types::Source;
    fn items(self) -> Vec<Self::Item> {
        self.sources
    }
    fn next_page_token(&self) -> Option<&str> {
        self.next_page_token.as_deref()
    }
}

impl PaginatedResponse for crate::types::SessionsResponse {
    type Item = crate::types::Session;
    fn items(self) -> Vec<Self::Item> {
        self.sessions
    }
    fn next_page_token(&self) -> Option<&str> {
        self.next_page_token.as_deref()
    }
}

impl PaginatedResponse for crate::types::ActivitiesResponse {
    type Item = crate::types::Activity;
    fn items(self) -> Vec<Self::Item> {
        self.activities
    }
    fn next_page_token(&self) -> Option<&str> {
        self.next_page_token.as_deref()
    }
}

pub struct Paginator<R, F>
where
    R: PaginatedResponse,
    F: Fn(Option<String>) -> Pin<Box<dyn std::future::Future<Output = Result<R>> + Send>>,
{
    fetch: F,
    buffer: Vec<R::Item>,
    next_page_token: Option<String>,
    done: bool,
}

impl<R, F> Paginator<R, F>
where
    R: PaginatedResponse,
    F: Fn(Option<String>) -> Pin<Box<dyn std::future::Future<Output = Result<R>> + Send>>,
{
    pub fn new(fetch: F) -> Self {
        Self {
            fetch,
            buffer: Vec::new(),
            next_page_token: Some("".to_string()), // Start with an initial empty token
            done: false,
        }
    }
}

impl<R, F> Stream for Paginator<R, F>
where
    R: PaginatedResponse + Unpin,
    F: Fn(Option<String>) -> Pin<Box<dyn std::future::Future<Output = Result<R>> + Send>> + Unpin,
    R::Item: Unpin,
{
    type Item = Result<R::Item>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        loop {
            if !self.buffer.is_empty() {
                return Poll::Ready(Some(Ok(self.buffer.remove(0))));
            }

            if self.done {
                return Poll::Ready(None);
            }

            let token = self.next_page_token.clone();
            // If the token is Some("") it's the first request, we should pass None.
            let api_token = if token.as_deref() == Some("") { None } else { token };

            let mut fetch_fut = (self.fetch)(api_token);

            match fetch_fut.as_mut().poll(cx) {
                Poll::Ready(Ok(response)) => {
                    self.next_page_token = response.next_page_token().map(|s| s.to_string());
                    if self.next_page_token.is_none() {
                        self.done = true;
                    }
                    self.buffer = response.items();
                    // Loop again to process the buffer
                }
                Poll::Ready(Err(e)) => {
                    self.done = true;
                    return Poll::Ready(Some(Err(e)));
                }
                Poll::Pending => return Poll::Pending,
            }
        }
    }
}