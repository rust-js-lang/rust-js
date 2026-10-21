import { beforeAll, expect, test } from "bun:test";
import { existsSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { runSync } from "./child";
import { buildCompiler, buildSerde, buildReact, compiler, fixture, root, run, target } from "./support";

beforeAll(buildCompiler, 600_000);

for (const [name, source, message, crate] of [
  ["type error", 'pub fn f() -> i32 { "wrong" }', "mismatched types"],
  ["borrow error", 'pub fn f() -> i32 { let mut x = 1; let r = &x; x = 2; *r }', "borrowed"],
  ["unsupported type", 'pub fn f(x: *const str) -> *const str { x }', "does not support addressable memory yet: values of type `*const str`"],
  // A `const { .. }` block's type is its value's, not its placeholder's.
  ["a const block of a static's dyn", 'trait T { fn get(&self) -> u32; }\nstruct A(u32);\nimpl T for A { fn get(&self) -> u32 { self.0 } }\nstatic S: A = A(1);\npub fn f() -> u32 { let x = [const { &S as &dyn T }; 1]; x[0].get() }', "does not support constants of type `&dyn T`"],
  // ADR 0122: an `f32`'s bits aren't yet: a NaN's payload is JS's to keep or not.
  // ADR 0390: addressable memory, which JavaScript has none of, says so.
  // ADR 0408: plain data is its numbers; a reference's bytes are an address.
  ["a transmute of a reference", "pub fn f(x: &u32) -> usize { unsafe { std::mem::transmute(x) } }", "does not support addressable memory yet: a `transmute` of `&u32` as `usize`, which aren't numbers alone"],
  ["a zeroed std struct", "pub fn f() -> std::time::Duration { unsafe { std::mem::zeroed() } }", "does not support a zeroed `std::time::Duration`, which may be no value of it"],
  ["a zeroed std wrapper", "pub fn f() -> std::num::Wrapping<u32> { unsafe { std::mem::zeroed() } }", "does not support a zeroed `std::num::Wrapping<u32>`, which may be no value of it"],
  // ADR 0407: a raw pointer is a handle on its place; what reads memory by where it is isn't JS's.
  // Its address is a number of rust-js's own (ADR 0562).
  ["a pointer to text", "pub fn f(s: &str) -> *const str { s as *const str }", "does not support addressable memory yet: values of type `*const str`"],
  ["an IntoFuture of the crate's given to generic code", "use std::future::{IntoFuture, Ready, ready};\npub struct F;\nimpl IntoFuture for F { type Output = u32; type IntoFuture = Ready<u32>; fn into_future(self) -> Ready<u32> { ready(1) } }\nasync fn g<T: IntoFuture<Output = u32>>(t: T) -> u32 { t.await }\npub async fn h() -> u32 { g(F).await }", "does not support `F` where any `IntoFuture` goes yet: its `IntoFuture` is the crate's own"],
  ["a ManuallyDrop of what's refused", "pub fn f(x: std::mem::ManuallyDrop<*const str>) -> std::mem::ManuallyDrop<*const str> { x }", "does not support addressable memory yet: values of type `*const str`"],
  // A struct's pointer as a field's that rustc doesn't lay at its offset 0 (ADR 0562).
  ["a struct's pointer as a later field's", "#[repr(C)] pub struct R { name: String, n: u32 }\npub fn f(p: *const R) -> u32 { unsafe { *(p as *const u32) } }", "does not support addressable memory yet: a pointer read as another type's"],
  ["a pointer into a subslice's copy", "pub fn f() -> bool { let x = [1u8, 2, 3]; let [_, ref y @ ..] = x; let p = &y[0] as *const u8; p.is_null() }", "does not support addressable memory yet: a pointer into what JS may hold a copy of"],
  ["a pointer to a constant", "const C: &u8 = &1;\npub fn f() -> bool { let p = C as *const u8; p.is_null() }", "does not support addressable memory yet: a pointer into what JS may hold a copy of"],
  ["a pointer through a &mut", "pub fn f(r: &mut u32) -> *mut u32 { r }", "does not support addressable memory yet: a raw pointer to what a `&mut` refers to"],
  ["a pointer to a value a box holds", "pub fn f(b: Box<u32>) -> bool { let p: *const u32 = &*b; p.is_null() }", "does not support addressable memory yet: a raw pointer to this place"],
  // A JS module's namespace, which an `export { .. } from` can't name (ADR 0240).
  ["a pub use of a JS module's namespace", 'mod bound {\n    #[cfg_attr(rust_js, rust_js::link_name = "./math.js#*")]\n    pub fn math() -> f64 { unreachable!() }\n}\npub use bound::math;\npub fn f() -> u32 { 1 }', "does not support a `pub use` of a JS module's namespace yet: it'd be `export * as ns from`; re-export each of its exports by name"],
  ["fields that are one JS property", 'pub struct P { pub first: u32, #[rust_js::name = "first"] pub other: u32 }\npub fn f(p: &P) -> u32 { p.first + p.other }', "both `first` in JS"],
  // A binding's nullable parameter is one of its own, an `Option` (ADR 0275).
  ["a nullable parameter that isn't an Option", '#[rust_js::link_name = "f"]\n#[rust_js::nullable(n)]\n#[allow(unused_variables)]\npub fn f(n: u32) {}', "names no parameter of this function that's an `Option`"],
  ["a nullable parameter that isn't there", '#[rust_js::link_name = "f"]\n#[rust_js::nullable(m)]\n#[allow(unused_variables)]\npub fn f(n: Option<u32>) {}', "`#[rust_js::nullable(m)]` names no parameter"],
  // A discriminated union's variants are named fields beside its tag (ADR 0284).
  ["a tagged enum's tuple variant", '#[rust_js::tag = "kind"]\npub enum E { A(u32) }\npub fn f() -> E { E::A(1) }', "has a tuple variant, whose fields have no names"],
  // Only its tag may be `true` or `false`: another enum's variant is its name, a string.
  ["a boolean name of an enum's variant", 'pub enum E { #[rust_js::name = true] A, B }\npub fn f() -> E { E::A }', "`#[rust_js::name]` is a string"],
  ["a number name of a tagged enum's variant", '#[rust_js::tag = "kind"]\npub enum E { #[rust_js::name = 1] A { n: u32 }, B { n: u32 } }\npub fn f() -> E { E::A { n: 1 } }', "`#[rust_js::name]` is a string"],
  // Its `otherwise` is any other object, which a derive would take for one of its own.
  ["an otherwise variant not last", '#[rust_js::tag = "kind"]\npub enum E { #[rust_js::otherwise] O(&\'static js::JsObject), A { n: u32 } }\npub fn f() -> E { E::A { n: 1 } }', "`otherwise` variant is its last", "js"],
  ["a derive of an open union", '#[rust_js::tag = "kind"]\n#[derive(Clone)]\npub enum E { A { n: u32 }, #[rust_js::otherwise] O(&\'static js::JsObject) }\npub fn f(e: &E) -> E { e.clone() }', "deriving `Clone` of a discriminated union with an `otherwise` variant yet: it would take the object it holds", "js"],
  ["#[thread_local] static", "#![feature(thread_local)]\n#[thread_local] static N: std::cell::Cell<u32> = std::cell::Cell::new(0);\npub fn f() -> u32 { N.get() }", "does not support `#[thread_local]` statics"],
  // One to another's number is that number (ADR 0096); to another's object,
  // whose copy `ptr::eq` would tell apart, isn't yet.
  ["static holding a reference to another's object", "pub struct P { pub x: u32 }\nstatic A: P = P { x: 1 };\nstatic B: &P = &A;\npub fn f() -> u32 { B.x }", "does not support statics of type `&'static P`"],
  // `B::from_iter(items)` is `items.collect()` of an array or an iterator (ADR 0160); an `Option`'s isn't yet.
  ["from_iter of an Option", "pub fn f(o: Option<u32>) -> Vec<u32> { Vec::from_iter(o) }", "calling `std::iter::FromIterator::from_iter`"],
  // `try_fold` into a `fmt::Result`, whose `Err` is thrown (ADR 0539).
  ["try_fold into a fmt::Result", "pub struct S(pub Vec<u32>);\nimpl std::fmt::Display for S { fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result { self.0.iter().try_fold((), |(), x| write!(f, \"{x}\")) } }", "`try_fold` into a `fmt::Result`"],
  // A std trait's dictionary has what it's given, a `dyn Error`'s `source` (ADR 0141), and none of what else std provides.
  ["a provided method of a dyn io::Write", "pub fn f(w: &mut dyn std::io::Write) -> usize { w.write_vectored(&[]).unwrap() }", "`write_vectored` of a"],
  // A placeholder's options are given to what shows the value (ADR 0058), but a
  // `&dyn Debug` is the string it shows already, and serde_json's `fmt`s are rust-js's.
  ["a width for a dyn Debug", 'pub fn f(v: u8) -> String { let d: &dyn std::fmt::Debug = &v; format!("{:5?}", d) }', "options for a"],
  // `ptr::eq` of a Rust value: an unchanged copy is one JS object, and a
  // number is its value, so a place's address isn't JS's (ADR 0285).
  // A trait default's own const parameter numbered as one its impl's `Self` names (ADR 0135).
  ["a default's const parameter numbered as its impl's", "pub trait Scaled<const N: usize> { fn base(&self) -> usize; fn scaled(&self) -> usize { self.base() * N } }\npub struct S<T, const K: usize>(T);\nimpl<T, const K: usize> Scaled<3> for S<T, K> { fn base(&self) -> usize { K } }\npub fn f(s: &S<u8, 4>) -> usize { s.scaled() }", "a trait default's const parameter numbered as its impl's"],
  // Text's `as_ptr()` is a copy's: compared, it'd be another's (ADR 0407).
  ["text's pointer compared", "pub fn f(a: &str, b: &str) -> bool { a.as_ptr() == b.as_ptr() }", "does not support addressable memory yet: a pointer into what JS may hold a copy of"],
  // A crate's own `Future` is never lowered: no JS promise is one (ADR 0425).
  ["an await of a crate's future", "use std::{future::Future, pin::Pin, task::{Context, Poll}};\npub struct S(u32);\nimpl Future for S { type Output = u32; fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<u32> { Poll::Ready(self.0) } }\npub async fn f() -> u32 { S(1).await }", "rust-js does not support `S` as a promise yet: its `Future` is the crate's own"],
  ["a crate's future given as an F: Future", "use std::{future::Future, pin::Pin, task::{Context, Poll}};\npub struct S(u32);\nimpl Future for S { type Output = u32; fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<u32> { Poll::Ready(self.0) } }\nasync fn g<F: Future<Output = u32>>(f: F) -> u32 { f.await }\npub async fn f() -> u32 { g(S(1)).await }", "rust-js does not support `S` as a promise yet"],
  ["a pinned crate's future awaited", "use std::{future::Future, pin::Pin, task::{Context, Poll}};\npub struct S(u32);\nimpl Future for S { type Output = u32; fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<u32> { Poll::Ready(self.0) } }\npub async fn f() -> u32 { let mut s = S(1); Pin::new(&mut s).await }", "rust-js does not support `S` as a promise yet"],
  ["a crate's future as a dyn Future", "use std::{future::Future, pin::Pin, task::{Context, Poll}};\npub struct S(u32);\nimpl Future for S { type Output = u32; fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<u32> { Poll::Ready(self.0) } }\npub fn f() -> Pin<Box<dyn Future<Output = u32>>> { Box::pin(S(1)) }", "rust-js does not support `S` as a promise yet"],
  // Generic code over a `T: ?Sized` resolves a blanket impl's method where rustc calls a `dyn`'s own (ADR 0145).
  ["a type's fact of a dyn given to generic code", "trait Trait { fn foo(&self) -> &'static str; }\nimpl<T: ?Sized> Trait for T { fn foo(&self) -> &'static str { std::any::type_name::<T>() } }\nfn bar<T: ?Sized>() -> fn(&T) -> &'static str { Trait::foo }\npub fn f() -> &'static str { bar::<dyn Trait>()(&1i32) }", "a type's fact of `dyn Trait` given to generic code"],
  // A C-variadic function's last parameter gathers the rest, which nothing can come after (ADR 0428).
  ["a C-variadic function given a dictionary", "pub unsafe extern \"C\" fn shown<T: std::fmt::Display>(x: T, mut rest: ...) -> String { format!(\"{x} {}\", unsafe { rest.next_arg::<i32>() }) }\npub fn f() -> String { unsafe { shown(1u8, 2) } }", "a C-variadic function given a dictionary, a fact or a drop"],
  // Nor its `io::Read`: what would call it is refused (ADR 0425).
  ["a read of a crate's reader", "use std::io::{self, Read};\npub struct S;\nimpl Read for S { fn read(&mut self, _: &mut [u8]) -> io::Result<usize> { Ok(0) } }\npub fn f() -> usize { let mut b = [0u8; 2]; S.read(&mut b).unwrap() }", "rust-js does not support calling `std::io::Read::read` yet"],
  // An adapter holding an iterator with a destructor, which it drops (ADR 0098).
  ["an adapter of an iterator with a destructor", "struct D(u32);\nimpl Iterator for D { type Item = u32; fn next(&mut self) -> Option<u32> { None } }\nimpl Drop for D { fn drop(&mut self) {} }\npub fn f() -> Vec<u32> { D(1).scan(0, |s, x| { *s += x; Some(*s) }).collect() }", "`scan` of an iterator with a destructor"],
  ["ptr::eq of a number", "pub fn f(a: &u32, b: &u32) -> bool { std::ptr::eq(a, b) }", "does not support addressable memory yet: a raw pointer to this place"],
  ["ptr::eq of a number's pointer, copied", "pub fn f(a: &u32, b: *const u32) -> bool { let p = a as *const u32; let q = p; std::ptr::eq(q, b) }", "does not support addressable memory yet: a raw pointer to this place"],
  // A JS error is the string JS shows of it, `String(error)` (ADR 0271).
  ["a width for a JsError", 'pub fn f(e: &js::JsError) -> String { format!("{:>9?}", e) }', "options for a", "js"],
  ["a width for a serde_json Value", 'pub fn f(v: &serde_json::Value) -> String { format!("{:>9}", v) }', "options for a", "serde"],
  // A static's initializer is code where rustc's value can't say it (ADR 0096),
  // but not one reading another static, nor of a value that can change in place.
  ["a static of a function read from another static", 'pub fn f() {}\nstatic A: fn() = f;\npub static B: &fn() = &A;\npub fn g() { B() }', "statics of type"],
  // A path map finds `a//b` as the `a/b` it holds, and a `Cow` map `"a"` as the
  // `Cow` it holds, which neither can give back yet (ADRs 0401, 0402).
  ["the key a path map holds", 'use std::{collections::HashMap, path::{Path, PathBuf}};\npub fn f(m: &HashMap<PathBuf, u32>) -> Option<(&PathBuf, &u32)> { m.get_key_value(Path::new("a//b")) }', "does not support the key a `std::collections::HashMap<std::path::PathBuf, u32>` holds, which what it's found by may not be"],
  ["the key a Cow map holds", 'use std::{borrow::Cow, collections::HashSet};\npub fn f(s: &HashSet<Cow<\'static, str>>) -> bool { s.get("a").is_some() }', "does not support the key a `std::collections::HashSet<std::borrow::Cow<'_, str>>` holds"],
  // A B-tree finds a key by its `Ord`, which, hand-written, `$key` wouldn't follow (ADR 0404).
  ["a B-tree key whose Ord is the crate's own", 'use std::cmp::Ordering;\nuse std::collections::BTreeSet;\n#[derive(PartialEq, Eq)]\npub struct K { pub a: u8, pub b: u8 }\nimpl PartialOrd for K { fn partial_cmp(&self, o: &Self) -> Option<Ordering> { Some(self.cmp(o)) } }\nimpl Ord for K { fn cmp(&self, o: &Self) -> Ordering { self.a.cmp(&o.a) } }\npub fn f(s: &mut BTreeSet<K>) -> bool { s.insert(K { a: 1, b: 2 }) }', "does not support values of type `K`"],
  ["a B-tree key holding a fieldless enum whose Ord is the crate's own", 'use std::cmp::Ordering;\nuse std::collections::BTreeSet;\n#[derive(PartialEq, Eq, Clone, Copy)]\npub enum E { A, B }\nimpl PartialOrd for E { fn partial_cmp(&self, o: &Self) -> Option<Ordering> { Some(self.cmp(o)) } }\nimpl Ord for E { fn cmp(&self, _: &Self) -> Ordering { Ordering::Equal } }\npub fn f(s: &mut BTreeSet<(E, u8)>) -> bool { s.insert((E::A, 1)) }', "does not support values of type `(E, u8)`"],
  // A union is the field it holds (ADR 0406); a `&mut` to one would change which unsaid.
  ["a &mut to a union's field", 'pub union U { pub a: u32, pub b: u32 }\npub fn f(u: &mut U) { let r = unsafe { &mut u.a }; *r = 2; }', "does not support a `&mut` to a union's field"],
  // A counted `Rc`'s is its cell given the `impl` in place (ADR 0320); a `Weak`'s not yet.
  ["a Weak of a RefCell of a dyn", 'use std::{cell::RefCell, rc::{Rc, Weak}};\npub trait T { fn f(&self) -> i32; }\nimpl T for i32 { fn f(&self) -> i32 { *self } }\npub fn w() -> bool { let a = Rc::new(RefCell::new(1)); let b: Weak<RefCell<dyn T>> = Rc::downgrade(&a) as Weak<_>; b.upgrade().is_some() }', "does not support a counted `std::rc::Weak<std::cell::RefCell<dyn T>>`"],
  // A cell made a `dyn` of two traits has one `impl`, both's methods (ADR 0397).
  // ADR 0409: a struct of a `dyn` last field, behind a reference; what owns one would drop it through its `impl`.
  ["a raw pointer to a struct of a dyn last field", 'pub struct Fat<U: ?Sized> { a: i32, last: U }\npub fn g() -> i32 { let f = Fat { a: 1, last: 2u8 }; let p: *const Fat<dyn Send> = &f; unsafe { (*p).a } }', "a `Fat<dyn std::marker::Send>`, whose last field is a `dyn`"],
  ["a RefCell made a dyn of two traits of one method name", 'use std::{cell::RefCell, rc::Rc};\npub trait A { fn f(&self) -> i32; }\npub trait B { fn f(&self) -> i32; }\npub struct S;\nimpl A for S { fn f(&self) -> i32 { 1 } }\nimpl B for S { fn f(&self) -> i32 { 2 } }\npub fn g() -> i32 { let s = Rc::new(RefCell::new(S)); let a: Rc<RefCell<dyn A>> = s.clone(); let b: Rc<RefCell<dyn B>> = s; a.borrow().f() + b.borrow().f() }', "may be a `dyn B` too: its one `impl` can't hold two methods of one name"],
  ["a generic RefCell made a dyn of two traits of one method name", 'use std::{cell::RefCell, rc::Rc};\npub trait A { fn f(&self) -> i32; }\npub trait B { fn f(&self) -> i32; }\npub fn g<S: A + B + \'static>(s: Rc<RefCell<S>>) -> i32 { let a: Rc<RefCell<dyn A>> = s.clone(); let b: Rc<RefCell<dyn B>> = s; a.borrow().f() + b.borrow().f() }', "may be a `dyn B` too: its one `impl` can't hold two methods of one name"],
  ["a RefCell of a dyn upcast", 'use std::{cell::RefCell, rc::Rc};\npub trait A { fn f(&self) -> i32; }\npub trait B: A {}\nimpl A for i32 { fn f(&self) -> i32 { *self } }\nimpl B for i32 {}\npub fn g() -> i32 { let s: Rc<RefCell<dyn B>> = Rc::new(RefCell::new(1)); let a: Rc<RefCell<dyn A>> = s; a.borrow().f() }', "does not support a `std::rc::Rc<std::cell::RefCell<dyn B>>` as a `std::rc::Rc<std::cell::RefCell<dyn A>>`"],
  // A constant's each use is a copy (ADR 0031); a static's one value isn't.
  ["a static of a Vec of functions", 'pub fn f() {}\npub static V: Vec<fn()> = Vec::new();\npub fn g() -> usize { V.len() }', "statics of type"],
  // ADR 0121: a key found by its value is one a derived `Eq` compares.
  ["map keyed by a struct of its own equality", '#[derive(Hash)] pub struct P { pub x: u32 }\nimpl PartialEq for P { fn eq(&self, o: &P) -> bool { self.x % 10 == o.x % 10 } }\nimpl Eq for P {}\npub fn f() -> usize { let m: std::collections::HashMap<P, u32> = std::collections::HashMap::new(); m.len() }', "does not support values of type `P`"],
  ["map keyed by a struct with a field of its own equality", '#[derive(Hash)] pub struct Q(pub u32);\nimpl PartialEq for Q { fn eq(&self, o: &Q) -> bool { self.0 % 10 == o.0 % 10 } }\nimpl Eq for Q {}\n#[derive(PartialEq, Eq, Hash)] pub struct P { pub q: Q }\npub fn f() -> usize { let m: std::collections::HashSet<P> = std::collections::HashSet::new(); m.len() }', "does not support values of type `P`"],
  ["parse to a type without FromStr support", 'pub fn f(s: &str) -> bool { s.parse::<std::net::IpAddr>().is_ok() }', "does not support"],
  // A string's byte offsets are counted (ADR 0138), but not yet of a closure's matches.
  // `*p` of a `P: DerefMut` is its dictionary's (ADR 0394), std's of a pointer
  // whose target is a cell or the same JS value: not a `LazyCell`'s, made when read.
  ["a LazyCell given as a DerefMut", 'fn f<P: std::ops::DerefMut<Target = u32>>(mut p: P) { *p = 1; }\npub fn g() { f(std::cell::LazyCell::new(|| 1u32)) }', "implementation evidence for `<std::cell::LazyCell"],
  ["an Rc's raw pointer", 'pub fn f(r: &std::rc::Rc<u32>) -> bool { std::rc::Rc::as_ptr(r).is_null() }', "does not support"],
  // A heap's `pop` of an item that could look like `None`, as any such `Option` (ADR 0051).
  ["a heap of options", 'pub fn f() -> bool { let mut h = std::collections::BinaryHeap::new(); h.push(Some(1u32)); h.pop().is_some() }', "this call, for an `Option` of what could look like `None`"],
  ["a pointer of the crate's own to a dyn", "#![feature(derive_coerce_pointee)]\nuse std::ops::Deref;\n#[derive(std::marker::CoercePointee)] #[repr(transparent)] pub struct Ptr<'a, #[pointee] T: ?Sized> { ptr: &'a T }\nimpl<T: ?Sized> Deref for Ptr<'_, T> { type Target = T; fn deref(&self) -> &T { self.ptr } }\npub trait Get { fn get(&self) -> u32; }\npub struct V(u32);\nimpl Get for V { fn get(&self) -> u32 { self.0 } }\npub fn f() -> u32 { let v = V(10); let p: Ptr<dyn Get> = Ptr { ptr: &v }; p.get() }", "does not support unsizing a `Ptr<'_, dyn Get>`"],
  // A `dyn` of std's traits has no dictionary to carry a drop (ADR 0098).
  ["flatten of an Option of an Option of a value with a destructor", 'pub struct D(pub u32);\nimpl Drop for D { fn drop(&mut self) {} }\npub fn f(a: Option<Option<D>>) -> bool { a.flatten().is_some() }', "`std::option::Option::<std::option::Option<T>>::flatten` of a value with a destructor"],
  ["a Box<dyn Debug> of a value with a destructor", 'pub struct D;\nimpl Drop for D { fn drop(&mut self) {} }\nimpl std::fmt::Debug for D { fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result { f.write_str("D") } }\npub fn f() { let _: Box<dyn std::fmt::Debug> = Box::new(D); }', "a `dyn` of a value with a destructor"],
  ["an Rc<dyn> of a value with a destructor", 'pub struct D;\nimpl Drop for D { fn drop(&mut self) {} }\nimpl std::fmt::Debug for D { fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result { f.write_str("D") } }\npub fn f() { let _: std::rc::Rc<dyn std::fmt::Debug> = std::rc::Rc::new(D); }', "a `dyn` of a value with a destructor"],
  // A std call given an iterator whose destructors rust-js can't follow would drop what it skips silently.
  // A generic iterator folded or looped over is one its callers give no destructor of (ADR 0190).
      // A `Cell` drops the value it's set over, which JS wouldn't (ADR 0320).
  ["a LazyCell of a value with a destructor", 'pub struct D(pub u32);\nimpl Drop for D { fn drop(&mut self) {} }\npub fn f() { let c = std::cell::LazyCell::new(|| D(1)); std::mem::forget(c) }', "a std type holding a value with a destructor, `std::cell::LazyCell<D"],
  // A function called by `&` given where it's called once: that call drops none of it (ADR 0470).
  ["an Fn given on where it's called once", 'pub struct D(pub u32);\nimpl Drop for D { fn drop(&mut self) {} }\nfn once<G: FnOnce() -> u32>(g: G) -> u32 { g() }\nfn relay<F: Fn() -> u32>(f: F) -> u32 { once(f) }\npub fn f(d: D) -> u32 { relay(move || { let r = &d; r.0 }) }', "giving `once`, which calls it once, a function called by `&` that holds a value with a destructor"],
  // A `take_while` holds its function, dropped with it, which it isn't made closing for yet (ADRs 0470, 0485).
  ["a function with a destructor a lazy take_while holds", 'pub struct D(pub u32);\nimpl Drop for D { fn drop(&mut self) {} }\npub fn f(v: &[u32], d: D) -> u32 { v.iter().take_while(move |x| { let r = &d; **x < r.0 }).sum() }', "a function that holds a value with a destructor, kept by what it's given to"],
  // A caller gives a type parameter's sort, not one of a type that holds one (ADR 0471).
  ["an unstable sort of items that hold a type parameter", 'pub fn f<T: Ord>(mut v: Vec<(T, u8)>) -> Vec<(T, u8)> { v.sort_unstable(); v }', "an unstable sort of items that hold a type parameter"],
  // `xor` of two `Some`s drops both, which it doesn't give back (ADR 0098).
  // A size of a type parameter is its caller's (ADR 0145), but `size_of_val` of an unsized one is the value's.
  ["size_of_val of an unsized type parameter", 'pub fn f<T: ?Sized>(x: &T) -> usize { std::mem::size_of_val(x) }\npub fn g() -> usize { f("ab") }', "`size_of` of a type parameter"],
  // `collect()` into a `Result` or an `Option` is of an array (ADR 0036).
  ["collecting into a Result of an Rc slice", 'pub fn f(v: Vec<Result<(u8, u8), ()>>) -> Result<std::rc::Rc<[(u8, u8)]>, ()> { v.into_iter().collect() }', "does not support `collect` into a `std::rc::Rc<[(u8, u8)]>`"],
  // `len()` takes no item, so it runs none of a chain's closures (ADR 0036).
  ["len of a chain whose closure prints", 'pub fn f(v: &[u32]) -> usize { v.iter().map(|x| { println!("{x}"); x }).len() }', "`len()` of a lazy or generic `std::iter::Map`"],
  // A builder's steps are a chain's (ADR 0136): one a loop steps is no chain.
  ["a map's key looked up through the crate's Borrow", "use std::borrow::Borrow;\n#[derive(PartialEq, Eq, Hash)]\npub struct Name(pub String);\nimpl Borrow<str> for Name { fn borrow(&self) -> &str { &self.0 } }\npub fn f(m: &std::collections::HashMap<Name, i32>) -> Option<&i32> { m.get(\"a\") }", "`std::collections::HashMap::<K, V, S, A>::get` of a `Name` as what its own `Borrow` gives"],
  ["items joined through the crate's Borrow", "use std::borrow::Borrow;\n#[derive(PartialEq, Eq, Hash)]\npub struct Name(pub String);\nimpl Borrow<str> for Name { fn borrow(&self) -> &str { &self.0 } }\npub fn f(v: &[Name]) -> String { v.join(\",\") }", "`std::slice::<impl [T]>::join` of a `Name` as what its own `Borrow` gives"],
  ["size_hint of an iterator whose hint isn't exact", "pub fn f(s: &str) -> usize { s.chars().size_hint().0 }", "`size_hint()` of a `std::str::Chars<'_>`"],
  ["a path's components", "use std::path::Path;\npub fn f(a: &Path) -> usize { a.components().count() }", "calling `std::path::Path::components`"],
  ["a user impl of a std trait", 'pub struct C;\nimpl std::net::ToSocketAddrs for C { type Iter = std::vec::IntoIter<std::net::SocketAddr>; fn to_socket_addrs(&self) -> std::io::Result<Self::Iter> { Ok(Vec::new().into_iter()) } }', "user implementations of `std::net::ToSocketAddrs`"],
  ["comparing another crate's struct", 'pub fn f(a: std::net::Ipv4Addr, b: std::net::Ipv4Addr) -> bool { a < b }', "does not support"],
  // Generic code given no `Clone` of its source can't clone the map it makes (ADR 0525).
  ["a map cloned where its source isn't bound by Clone", "pub fn doubled<I: Iterator<Item = i32>>(it: I) -> std::iter::Map<I, fn(i32) -> i32> { it.map(|x| x * 2) }\npub fn f() -> usize { let d = doubled(1..4); d.clone().count() }", "made where `I` isn't bound by `Clone`"],
  ["a new closure assigned through a &mut to one", 'pub fn replace<F: FnMut()>(f: &mut F, g: F) { *f = g; }', "assigning a whole value through a `&mut`"],
  ["a collection of the crate's extending a Vec", "struct Bag { items: Vec<i32> }\nimpl IntoIterator for Bag { type Item = i32; type IntoIter = std::vec::IntoIter<i32>; fn into_iter(self) -> Self::IntoIter { self.items.into_iter() } }\npub fn f(v: &mut Vec<i32>, b: Bag) { v.extend(b) }", "a `Bag`, whose `IntoIterator` is the crate's"],
  ["a collection of the crate's given to generic code", "struct Bag { items: Vec<i32> }\nimpl IntoIterator for Bag { type Item = i32; type IntoIter = std::vec::IntoIter<i32>; fn into_iter(self) -> Self::IntoIter { self.items.into_iter() } }\nfn count<I: IntoIterator>(i: I) -> usize { i.into_iter().count() }\npub fn f(b: Bag) -> usize { count(b) }", "a `Bag`, whose `IntoIterator` is the crate's"],
  ["a generic &mut T to an object in a struct", "pub struct C { pub n: i32 }\npub struct H<'a, T> { pub r: &'a mut T }\nfn get<'a, T>(h: H<'a, T>) -> &'a mut T { h.r }\npub fn f(c: &mut C) { get(H { r: c }).n += 1; }", "a `&mut C` inside a generic function's parameters or result"],
  // A closure's parameter that holds a `&mut` to an object, `F: FnOnce(Vec<&mut T>)`, past its own `&mut` (ADR 0454).
  ["a Vec of generic &mut Ts to objects given to a closure", "pub struct C { pub n: i32 }\nfn first<T, F: FnOnce(Vec<&mut T>)>(t: &mut T, f: F) { f(vec![t]) }\npub fn f(c: &mut C) { first(c, |v| { for x in v { x.n += 1; } }) }", "a `&mut C` inside a generic function's parameters or result"],
  ["generic &mut Ts to objects returned in a Vec", "pub struct C { pub n: i32 }\nfn to_vec<T>(a: &mut T) -> Vec<&mut T> { vec![a] }\npub fn f(c: &mut C) -> usize { to_vec(c).len() }", "a `&mut C` inside a generic function's parameters or result"],
  ["a generic &mut T to an object in an Option", "pub struct C { pub n: i32 }\nfn set<T>(o: Option<&mut T>, v: T) { if let Some(r) = o { *r = v; } }\npub fn f(c: &mut C) { set(Some(c), C { n: 7 }); }", "a `&mut C` inside a generic function's parameters or result"],
  // A generic associated type of lifetimes is an associated type (ADR 0146); one of a type with a bound isn't.
  ["a generic associated type of a type with a bound", "pub trait Wrap { type Of<T>: Clone; fn wrap<T: Clone>(t: T) -> Self::Of<T>; }", "generic associated types with bounds"],
  ["reversing a zip of items with a destructor", "pub struct Guard;\nimpl Drop for Guard { fn drop(&mut self) {} }\npub fn f() -> usize { vec![1].into_iter().zip(vec![Guard, Guard]).rev().count() }", "as a `DoubleEndedIterator`, which rust-js can't step from its back"],
  ["cycling an array's items with a destructor", "#[derive(Clone)]\npub struct Guard;\nimpl Drop for Guard { fn drop(&mut self) {} }\npub fn f() -> usize { vec![Guard].into_iter().cycle().take(2).count() }", "`cycle()` of an array's items with a destructor"],
  ["an extern declaration of the crate's own no_mangle function", "pub mod export { #[unsafe(no_mangle)] pub extern \"C\" fn twice(t: i32) -> i32 { t * 2 } }\nunsafe extern \"C\" { fn twice(t: i32) -> i32; }\npub fn f() -> i32 { unsafe { twice(3) } }", "not in an `extern` block"],
  ["a generic constant", "#![feature(generic_const_items)]\n#![allow(incomplete_features)]\npub trait Sizes { const SIZE<T>: usize; }", "generic constants"],
  ["a type constant, refused not crashed", "#![feature(min_generic_const_args)]\n#![allow(incomplete_features)]\npub trait Foo { type const N: usize; }\npub struct Bar;\nimpl Foo for Bar { type const N: usize = 3; }", "type constants"],
  ["a generic const expression", "#![feature(generic_const_exprs)]\n#![allow(incomplete_features)]\nfn count<const N: usize>() -> usize { N }\nfn one_more<const N: usize>() -> usize where [(); N + 1]: { count::<{ N + 1 }>() }\npub fn f() -> usize { one_more::<2>() }", "this const argument"],
  ["an externally implementable item", "#![feature(extern_item_impls)]\n#[eii(hello)]\nstatic HELLO: u64;\n#[hello]\nstatic HELLO_IMPL: u64 = 5;\npub fn f() -> u64 { HELLO }", "externally implementable items"],
  ["malformed import", '#[rust_js::import("./style.css")]\nconst _: () = ();\npub fn f() {}', "write it"],
  ["malformed binding", '#[rust_js::link_name(123)] pub fn f() {}', "a binding needs"],
  ["handwritten JSX binding", '#[rust_js::link_name = "<div>"] fn div(a: i32, b: i32) -> i32 { unreachable!() }\npub fn f() -> i32 { div(1, 2) }', "element builders are compiler-only"],
  // An untagged enum (ADR 0214): JS tells a variant by its payload's kind, so each holds one
  // value of a kind of its own, and a `From` into one is its variant of its argument.
  ["an untagged enum's variants of one kind", '#[rust_js::untagged] pub enum E { A(String), B(&\'static str) }\npub fn f(e: E) -> u32 { match e { E::A(_) => 1, E::B(_) => 2 } }', "which JS can't tell apart"],
  // Only made, they may share a kind; a `match` that tells them apart can't
  // be, nor an impl.
  ["an untagged enum's variants of one kind, matched", '#[rust_js::untagged] pub enum E { A(&\'static str), B(char) }\npub fn f(e: E) -> u32 { match e { E::A(_) => 1, E::B(_) => 2 } }', "which JS can't tell apart"],
  ["an untagged enum's variants of one kind, cloned", '#[rust_js::untagged] #[derive(Clone)] pub enum E { A(String), B(&\'static str) }\npub fn f(s: String) -> E { E::A(s) }', "which JS can't tell apart"],
  // A variadic binding's rest arguments are its last parameter, a slice (ADR 0221).
  ["a variadic binding of no slice", '#[rust_js::link_name = "Math.max"]\n#[rust_js::variadic]\n#[allow(unused_variables)]\nfn max(a: f64, b: f64) -> f64 { unreachable!() }\npub fn f() -> f64 { max(1.0, 2.0) }', "isn't a slice"],
  // A JS function is bound one way, an ordinary function with
  // `#[rust_js::link_name]` (ADR 0373): one in an `extern` block is
  // refused, its error saying the one way.
  // A JS type is one form, a struct of a `JsObject` (ADR 0373): an `extern`
  // type, of a nightly feature, is refused, its error saying the one form.
  ["an extern type", '#![feature(extern_types)]\nunsafe extern "Rust" {\n    type Url;\n}\npub fn f(_: &Url) {}', "not an `extern` type"],
  ["a function in an extern block", 'unsafe extern "Rust" {\n    #[link_name = "Math.max"]\n    pub safe fn max(a: f64, b: f64) -> f64;\n}\npub fn f() -> f64 { 1.0 }', 'not in an `extern` block'],
  ["an untagged enum's otherwise variant before another", '#[rust_js::untagged] pub enum E { #[rust_js::otherwise] A(String), B(u32) }\npub fn f(e: E) -> u32 { match e { E::A(_) => 0, E::B(n) => n } }', "is its last"],
  // One of no value is its name's string (ADR 0214); one of two isn't one.
  ["an untagged enum's variant of two values", '#[rust_js::untagged] pub enum E { A(u32), B(u32, u32) }\npub fn f(e: E) -> u32 { match e { E::A(n) => n, E::B(a, _) => a } }', "holds one value"],
  ["an untagged enum's variant of an Option", '#[rust_js::untagged] pub enum E { A(Option<u32>), B(String) }\npub fn f(e: &E) -> u32 { match e { E::A(_) => 1, E::B(_) => 2 } }', "from another variant's value"],
  ["a From into an untagged enum that does more", '#[rust_js::untagged] pub enum E { A(u32), B(String) }\nimpl From<u32> for E { fn from(n: u32) -> Self { E::A(n + 1) } }\npub fn f(n: u32) -> E { n.into() }', "is its variant of its argument"],
  ["serde of an untagged enum", '#[rust_js::untagged]\n#[derive(serde::Serialize)] pub enum E { A(u32), B(String) }\npub fn f(e: &E) -> String { serde_json::to_string(e).unwrap() }', "serde of the untagged enum", "serde"],
  ["writing a u128", 'pub fn f(n: u128) -> String { serde_json::to_string(&n).unwrap() }', "does not support", "serde"],
  ["reading a u128", 'pub fn f(s: &str) -> bool { serde_json::from_str::<u128>(s).is_ok() }', "does not support", "serde"],
  ["reading a BinaryHeap", 'pub fn f(s: &str) -> bool { serde_json::from_str::<std::collections::BinaryHeap<u32>>(s).is_ok() }', "deserializing", "serde"],
  ["unsupported Value method", 'pub fn f(v: &serde_json::Value) -> bool { v.pointer("/name").is_some() }', "`Value::pointer`", "serde"],
  ["assigning to a Value's key", 'pub fn f() -> String { let mut v = serde_json::json!({}); v["k"] = serde_json::json!(1); v.to_string() }', "`std::ops::IndexMut::index_mut`", "serde"],
  // A JSON object's keys are strings: serde_json refuses a struct's (ADR 0121).
  ["a map keyed by a struct in JSON", '#[derive(PartialEq, Eq, Hash, serde::Serialize)] pub struct P { pub x: u32 }\npub fn f(m: &std::collections::HashMap<P, u32>) -> String { serde_json::to_string(m).unwrap() }', "a map key of", "serde"],
] as [string, string, string, string?][]) {
  test(`${name} reports a source location and preserves existing output`, () => {
    const dir = fixture("diagnostic");
    const input = join(dir, "lib.rs"), output = join(dir, "lib.js"), manifest = join(dir, "manifest.json");
    writeFileSync(input, source);
    const crates = crate === "js" ? (buildReact(), ["--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]) : crate ? buildSerde() : [];
    const args = [compiler, input, "-o", output, "--manifest", manifest, ...(crates.length ? ["--", ...crates] : [])];
    rmSync(output, { force: true });
    rmSync(output + ".map", { force: true });
    rmSync(manifest, { force: true });
    const rejected = Bun.spawnSync(args);
    expect(rejected.exitCode).not.toBe(0);
    expect(rejected.stderr.toString()).toContain(message);
    expect(rejected.stderr.toString()).toContain("lib.rs:");
    expect(existsSync(output)).toBe(false);
    expect(existsSync(manifest)).toBe(false);
    writeFileSync(output, "last successful build");
    writeFileSync(output + ".map", "previous map");
    expect(Bun.spawnSync(args).exitCode).not.toBe(0);
    expect(readFileSync(output, "utf8")).toBe("last successful build");
    expect(readFileSync(output + ".map", "utf8")).toBe("previous map");
  });
}

// rust-js is a stable release's rustc (ADR 0109): a crate's own
// `#![feature]` is refused, as that release refuses it, unless
// `RUSTC_BOOTSTRAP=1`, as for rustc. rust-js has no feature of its own to
// turn on (ADR 0110), so one it once used is refused too.
test("a crate's own #![feature] is refused, as on a stable release", () => {
  const dir = fixture("stable-feature");
  const compile = (source: string, bootstrap?: string) => {
    const input = join(dir, "lib.rs");
    writeFileSync(input, source);
    const { RUSTC_BOOTSTRAP: _, ...env } = process.env;
    return Bun.spawnSync([compiler, input, "-o", join(dir, "lib.js")], { env: bootstrap ? { ...env, RUSTC_BOOTSTRAP: bootstrap } : env });
  };
  const never = "#![feature(never_type)]\npub fn f() -> u32 { 1 }\n";
  const refused = compile(never);
  expect(refused.exitCode).not.toBe(0);
  expect(refused.stderr.toString()).toContain("error[E0554]: `#![feature]` may not be used on the stable release channel");
  expect(refused.stderr.toString()).toContain("lib.rs:1:1");
  expect(compile(never, "1").exitCode).toBe(0);
  expect(compile("#![feature(register_tool)]\n#![register_tool(rust_js)]\npub fn f() -> u32 { 1 }\n").exitCode).not.toBe(0);
  expect(compile("pub fn f() -> u32 { 1 }\n").exitCode).toBe(0);
});

// rust-js's syntax is stable Rust's (ADR 0110): what it turns on for itself
// isn't a program's to use. Each of these is refused as stable 1.99.0
// refuses it, which its rustc confirms; `rust_js`'s attributes are rust-js's.
test("a program can use no unstable feature rust-js's syntax once used", () => {
  const dir = fixture("stable-syntax");
  const { RUSTC_BOOTSTRAP: _, ...env } = process.env;
  const input = join(dir, "lib.rs");
  const verdicts = (source: string) => {
    writeFileSync(input, source);
    const js = Bun.spawnSync([compiler, input, "-o", join(dir, "lib.js")], { env });
    const rustc = Bun.spawnSync(["rustc", "--edition=2024", "--crate-type=lib", "--emit=metadata", input, "-o", join(dir, "lib.rmeta")], { env, cwd: root });
    const first = (text: string) => text.split("\n").find((line) => line.startsWith("error")) ?? "";
    return { rustJs: js.exitCode === 0, rustc: rustc.exitCode === 0, stderr: js.stderr.toString(), same: first(js.stderr.toString()) === first(rustc.stderr.toString()) };
  };
  for (const [feature, source] of [
    ["stmt_expr_attributes", "pub fn f() -> u32 { let x = #[allow(unused)] 5; x }\n"],
    ["decl_macro", "macro m() {}\npub fn f() {}\n"],
    ["register_tool", "#![register_tool(foo)]\npub fn f() {}\n"],
    ["custom_inner_attributes", "mod m {\n    #![rustfmt::skip]\n}\npub fn f() {}\n"],
  ]) {
    const v = verdicts(source);
    expect([feature, v.rustc]).toEqual([feature, false]);
    expect([feature, v.rustJs]).toEqual([feature, false]);
    // The same error, rustc's own: a stable release names no feature to turn on.
    expect([feature, v.same]).toEqual([feature, true]);
  }
  // A binding's attribute is rust-js's own tool's, which a program needs no
  // feature to write.
  const binding = verdicts('#[rust_js::link_name = "Date.now"]\npub fn now() -> f64 {\n    unreachable!()\n}\n');
  expect([binding.rustJs, binding.stderr]).toEqual([true, ""]);
});

// A binding crate rustc builds, as webapi's is: its function in an `extern`
// block is refused where a program rust-js compiles calls it, its error
// saying the one form (ADR 0373).
test("another crate's function in an extern block is refused where it's called", () => {
  const dir = fixture("extern-fn-dep");
  writeFileSync(join(dir, "dep.rs"), 'unsafe extern "Rust" {\n    #[link_name = "Math.max"]\n    pub safe fn max(a: f64, b: f64) -> f64;\n}\n');
  run([compiler, "--rustc", "--edition=2024", "--crate-type=lib", "--crate-name=dep", "--emit=metadata", "--target=wasm32-unknown-unknown",
    join(dir, "dep.rs"), "-o", join(dir, "libdep.rmeta")]);
  const compile = (rust: string) => {
    writeFileSync(join(dir, "lib.rs"), rust);
    const out = runSync([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `dep=${join(dir, "libdep.rmeta")}`, "-L", dir], dir, 60_000);
    return [out.code, out.stderr.includes("not in an `extern` block")];
  };
  // Called, and given as a value.
  expect(compile("pub fn f() -> f64 {\n    dep::max(1.0, 2.0)\n}\n")).toEqual([1, true]);
  expect(compile("pub fn f() -> Vec<f64> {\n    [1.0, 2.0].map(|x| x * 2.0).into_iter().zip([3.0, 1.0]).map(|(a, b)| (dep::max as fn(f64, f64) -> f64)(a, b)).collect()\n}\n")).toEqual([1, true]);
});
