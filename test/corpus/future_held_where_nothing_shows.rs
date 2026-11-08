// A future of the crate's runs as it's made in JS, where Rust's runs once
// it's awaited, and that's kept where nothing can tell (#965, ADR 0029
// amended): held past what can't be seen, arithmetic or a ready future's
// await; pinned; dropped unawaited where its body can't be seen; given to
// an async fn that only awaits it. Each compiles; the async table
// (test/async-points.ts) runs them against native Rust.
use std::future::{Future, ready};
use std::pin::pin;

async fn greet(n: u32) -> u32 {
    println!("hello {n}");
    n
}

async fn five() -> u32 {
    5
}

async fn awaited(f: impl Future<Output = u32>) -> u32 {
    f.await
}

pub async fn past_arithmetic() -> u32 {
    let f = greet(1);
    let n = 2 + 3;
    f.await + n
}

pub async fn past_a_ready_await() -> u32 {
    let f = greet(2);
    let n = ready(3).await;
    f.await + n
}

pub async fn pinned() -> u32 {
    let mut done = pin!(ready(4));
    let block = pin!(async { greet(5).await });
    done.as_mut().await + block.await
}

pub async fn dropped_quiet() {
    let f = five();
    drop(f);
}

pub async fn given_on() -> u32 {
    awaited(five()).await
}

fn main() {
    let _ = (past_arithmetic, past_a_ready_await, pinned, dropped_quiet, given_on);
    println!("compiled");
}
