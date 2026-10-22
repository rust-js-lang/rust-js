# Known limitations

What rust-js doesn't do yet. It never writes JS that runs differently from
Rust by accident: what it can't compile as Rust runs, it refuses, with an
error that names it, `rust-js does not support .. yet`
([ADR 0006](decisions/0006-errors-and-unsupported-features.md)). This page
lists what's refused. How a program that compiles behaves is
[the semantics](semantics.md), area by area, and where it deliberately
differs from native Rust, [its one list](semantics.md#differences-from-native-rust-in-one-list).

Many of these will go; this page goes with them. Each item that's refused
at compile time ends with an example, which `test/limitations.test.ts`
compiles: one that compiles fails the test, so the page is kept true. A
change that lifts a refusal removes its line here; one that adds a refusal
a person can meet adds one.

Where to see more:

- **rustc's own run-pass tests**: each in scope that fails is in
  [`test/rustc-known-failures.txt`](../test/rustc-known-failures.txt), with
  its first error ([ADR 0089](decisions/0089-rustc-tests.md)). Most need
  addressable memory, below.
- **Crates from crates.io**: which compile and run, and what stops the rest,
  in [the crate corpus](crate-corpus.md).
- **React and JSX**: [their own page](jsx.md).

## Memory and unsafe code

JS has no addressable memory: a value is an object or a number, not bytes
at an address. rust-js gives raw pointers, `transmute` and the like the
meaning Rust's have where it can model them: a raw pointer to a local or
into an array, its `add` and `offset_from`, a `transmute` between types of
numbers alone (`u16` to `[u8; 2]`, a struct of two `u32`s to a `u64`),
or between a `#[repr(transparent)]` struct and its field, by value or
through a `&`, `slice::from_raw_parts` of an array's pointer. What reads memory by where
it is, it refuses:

- A raw address of a `static`, or a `&mut` to a `static mut`: `static mut N: u32 = 0; let r = unsafe { &mut *std::ptr::addr_of_mut!(N) }; *r += 1;`
- A `transmute` of an enum, whose bytes a niche or a tag lays out: `#[repr(u8)] enum Size { One = 1 } let n: u8 = unsafe { std::mem::transmute::<Option<Size>, u8>(None) }; let _ = (n, Size::One);`
- A `transmute` of a `&mut` to a `#[repr(transparent)]` struct's field as one to the struct, whose whole a write through it replaces: `#[repr(transparent)] struct W(String); let mut s = String::new(); let w = unsafe { std::mem::transmute::<&mut String, &mut W>(&mut s) }; *w = W(String::from("x"));`

And refused when it runs, as a panic that says so, since which it is
depends on what the program does:

- A `TypeId`'s hash shown, which rust-js doesn't keep. Only an assertion
  that fails shows one: `assert_eq!` of two compiles. A pointer's address
  is shown, a number of rust-js's own (ADR 0562).
- Reading through a pointer with no place, an integer made a pointer, or
  the address of a value JS holds as itself, a number reached through a
  `&`.
- A union's field read while it holds one whose bytes JS has none of, a
  `&str`'s or a `String`'s; plain data's bytes read as another type's
  (ADR 0562).
- A pointer to what has no bytes, a `String`, read as bytes; a pointer
  stepped outside an array or into what JS holds as a copy.

## Types

- A `dyn Debug` of a value with a destructor, which the string it shows can't keep: `struct D; impl Drop for D { fn drop(&mut self) {} } impl std::fmt::Debug for D { fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result { f.write_str("D") } } let b: Box<dyn std::fmt::Debug> = Box::new(D); println!("{b:?}");`
- A struct whose last field is a `dyn Debug`, the string it shows: `struct W<T: ?Sized> { n: u32, v: T } let w: Box<W<dyn std::fmt::Debug>> = Box::new(W { n: 1, v: 5u8 }); println!("{:?}", &w.v);`
- A `RefCell` of a `dyn` of the crate's trait given to generic code as a cell of a type parameter, which its pair isn't: `trait N { fn n(&self) -> u32; } struct A; impl N for A { fn n(&self) -> u32 { 1 } } fn get<T: ?Sized + N>(c: &std::cell::RefCell<T>) -> u32 { c.borrow().n() } let c: &std::cell::RefCell<dyn N> = &std::cell::RefCell::new(A); println!("{}", get(c));`
- The `TypeId` of a type made of a type parameter, a closure's in generic code: `fn make<T: 'static>(t: T) -> std::any::TypeId { fn id<C: 'static>(_: &C) -> std::any::TypeId { std::any::TypeId::of::<C>() } let c = move || drop(t); id(&c) } println!("{:?}", make(1u8) == make(2u8));`
- `size_of_val` of a value without one size, a `dyn`'s: `let b: Box<dyn std::fmt::Debug> = Box::new(5u64); println!("{}", std::mem::size_of_val(&*b));`
- A map key whose `PartialEq` is the crate's own: `use std::collections::HashMap; #[derive(Hash)] struct K(u32); impl PartialEq for K { fn eq(&self, o: &K) -> bool { self.0 % 10 == o.0 % 10 } } impl Eq for K {} let mut m = HashMap::new(); m.insert(K(1), 1); println!("{}", m.contains_key(&K(11)));`
- A `&mut` to an object given for a type parameter inside what generic code takes or gives, an `Option`, which it has as a box and its caller as the object: `fn some<D>(d: Option<D>) -> Option<D> { d } let mut v = vec![1]; if let Some(r) = some(Some(&mut v)) { r.push(2); }`
- A `&mut` to an object given for a type parameter that's a closure's parameter: `fn apply<D, F: Fn(D) -> usize>(d: D, f: F) -> usize { f(d) } let mut v = vec![1]; println!("{}", apply(&mut v, |w: &mut Vec<i32>| w.len()));`

## Iterators and collections

- A clone of a lazy iterator generic code makes where its source isn't bound by `Clone`: `fn doubled<I: Iterator<Item = i32>>(it: I) -> std::iter::Map<I, fn(i32) -> i32> { it.map(|x| x * 2) } let d = doubled(1..4); let e = d.clone(); println!("{:?} {:?}", d.count(), e.count());`
- Looking a key up by what the crate's own `Borrow` gives: `use std::borrow::Borrow; use std::collections::HashMap; #[derive(PartialEq, Eq, Hash)] struct K(String); impl Borrow<str> for K { fn borrow(&self) -> &str { &self.0 } } let mut m = HashMap::new(); m.insert(K("a".into()), 1); println!("{:?}", m.get("a"));`

## Numbers and text

- A C string, a `c"..."` literal: `let c = c"hi"; println!("{:?}", c);`
- `CString` and `CStr`: `let c = std::ffi::CString::new("hi").unwrap(); println!("{:?}", c);`

## Formatting

- A placeholder's options, a width or `{:x?}`, for a `dyn Debug`, which is the string it shows already (ADR 0060): `let b: Box<dyn std::fmt::Debug> = Box::new(31u8); println!("{:x?}", b);`
- A `Formatter`'s flags asked by a `Display` of the crate's: `struct X; impl std::fmt::Display for X { fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result { write!(f, "{}", f.sign_minus()) } } println!("{:-}", X);`

## Hashing

std's hashers and a crate's own hash as native Rust's
([ADR 0463](decisions/0463-hashing-is-stds.md)); a `HashMap` or a `HashSet`
keys by value and never hashes ([ADR 0121](decisions/0121-value-keys.md)).

- A `BuildHasher`, `RandomState`'s: `use std::hash::BuildHasher; let s = std::hash::RandomState::new(); println!("{}", s.hash_one(5u32) == s.hash_one(5u32));`
- `{:?}` of a std hasher: `println!("{:?}", std::hash::DefaultHasher::new());`
- Hashing a std type rust-js doesn't write: `use std::hash::{Hash, Hasher}; let mut h = std::hash::DefaultHasher::new(); std::time::Duration::from_secs(1).hash(&mut h); println!("{}", h.finish());`

## Panics, processes and I/O

- `catch_unwind`: `let r = std::panic::catch_unwind(|| 1); println!("{}", r.is_ok());`
- `panic::set_hook`: `std::panic::set_hook(Box::new(|_| {}));`
- `process::abort`: `std::process::abort();`
- Standard input: `let mut s = String::new(); let _ = std::io::stdin().read_line(&mut s);`
- The file system: `let _ = std::fs::read_to_string("x");`

## Async, threads and time

There's one thread ([the semantics](semantics.md#async-threads-and-time)).

- Spawning a thread: `let h = std::thread::spawn(|| 1); println!("{}", h.join().unwrap());`
- Sleeping: `std::thread::sleep(std::time::Duration::from_millis(1));`
- A time's `{:?}`, its platform's fields: `println!("{:?}", std::time::Instant::now());`
- `future::poll_fn`: `let f = std::future::poll_fn(|_| std::task::Poll::Ready(1)); let _ = f;`
- An `IntoFuture` of the crate's given to generic code bound by `IntoFuture`, which awaits it as a promise: `struct F; impl std::future::IntoFuture for F { type Output = u32; type IntoFuture = std::future::Ready<u32>; fn into_future(self) -> Self::IntoFuture { std::future::ready(1) } } async fn g<T: std::future::IntoFuture<Output = u32>>(t: T) -> u32 { t.await } let _ = g(F);`
- A `Wake` of the crate's: `struct W; impl std::task::Wake for W { fn wake(self: std::sync::Arc<Self>) {} } let w = std::task::Waker::from(std::sync::Arc::new(W)); drop(w);`

## serde

serde_json's common attributes work ([the semantics](semantics.md#the-javascript-boundary)).
Refused, each held by `test/diagnostics.test.ts`, which compiles them with
serde: a `u128` in JSON; reading a `BinaryHeap`; a map keyed by a struct;
`Value::pointer`, and assigning to a `Value`'s key.

## React and JSX

- A component is a function; a namespace of them, `<AlertDialog.Root>`, is
  a module of them.

## Platforms and builds

- Releases are for macOS on Apple silicon ([the changelog](../CHANGELOG.md)).
- What else isn't built yet is in [the architecture's "Not yet"](architecture.md#not-yet)
  and [the roadmap](../ROADMAP.md).
