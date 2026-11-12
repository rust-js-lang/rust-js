// A type's own `Future` is never lowered (ADR 0425), and so neither is a
// private function only it calls: no JS calls them. Its `io::Read`'s and
// `Seek`'s are lowered since (#1297, #1489).
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

struct Ready<T> {
    value: Option<T>,
    polled: u32,
}

impl<T> Ready<T> {
    fn new(value: T) -> Self {
        Ready {
            value: Some(value),
            polled: 0,
        }
    }

    fn take_ready(&mut self) -> Poll<T> {
        self.polled += 1;
        match self.value.take() {
            Some(value) => Poll::Ready(value),
            None => Poll::Pending,
        }
    }
}

impl<T: Unpin> Future for Ready<T> {
    type Output = T;

    fn poll(mut self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<T> {
        self.take_ready()
    }
}

fn main() {
    let ready = Ready::new(5u32);
    println!("{} {:?}", ready.polled, ready.value);
}
