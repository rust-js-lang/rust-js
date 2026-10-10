import { beforeAll, expect, test } from "bun:test";
import { existsSync, readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { buildCompiler, buildSerde, buildWebapi, compiler, fixture, root, target } from "./support";

beforeAll(buildCompiler, 600_000);

for (const [name, source, message, crate] of [
  ["type error", 'pub fn f() -> i32 { "wrong" }', "mismatched types"],
  ["borrow error", 'pub fn f() -> i32 { let mut x = 1; let r = &x; x = 2; *r }', "borrowed"],
  ["unsupported type", 'pub fn f(x: *const u8) -> *const u8 { x }', "does not support"],
  // ADR 0122: an `f32`'s bits aren't yet: a NaN's payload is JS's to keep or not.
  // ADR 0123: what `..` binds is a copy, as `&v[a..b]` is (ADR 0063), so one to write through can't be.
  ["a &mut rest of a slice pattern", "pub fn f(v: &mut [u32]) { if let [_, rest @ ..] = v { rest[0] = 1; } }", "a `&mut` to part of a slice"],
  // ADR 0124: a choice of places, `p[0] === 0 ? p[1] : p[0]`, can't be written through.
  ["a ref mut bound at two places of a | pattern", "pub fn f(p: &mut (i32, i32)) { if let (0, x) | (x, 0) = p { *x += 1; } }", "a `ref mut` bound at another place in each alternative of a `|` pattern"],
  // A guard is tried with each alternative in turn, which a choice of places, the first's, isn't:
  // `(3, 42)` takes this arm in Rust, with `a` 42 (rustc's issue-70413).
  ["a guard of a | pattern binding at two places", "pub fn f(p: (i32, i32)) -> i32 { match p { (a, _) | (_, a) if a > 10 => a, _ => 0 } }", "a guard of a `|` pattern binding a name at another place in each alternative"],
  ["camelCase fields that collide", '#![allow(non_snake_case)]\n#[rust_js::camel_case]\nconst _: () = ();\npub struct P { pub first_name: u32, pub firstName: u32 }\npub fn f(p: &P) -> u32 { p.first_name + p.firstName }', "both `firstName` in JS"],
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
  ["a derive of an open union", '#[rust_js::tag = "kind"]\n#[derive(Clone)]\npub enum E { A { n: u32 }, #[rust_js::otherwise] O(&\'static js::JsObject) }\npub fn f(e: &E) -> E { e.clone() }', "deriving `Clone` of a discriminated union with an `otherwise`", "js"],
  ["#[thread_local] static", "#![feature(thread_local)]\n#[thread_local] static N: std::cell::Cell<u32> = std::cell::Cell::new(0);\npub fn f() -> u32 { N.get() }", "does not support `#[thread_local]` statics"],
  ["static holding a reference to another", "static A: u32 = 1;\nstatic B: &u32 = &A;\npub fn f() -> u32 { *B }", "does not support statics of type `&'static u32`"],
  // A `dyn Error`'s dictionary has its `source`, and none of what else std provides (ADR 0141).
  ["a deprecated method of a dyn Error", '#![allow(deprecated)]\npub fn f(e: &dyn std::error::Error) -> String { e.description().to_string() }', "`description` of a"],
  // A placeholder's options are given to what shows the value (ADR 0058), but a
  // `&dyn Debug` is the string it shows already, and serde_json's `fmt`s are rust-js's.
  ["a width for a dyn Debug", 'pub fn f(v: u8) -> String { let d: &dyn std::fmt::Debug = &v; format!("{:5?}", d) }', "options for a"],
  // `ptr::eq` of a Rust value: an unchanged copy is one JS object, and a
  // number is its value, so a place's address isn't JS's (ADR 0285).
  ["ptr::eq of a Rust struct", "#[derive(Clone, Copy)]\npub struct P { pub x: u32 }\npub fn f(a: &P, b: &P) -> bool { std::ptr::eq(a, b) }", "`std::ptr::eq` of a `P`"],
  ["ptr::eq of a number", "pub fn f(a: &u32, b: &u32) -> bool { std::ptr::eq(a, b) }", "`std::ptr::eq` of a `u32`"],
  // `replacen` of the first is JS's `replace`; of another count, none (ADR 0034).
  ["replacen of more than one", 'pub fn f(s: &str) -> String { s.replacen("a", "b", 2) }', "`replacen` of a count other than 1"],
  // A JS error is the string JS shows of it, `String(error)` (ADR 0271).
  ["a width for a JsError", 'pub fn f(e: &js::JsError) -> String { format!("{:>9?}", e) }', "options for a", "js"],
  ["a width for a serde_json Value", 'pub fn f(v: &serde_json::Value) -> String { format!("{:>9}", v) }', "options for a", "serde"],
  // A static's initializer is code where rustc's value can't say it (ADR 0096),
  // but not one reading another static, nor of a value that can change in place.
  ["a static of a function read from another static", 'pub fn f() {}\nstatic A: fn() = f;\npub static B: &fn() = &A;\npub fn g() { B() }', "statics of type"],
  ["a constant of a Vec of functions", 'pub fn f() {}\npub const C: Vec<fn()> = Vec::new();\npub fn g() -> usize { C.len() }', "constants of type"],
  // ADR 0121: a key found by its value is one a derived `Eq` compares.
  ["map keyed by a struct of its own equality", '#[derive(Hash)] pub struct P { pub x: u32 }\nimpl PartialEq for P { fn eq(&self, o: &P) -> bool { self.x % 10 == o.x % 10 } }\nimpl Eq for P {}\npub fn f() -> usize { let m: std::collections::HashMap<P, u32> = std::collections::HashMap::new(); m.len() }', "does not support values of type `P`"],
  ["map keyed by a struct with a field of its own equality", '#[derive(Hash)] pub struct Q(pub u32);\nimpl PartialEq for Q { fn eq(&self, o: &Q) -> bool { self.0 % 10 == o.0 % 10 } }\nimpl Eq for Q {}\n#[derive(PartialEq, Eq, Hash)] pub struct P { pub q: Q }\npub fn f() -> usize { let m: std::collections::HashSet<P> = std::collections::HashSet::new(); m.len() }', "does not support values of type `P`"],
  ["B-tree keyed by a struct", '#[derive(PartialEq, Eq, PartialOrd, Ord)] pub struct P { pub x: u32 }\npub fn f() -> usize { let m: std::collections::BTreeMap<P, u32> = std::collections::BTreeMap::new(); m.len() }', "does not support values of type `P`"],
  ["== on maps", 'pub fn f(a: &std::collections::HashMap<u32, u32>, b: &std::collections::HashMap<u32, u32>) -> bool { a == b }', "`==` on"],
  ["parse to a type without FromStr support", 'pub fn f(s: &str) -> bool { s.parse::<std::net::IpAddr>().is_ok() }', "does not support"],
  ["next() of a RangeInclusive kept as a value", 'pub fn f(r: &mut std::ops::RangeInclusive<u32>) -> Option<u32> { r.next() }', "kept as a value"],
  // A string's byte offsets are counted (ADR 0138), but not yet of a closure's matches.
  // `a + b` of a `T: Add` is its dictionary's (ADR 0108); `a += b` isn't yet.
  ["an assigning operator in generic code", 'pub fn f<T: std::ops::AddAssign>(a: &mut T, b: T) { *a += b; }', "does not support"],
  ["an Rc's raw pointer", 'pub fn f(r: &std::rc::Rc<u32>) -> bool { std::rc::Rc::as_ptr(r).is_null() }', "does not support"],
  ["a heap of options", 'pub fn f() -> bool { let mut h = std::collections::BinaryHeap::new(); h.push(Some(1u32)); h.pop().is_some() }', "a heap of"],
  ["a pointer of the crate's own to a dyn", "#![feature(derive_coerce_pointee)]\nuse std::ops::Deref;\n#[derive(std::marker::CoercePointee)] #[repr(transparent)] pub struct Ptr<'a, #[pointee] T: ?Sized> { ptr: &'a T }\nimpl<T: ?Sized> Deref for Ptr<'_, T> { type Target = T; fn deref(&self) -> &T { self.ptr } }\npub trait Get { fn get(&self) -> u32; }\npub struct V(u32);\nimpl Get for V { fn get(&self) -> u32 { self.0 } }\npub fn f() -> u32 { let v = V(10); let p: Ptr<dyn Get> = Ptr { ptr: &v }; p.get() }", "does not support unsizing a `Ptr<'_, dyn Get>`"],
  // A let-chain's condition's temporaries end at its `&&`, its `let`s' with the `if`: not yet (ADR 0098).
  ["a temporary with a destructor", 'pub struct D;\nimpl Drop for D { fn drop(&mut self) {} }\nimpl D { pub fn n(&self) -> u32 { 1 } }\npub fn f(b: bool) -> u32 { if D.n() == 1 && let true = b { 1 } else { 0 } }', "does not support a temporary with a destructor"],
  // A `dyn` of std's traits has no dictionary to carry a drop (ADR 0098).
  ["a Box<dyn Send> of a value with a destructor", 'pub struct D;\nimpl Drop for D { fn drop(&mut self) {} }\npub fn f() { let _: Box<dyn Send> = Box::new(D); }', "a `dyn` of a value with a destructor"],
  ["an Rc<dyn> of a value with a destructor", 'pub struct D;\nimpl Drop for D { fn drop(&mut self) {} }\npub fn f() { let _: std::rc::Rc<dyn Send> = std::rc::Rc::new(D); }', "a `dyn` of a value with a destructor"],
  // A std call given an iterator whose destructors rust-js can't follow would drop what it skips silently.
  // A generic iterator folded or looped over is one its callers give no destructor of (ADR 0190).
  ["a destructor given where a generic fold takes none", 'pub struct Counter(pub u32);\nimpl Iterator for Counter { type Item = u32; fn next(&mut self) -> Option<u32> { if self.0 < 3 { self.0 += 1; Some(self.0) } else { None } } }\nimpl Drop for Counter { fn drop(&mut self) {} }\nfn total<I: Iterator<Item = u32>>(items: I) -> u32 { items.fold(0, |a, b| a + b) }\npub fn f() -> u32 { total(Counter(0)) }', "giving `total`'s `I` a type with a destructor, where it takes none"],
  ["a destructor given through a generic function", 'pub struct Counter(pub u32);\nimpl Iterator for Counter { type Item = u32; fn next(&mut self) -> Option<u32> { if self.0 < 3 { self.0 += 1; Some(self.0) } else { None } } }\nimpl Drop for Counter { fn drop(&mut self) {} }\nfn total<I: Iterator<Item = u32>>(items: I) -> u32 { items.fold(0, |a, b| a + b) }\nfn relay<J: Iterator<Item = u32>>(j: J) -> u32 { total(j) }\npub fn f() -> u32 { relay(Counter(0)) }', "giving `relay`'s `J` a type with a destructor, where it takes none"],
  ["a destructor given where an impl's method takes none, through its dictionary", 'pub trait Empty { fn empty(&mut self); }\npub struct Wrap<T>(pub Vec<T>);\nimpl<T> Empty for Wrap<T> { fn empty(&mut self) { self.0.clear(); } }\npub struct Loud;\nimpl Drop for Loud { fn drop(&mut self) {} }\nfn g<E: Empty>(e: &mut E) { e.empty(); }\npub fn f() { let mut w = Wrap(vec![Loud]); g(&mut w); }', "giving `<Wrap<T> as Empty>`'s `T` a type with a destructor, where it takes none"],
  // Which alternative matched, where both may, is the value's to know (ADR 0191).
  ["an or-pattern whose alternatives move parts no variant tells apart", 'pub struct D;\nimpl Drop for D { fn drop(&mut self) {} }\npub fn f(p: (Option<D>, Option<D>)) -> Option<D> { match p { (Some(t), _) | (_, Some(t)) => Some(t), _ => None } }', "moving part of a value with a destructor"],
  ["last of owned items", 'pub struct D;\nimpl Drop for D { fn drop(&mut self) {} }\npub fn f(v: Vec<D>) -> Option<D> { v.into_iter().last() }', "`std::iter::Iterator::last` of a value with a destructor"],
  // `ok_or_else` keeps its value, but its function, unrun for a `Some`, is dropped.
  ["ok_or_else whose function holds a value with a destructor", 'pub struct D(pub u32);\nimpl Drop for D { fn drop(&mut self) {} }\npub fn f(o: Option<D>, held: D) -> Result<D, u32> { o.ok_or_else(move || { let h = held; h.0 }) }', "`std::option::Option::<T>::ok_or_else` of a value with a destructor"],
  // A size of a type parameter is its caller's (ADR 0145), but `size_of_val` of an unsized one is the value's.
  ["size_of_val of an unsized type parameter", 'pub fn f<T: ?Sized>(x: &T) -> usize { std::mem::size_of_val(x) }\npub fn g() -> usize { f("ab") }', "`size_of` of a type parameter"],
  // `ToString` is a dictionary of a generic `T`'s, but a `dyn ToString` has no pair to call through.
  ["to_string of a dyn ToString", 'pub fn f(x: &dyn ToString) -> String { x.to_string() }', "dyn std::string::ToString"],
  // `collect()` into a `Result` or an `Option` is of an array (ADR 0036).
  ["collecting into a Result of a String", 'pub fn f(v: Vec<Result<char, ()>>) -> Result<String, ()> { v.into_iter().collect() }', "collecting into a"],
  // `len()` takes no item, so it runs none of a chain's closures (ADR 0036).
  ["len of a chain whose closure prints", 'pub fn f(v: &[u32]) -> usize { v.iter().map(|x| { println!("{x}"); x }).len() }', "`len()` of an iterator whose closures do what can be seen"],
  ["len of a Peekable", 'pub fn f(v: &[u32]) -> usize { let mut p = v.iter().peekable(); p.peek(); p.len() }', "`len()` of a `Peekable`"],
  ["an Rc of a value with a destructor", 'pub struct D;\nimpl Drop for D { fn drop(&mut self) {} }\npub fn f() { let r = std::rc::Rc::new(D); drop(r); }', "a std type holding a value with a destructor, `std::rc::Rc<D>`"],
  ["a writer of the crate's that fails", "use std::fmt::{self, Write};\npub struct Full(pub usize);\nimpl Write for Full { fn write_str(&mut self, s: &str) -> fmt::Result { if s.len() > self.0 { return Err(fmt::Error); } self.0 -= s.len(); Ok(()) } }", "a `fmt::Error` of a writer of the crate's"],
  ["a map's key looked up through the crate's Borrow", "use std::borrow::Borrow;\n#[derive(PartialEq, Eq, Hash)]\npub struct Name(pub String);\nimpl Borrow<str> for Name { fn borrow(&self) -> &str { &self.0 } }\npub fn f(m: &std::collections::HashMap<Name, i32>) -> Option<&i32> { m.get(\"a\") }", "`std::collections::HashMap::<K, V, S, A>::get` of a `Name` as what its own `Borrow` gives"],
  ["items joined through the crate's Borrow", "use std::borrow::Borrow;\n#[derive(PartialEq, Eq, Hash)]\npub struct Name(pub String);\nimpl Borrow<str> for Name { fn borrow(&self) -> &str { &self.0 } }\npub fn f(v: &[Name]) -> String { v.join(\",\") }", "`std::slice::<impl [T]>::join` of a `Name` as what its own `Borrow` gives"],
  ["size_hint of an iterator whose hint isn't exact", "pub fn f(s: &str) -> usize { s.chars().size_hint().0 }", "calling `std::iter::Iterator::size_hint`"],
  ["comparing paths, which compares their components", "use std::path::Path;\npub fn f(a: &Path, b: &Path) -> bool { a == b }", "`==` on `std::path::Path`"],
  ["a user impl of a std trait", 'pub struct C;\nimpl std::hash::Hasher for C { fn finish(&self) -> u64 { 0 } fn write(&mut self, _: &[u8]) {} }', "user implementations of `std::hash::Hasher`"],
  ["comparing another crate's struct", 'pub fn f(a: std::time::Instant, b: std::time::Instant) -> bool { a < b }', "does not support"],
  ["next() of an iterator in a field", 'pub struct L<\'a> { c: std::str::Chars<\'a> }\npub fn f(l: &mut L) -> Option<char> { l.c.next() }', "make it a `Peekable`"],
  ["peekable of a lazy iterator", 'pub struct C(u32);\nimpl Iterator for C { type Item = u32; fn next(&mut self) -> Option<u32> { self.0 += 1; Some(self.0) } }\npub fn f() -> Option<u32> { let mut p = C(0).peekable(); p.peek().copied() }', "`peekable` of a lazy iterator"],
  ["a new closure assigned through a &mut to one", 'pub fn replace<F: FnMut()>(f: &mut F, g: F) { *f = g; }', "assigning a whole value through a `&mut`"],
  ["a collection of the crate's extending a Vec", "struct Bag { items: Vec<i32> }\nimpl IntoIterator for Bag { type Item = i32; type IntoIter = std::vec::IntoIter<i32>; fn into_iter(self) -> Self::IntoIter { self.items.into_iter() } }\npub fn f(v: &mut Vec<i32>, b: Bag) { v.extend(b) }", "a `Bag`, whose `IntoIterator` is the crate's"],
  ["a collection of the crate's given to generic code", "struct Bag { items: Vec<i32> }\nimpl IntoIterator for Bag { type Item = i32; type IntoIter = std::vec::IntoIter<i32>; fn into_iter(self) -> Self::IntoIter { self.items.into_iter() } }\nfn count<I: IntoIterator>(i: I) -> usize { i.into_iter().count() }\npub fn f(b: Bag) -> usize { count(b) }", "a `Bag`, whose `IntoIterator` is the crate's"],
  ["a std &mut to a number bound and kept", "use std::collections::HashMap;\npub fn f(m: &mut HashMap<u32, i32>) { let k; match m.get_mut(&1) { Some(v) => k = v, None => return } *k += 1; }", "a `&mut i32` from a std call used as a value"],
  ["a generic &mut T to an object in a struct", "pub struct C { pub n: i32 }\npub struct H<'a, T> { pub r: &'a mut T }\nfn get<'a, T>(h: H<'a, T>) -> &'a mut T { h.r }\npub fn f(c: &mut C) { get(H { r: c }).n += 1; }", "a `&mut C` inside a generic function's parameters or result"],
  ["generic &mut Ts to objects returned in a Vec", "pub struct C { pub n: i32 }\nfn to_vec<T>(a: &mut T) -> Vec<&mut T> { vec![a] }\npub fn f(c: &mut C) -> usize { to_vec(c).len() }", "a `&mut C` inside a generic function's parameters or result"],
  ["a generic &mut T to an object in an Option", "pub struct C { pub n: i32 }\nfn set<T>(o: Option<&mut T>, v: T) { if let Some(r) = o { *r = v; } }\npub fn f(c: &mut C) { set(Some(c), C { n: 7 }); }", "a `&mut C` inside a generic function's parameters or result"],
  ["a generic &mut T to an object given to a closure", "pub struct C { pub n: i32 }\nfn apply<T, F: FnMut(&mut T)>(t: &mut T, mut f: F) { f(t); }\npub fn f() -> i32 { let mut c = C { n: 1 }; apply(&mut c, |x| x.n += 1); c.n }", "a `&mut C` inside a generic function's parameters or result"],
  ["a default method's &mut self of an object Self", "pub trait Ch: Sized { fn change(mut self) -> Self { self.set(5); self } fn set(&mut self, a: i32); }\npub struct X { pub a: i32 }\nimpl Ch for X { fn set(&mut self, a: i32) { self.a = a; } }\npub fn f() -> i32 { X { a: 1 }.change().a }", "`&mut` to a `Self`"],
  // A generic associated type of lifetimes is an associated type (ADR 0146); one of a type with a bound isn't.
  ["a generic associated type of a type with a bound", "pub trait Wrap { type Of<T>: Clone; fn wrap<T: Clone>(t: T) -> Self::Of<T>; }", "generic associated types with bounds"],
  ["a std trait's associated type's value, where a type has a destructor", "pub struct Guard;\nimpl Drop for Guard { fn drop(&mut self) {} }\npub fn first<I: Iterator>(mut i: I) -> usize { let _x = i.next(); 0 }", "a value of an associated type, where a type may have a destructor"],
  ["an extern declaration of the crate's own no_mangle function", "pub mod export { #[unsafe(no_mangle)] pub extern \"C\" fn twice(t: i32) -> i32 { t * 2 } }\nunsafe extern \"C\" { fn twice(t: i32) -> i32; }\npub fn f() -> i32 { unsafe { twice(3) } }", "an `extern` declaration of this crate's own `#[no_mangle]` function"],
  ["a generic constant", "#![feature(generic_const_items)]\n#![allow(incomplete_features)]\npub trait Sizes { const SIZE<T>: usize; }", "generic constants"],
  ["a type constant, refused not crashed", "#![feature(min_generic_const_args)]\n#![allow(incomplete_features)]\npub trait Foo { type const N: usize; }\npub struct Bar;\nimpl Foo for Bar { type const N: usize = 3; }", "type constants"],
  ["a generic impl's constant of its parameter's size", "pub trait Size { const SIZE: usize; }\npub struct W<T>(pub T);\nimpl<T> Size for W<T> { const SIZE: usize = std::mem::size_of::<T>(); }\nfn size<S: Size>() -> usize { S::SIZE }\npub fn f() -> usize { size::<W<u8>>() }", "`size_of` of a type parameter"],
  ["a generic const expression", "#![feature(generic_const_exprs)]\n#![allow(incomplete_features)]\nfn count<const N: usize>() -> usize { N }\nfn one_more<const N: usize>() -> usize where [(); N + 1]: { count::<{ N + 1 }>() }\npub fn f() -> usize { one_more::<2>() }", "this const argument"],
  ["an externally implementable item", "#![feature(extern_item_impls)]\n#[eii(hello)]\nstatic HELLO: u64;\n#[hello]\nstatic HELLO_IMPL: u64 = 5;\npub fn f() -> u64 { HELLO }", "externally implementable items"],
  // A generic iterator lent as `&mut` is the lender's, which must know where it is (ADR 0071): a field's doesn't.
  // A trait's method is called through a dictionary, whose `&mut self` is a handle (ADR 0099).
  ["a &mut to a generic iterator in a trait's method", 'pub trait Step { fn step(&mut self) -> Option<u32>; }\nimpl<I: Iterator<Item = u32>> Step for I { fn step(&mut self) -> Option<u32> { self.next() } }', "of a trait's method"],
  ["lending an iterator kept in a field", 'pub struct P { pub v: std::vec::IntoIter<u32> }\nfn first<I: Iterator<Item = u32>>(it: &mut I) -> Option<u32> { it.next() }\npub fn f(p: &mut P) -> Option<u32> { first(&mut p.v) }', "lending an iterator that isn't a local"],
  ["{:.2e}", 'pub fn f(x: f64) -> String { format!("{:.2e}", x) }', "`{:.2e}` and the like"],
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
  ["a variadic binding of no slice", 'unsafe extern "Rust" {\n    #[link_name = "Math.max"]\n    #[rust_js::variadic]\n    safe fn max(a: f64, b: f64) -> f64;\n}\npub fn f() -> f64 { max(1.0, 2.0) }', "isn't a slice"],
  ["an untagged enum's otherwise variant before another", '#[rust_js::untagged] pub enum E { #[rust_js::otherwise] A(String), B(u32) }\npub fn f(e: E) -> u32 { match e { E::A(_) => 0, E::B(n) => n } }', "is its last"],
  // One of no value is its name's string (ADR 0214); one of two isn't one.
  ["an untagged enum's variant of two values", '#[rust_js::untagged] pub enum E { A(u32), B(u32, u32) }\npub fn f(e: E) -> u32 { match e { E::A(n) => n, E::B(a, _) => a } }', "holds one value"],
  ["an untagged enum's variant of an Option", '#[rust_js::untagged] pub enum E { A(Option<u32>), B(String) }\npub fn f(e: &E) -> u32 { match e { E::A(_) => 1, E::B(_) => 2 } }', "from another variant's value"],
  ["a From into an untagged enum that does more", '#[rust_js::untagged] pub enum E { A(u32), B(String) }\nimpl From<u32> for E { fn from(n: u32) -> Self { E::A(n + 1) } }\npub fn f(n: u32) -> E { n.into() }', "is its variant of its argument"],
  ["serde of an untagged enum", '#[rust_js::untagged]\n#[derive(serde::Serialize)] pub enum E { A(u32), B(String) }\npub fn f(e: &E) -> String { serde_json::to_string(e).unwrap() }', "serde of the untagged enum", "serde"],
  ["#[serde(with)]", 'mod m { pub fn serialize<S: serde::Serializer>(v: &u32, s: S) -> Result<S::Ok, S::Error> { s.serialize_u32(*v) } }\n#[derive(serde::Serialize)] pub struct W { #[serde(with = "m")] pub x: u32 }\npub fn f(w: &W) -> String { serde_json::to_string(w).unwrap() }', "`#[serde(with)]`", "serde"],
  ["#[serde(serialize_with)]", 'fn s<S: serde::Serializer>(v: &u32, s: S) -> Result<S::Ok, S::Error> { s.serialize_u32(*v) }\n#[derive(serde::Serialize)] pub struct W { #[serde(serialize_with = "s")] pub x: u32 }\npub fn f(w: &W) -> String { serde_json::to_string(w).unwrap() }', "`#[serde(serialize_with)]`", "serde"],
  ["#[serde(deserialize_with)]", 'fn d<\'de, D: serde::Deserializer<\'de>>(d: D) -> Result<u32, D::Error> { <u32 as serde::Deserialize>::deserialize(d) }\n#[derive(serde::Deserialize)] pub struct W { #[serde(deserialize_with = "d")] pub x: u32 }\npub fn f(s: &str) -> bool { serde_json::from_str::<W>(s).is_ok() }', "`#[serde(deserialize_with)]`", "serde"],
  ["writing a u128", 'pub fn f(n: u128) -> String { serde_json::to_string(&n).unwrap() }', "does not support", "serde"],
  ["reading a u128", 'pub fn f(s: &str) -> bool { serde_json::from_str::<u128>(s).is_ok() }', "does not support", "serde"],
  ["reading a BinaryHeap", 'pub fn f(s: &str) -> bool { serde_json::from_str::<std::collections::BinaryHeap<u32>>(s).is_ok() }', "deserializing", "serde"],
  ["unsupported Value method", 'pub fn f(v: &serde_json::Value) -> bool { v.pointer("/name").is_some() }', "`Value::pointer`", "serde"],
  ["assigning to a Value's key", 'pub fn f() -> String { let mut v = serde_json::json!({}); v["k"] = serde_json::json!(1); v.to_string() }', "assigning to this place", "serde"],
  // A JSON object's keys are strings: serde_json refuses a struct's (ADR 0121).
  ["a map keyed by a struct in JSON", '#[derive(PartialEq, Eq, Hash, serde::Serialize)] pub struct P { pub x: u32 }\npub fn f(m: &std::collections::HashMap<P, u32>) -> String { serde_json::to_string(m).unwrap() }', "a map key of", "serde"],
] as [string, string, string, string?][]) {
  test(`${name} reports a source location and preserves existing output`, () => {
    const dir = fixture("diagnostic");
    const input = join(dir, "lib.rs"), output = join(dir, "lib.js"), manifest = join(dir, "manifest.json");
    writeFileSync(input, source);
    const crates = crate === "js" ? (buildWebapi(), ["--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]) : crate ? buildSerde() : [];
    const args = [compiler, input, "-o", output, "--manifest", manifest, ...(crates.length ? ["--", ...crates] : [])];
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
