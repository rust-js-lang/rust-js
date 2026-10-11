//! `async fn`, `.await`, `async` blocks and closures, and `spawn` (ADR 0029),
//! and the webapi crate's promises: `fetch`, binary data and WebAssembly.

use std::cell::RefCell;
use std::rc::Rc;

use js::{JsObject, Promise, Uint8Array, spawn, uint8_array};
use webapi::{AddEventListenerOptions, EventTargetExt, RequestInit, abort_controller, event, event_target, web_assembly, window};

unsafe extern "Rust" {
    /// Resolves with `value` after `ms` milliseconds.
    #[link_name = "node:timers/promises#setTimeout"]
    safe fn later(ms: u32, value: u32) -> Promise<u32>;
}

async fn double(x: u32) -> u32 {
    later(1, x).await * 2
}

/// `.await` in the middle of an expression, in Rust's order.
pub async fn sum(a: u32, b: u32) -> u32 {
    double(a).await + double(b).await
}

/// What an `.await` gives, given back.
pub async fn twice(n: u32) -> u32 {
    let d = double(n).await;
    double(d).await
}

/// A parameter that changes, and one taken apart.
pub async fn countdown(mut n: u32) -> u32 {
    let mut steps = 0;
    while n > 0 {
        n = later(0, n - 1).await;
        steps += 1;
    }
    steps
}

pub async fn swap((a, b): (u32, u32)) -> (u32, u32) {
    (later(0, b).await, a)
}

pub struct Context {
    pub params: Option<u32>,
    pub locale: Option<String>,
}

/// A struct taken apart where it's given, as Next.js's `getStaticProps`
/// takes its context.
pub async fn given(Context { params, .. }: Context) -> u32 {
    later(0, params.unwrap_or(0)).await + 1
}

/// An `async` block, and an `async` closure.
pub async fn blocks(x: u32) -> u32 {
    let block = async move { double(x).await + 1 };
    let add = async |y: u32| later(0, y).await + x;
    block.await + add(10).await
}

/// A future held in a variable, awaited later.
pub async fn held() -> u32 {
    let first = later(5, 1);
    let second = double(2);
    first.await + second.await
}

/// A spawned task runs up to its first `.await` right away (a JS promise is
/// already running), and the rest later.
pub fn spawned() -> Rc<RefCell<Vec<u32>>> {
    let log = Rc::new(RefCell::new(Vec::new()));
    let task_log = log.clone();
    spawn(Box::new(async move {
        task_log.borrow_mut().push(1);
        later(0, 0).await;
        task_log.borrow_mut().push(3);
    }));
    log.borrow_mut().push(2);
    log
}

/// A closure of an `async` block is an async function, called as react.dev's
/// useSandpackLint calls its `loadLinter`: up to its first `.await` at once.
pub fn started() -> Rc<RefCell<Vec<u32>>> {
    let log = Rc::new(RefCell::new(Vec::new()));
    let task_log = log.clone();
    let loading = move || async move {
        task_log.borrow_mut().push(1);
        later(0, 0).await;
        task_log.borrow_mut().push(3);
    };
    spawn(Box::new(loading()));
    log.borrow_mut().push(2);
    log
}

/// `fetch`, from the webapi crate: its promises, awaited one after the other.
pub async fn load(url: &str) -> (u16, bool, String) {
    let response = window.fetch(url).await;
    let body = response.text().await;
    (response.status(), response.ok(), body)
}

/// A dictionary a function takes (ADR 0102): a struct of what's given, the
/// rest `..Default::default()`, which JS reads as not given.
pub async fn post(url: &str, body: &str) -> String {
    let init = RequestInit { method: Some("POST"), body: Some(body.into()), ..Default::default() };
    window.fetch_with_init(url, init).await.text().await
}

/// A listener that goes when its signal aborts: `addEventListener`'s options.
pub fn listen_until_aborted() -> u32 {
    let count = Rc::new(RefCell::new(0));
    let target = event_target::new();
    let controller = abort_controller::new();
    let counted = Rc::clone(&count);
    let options = AddEventListenerOptions { signal: Some(controller.signal()), ..Default::default() };
    target.add_event_listener_named_with_options("ping", move |_| *counted.borrow_mut() += 1, options);
    let _ = target.dispatch_event(event::new("ping"));
    controller.abort();
    let _ = target.dispatch_event(event::new("ping"));
    *count.borrow()
}

/// Binary data: `bytes()`, `arrayBuffer()`, and a view of a buffer.
pub async fn load_bytes(url: &str) -> (u32, u32, u32) {
    let response = window.fetch(url).await;
    let copy = response.clone();
    let bytes = response.bytes().await;
    let buffer = copy.array_buffer().await;
    let view = uint8_array::new_with_buffer(buffer);
    (bytes.length(), buffer.byte_length(), view.length())
}

/// What a WebAssembly module imports: `{ env: { double } }`. The module
/// reads the fields; Rust never does.
#[allow(dead_code)]
struct Imports {
    env: Env,
}

#[allow(dead_code)]
struct Env {
    double: Box<dyn Fn(i32) -> i32>,
}

unsafe extern "Rust" {
    /// The module's export, called on its `exports` object.
    #[link_name = "add"]
    safe fn wasm_add(this: &JsObject, a: i32, b: i32) -> i32;
}

/// WebAssembly: compile a module, instantiate it with imports from Rust
/// (a struct, which is a JS object), and call its export, which calls back.
pub async fn run_wasm(bytes: &Uint8Array, a: i32, b: i32) -> i32 {
    let module = web_assembly::compile(bytes).await;
    let imports = Imports { env: Env { double: Box::new(|x| x * 2) } };
    let instance = web_assembly::instantiate_with_web_assembly_module_and_import_object(module, &imports).await;
    wasm_add(instance.exports(), a, b)
}

/// The other `instantiate`: from bytes, to a `{ module, instance }` dictionary.
pub async fn instantiate_bytes(bytes: &Uint8Array) -> bool {
    let imports = Imports { env: Env { double: Box::new(|x| x) } };
    let source = web_assembly::instantiate_with_import_object(bytes.buffer(), &imports).await;
    web_assembly::validate(bytes) && wasm_add(source.instance.exports(), 1, 2) == 3
}
