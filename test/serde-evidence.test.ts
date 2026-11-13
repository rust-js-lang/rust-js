// Where generic code's codec of a type comes from, against native
// serde_json, the oracle, built of the same Rust and dependency versions:
// a bound its own signature names, or one a trait declares on an
// associated type, which its dictionary carries (#1522, ADR 0081).
import { beforeAll, expect, test } from "bun:test";
import { writeFileSync } from "node:fs";
import { join } from "node:path";
import { compiled, native } from "./serde-support";
import { buildCompiler, buildSerde, fixture, run } from "./support";

beforeAll(() => {
  buildCompiler();
  buildSerde("rlib");
}, 600_000);

// An endpoint's trait, as cross-agent-message's is: 'type Request:
// Serialize' and 'type Response: DeserializeOwned', which generic code
// writes and reads with no bound of its own; the codec is the impl's
// dictionary's entry, 'EEndpoint.RequestSerialize()'. Beside it, the bound
// repeated on the function, items a 'Vec' and '()', a supertrait's item,
// an error, and a 'Value'.
test("serde: a codec a trait declares on its associated type is its dictionary's", async () => {
  const dir = fixture("serde-evidence-associated-type");
  writeFileSync(join(dir, "cases.rs"), String.raw`
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

pub trait Endpoint {
    const PATH: &'static str;
    type Request: Serialize;
    type Response: DeserializeOwned;
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Send {
    pub body: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Message {
    pub id: u64,
}

pub struct SendMessage;

impl Endpoint for SendMessage {
    const PATH: &'static str = "/api/messages";
    type Request = Send;
    type Response = Message;
}

/// Its items a 'Vec' and '()'.
pub struct Batch;

impl Endpoint for Batch {
    const PATH: &'static str = "/api/batch";
    type Request = Vec<Send>;
    type Response = ();
}

/// A supertrait's associated type, read through the subtrait.
pub trait Named: Endpoint {
    fn name() -> &'static str;
}

impl Named for SendMessage {
    fn name() -> &'static str {
        "send"
    }
}

pub fn wire<E: Endpoint>(request: &E::Request) -> String {
    serde_json::to_string(request).unwrap_or_default()
}

pub fn read<E: Endpoint>(answer: &str) -> Result<E::Response, String> {
    serde_json::from_str(answer).map_err(|error| error.to_string())
}

/// The bound repeated, as it was written before.
pub fn wire_where<E: Endpoint>(request: &E::Request) -> String
where
    E::Request: Serialize,
{
    serde_json::to_string(request).unwrap_or_default()
}

pub fn named_wire<N: Named>(request: &N::Request) -> String {
    format!("{} {} {}", N::name(), N::PATH, wire::<N>(request))
}

pub fn round<E: Endpoint>(request: &E::Request, answer: &str) -> String
where
    E::Response: std::fmt::Debug,
{
    format!("{} {:?}", wire::<E>(request), read::<E>(answer))
}

pub fn to_value<E: Endpoint>(request: &E::Request) -> String {
    format!("{:?}", serde_json::to_value(request))
}

pub fn report() -> String {
    let send = Send { body: "hi".to_string() };
    [
        wire::<SendMessage>(&send),
        format!("{:?}", read::<SendMessage>(r#"{"id":7}"#)),
        format!("{:?}", read::<SendMessage>(r#"{"id":"x"}"#)),
        wire_where::<SendMessage>(&send),
        wire::<Batch>(&vec![Send { body: "a".to_string() }, Send { body: "b".to_string() }]),
        format!("{:?}", read::<Batch>("null")),
        named_wire::<SendMessage>(&send),
        round::<SendMessage>(&send, r#"{"id":1}"#),
        to_value::<SendMessage>(&send),
    ]
    .join("\n")
}
`);
  const expected = JSON.parse(run([native(dir)]));
  expect((await compiled(dir)).report()).toBe(expected);
});
