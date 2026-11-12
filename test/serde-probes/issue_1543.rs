// #1543, its repro: A stateful serde skip predicate changes the JSON: native writes `{}`, rust-js writes `{"n":7}`
use serde::Serialize;
use std::cell::Cell;
thread_local! { static ASKED: Cell<bool> = const { Cell::new(false) }; }
fn skip(_: &u32) -> bool {
    ASKED.with(|x| x.replace(true))
}
#[derive(Serialize)]
struct Data {
    #[serde(skip_serializing_if = "skip")]
    n: u32,
}
pub fn main() {
    println!("{:?}", serde_json::to_string(&Data { n: 7 }));
}
