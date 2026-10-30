# Known limitations

What rust-js doesn't do yet. It never writes JS that runs differently from
Rust by accident: what it can't compile as Rust runs, it refuses, with an
error that names it, `rust-js does not support .. yet`
([ADR 0006](../decisions/0006-errors-and-unsupported-features.md)). This page
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
  [`test/rustc-known-failures.txt`](../../test/rustc-known-failures.txt), with
  its first error ([ADR 0089](../decisions/0089-rustc-tests.md)). Most need
  addressable memory, below.
- **Crates from crates.io**: how they compile, and what's refused, in
  [Crates from crates.io](libraries/crates-io.md); which compile and run,
  and what stops the rest, in [the crate corpus](../crate-corpus.md).
- **React and JSX**: [their own pages](README.md#react).

## Memory and unsafe code

JS has no addressable memory: a value is an object or a number, not bytes
at an address. rust-js gives raw pointers, `transmute` and the like the
meaning Rust's have where it can model them: a raw pointer to a local or
into an array, its `add` and `offset_from`, a `transmute` between types of
numbers alone (`u16` to `[u8; 2]`, a struct of two `u32`s to a `u64`),
`slice::from_raw_parts` of an array's pointer. A `#[repr(transparent)]`
struct is its field, so a `transmute` between them, by value or through
either reference, is the value itself
([ADR 0584](../decisions/0584-a-transparent-struct-is-its-field.md)). What
reads memory by where it is, it refuses:

- A raw address of an immutable `static`: `static N: u32 = 7; let p = &raw const N; println!("{}", unsafe { *p });`
- A `&mut` to a `static mut`, which Rust 2024 denies too (`static_mut_refs`); a pointer to it, `&mut *addr_of_mut!(N)`, compiles: `static mut N: u32 = 0; #[allow(static_mut_refs)] let r = unsafe { &mut N }; *r += 1;`
- A `transmute` of an enum, whose bytes a niche or a tag lays out: `#[repr(u8)] enum Size { One = 1 } let n: u8 = unsafe { std::mem::transmute::<Option<Size>, u8>(None) }; let _ = (n, Size::One);`
- A `static` of a reference to another `static`'s array or item; one to a literal or a `const` compiles: `static NAMES: [&str; 2] = ["x", "y"]; static REF: &[&str] = &NAMES; println!("{}", REF.join(""));`
- `==` or `{:?}` of raw pointers in generic code to a type that holds a type parameter, `NonNull<Node<T>>`, whose size rust-js can't give: `struct Node<T>(T); fn same<T>(a: *const Node<T>, b: *const Node<T>) -> bool { a == b } let n = Node(1); println!("{}", same(&n, &n));`

And refused when it runs, as a panic that says so, since which it is
depends on what the program does:

- A `TypeId`'s hash shown, which rust-js doesn't keep. Only an assertion
  that fails shows one: `assert_eq!` of two compiles. A pointer's address
  is shown, a number of rust-js's own ([ADR 0562](../decisions/0562-a-pointer-is-a-place-and-an-address.md)).
- Reading through a pointer with no place, an integer made a pointer, or
  the address of a value JS holds as itself, a number reached through a
  `&`.
- A union's field read while it holds one whose bytes JS has none of, a
  `&str`'s or a `String`'s; plain data's bytes read as another type's
  ([ADR 0562](../decisions/0562-a-pointer-is-a-place-and-an-address.md)).
- A pointer to what has no bytes, a `String`, read as bytes; a pointer
  stepped outside an array or into what JS holds as a copy.

## Types

- A `dyn Debug` of a value with a destructor, which the string it shows can't keep: `struct D; impl Drop for D { fn drop(&mut self) {} } impl std::fmt::Debug for D { fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result { f.write_str("D") } } let b: Box<dyn std::fmt::Debug> = Box::new(D); println!("{b:?}");`
- A struct whose last field is a `dyn Debug`, the string it shows: `struct W<T: ?Sized> { n: u32, v: T } let w: Box<W<dyn std::fmt::Debug>> = Box::new(W { n: 1, v: 5u8 }); println!("{:?}", &w.v);`
- A struct whose last field is a `dyn`, behind a `Mutex`, which holds its value apart from the struct's `impl` (#375): `trait N { fn n(&self) -> u32; } struct A; impl N for A { fn n(&self) -> u32 { 1 } } struct W<T: ?Sized> { k: u32, v: T } let m = std::sync::Mutex::new(W { k: 0, v: A }); let d: &std::sync::Mutex<W<dyn N>> = &m; println!("{}", d.lock().unwrap().v.n());`
- A `RefCell` of a `dyn` of the crate's trait given to generic code as a cell of a type parameter, which its pair isn't: `trait N { fn n(&self) -> u32; } struct A; impl N for A { fn n(&self) -> u32 { 1 } } fn get<T: ?Sized + N>(c: &std::cell::RefCell<T>) -> u32 { c.borrow().n() } let c: &std::cell::RefCell<dyn N> = &std::cell::RefCell::new(A); println!("{}", get(c));`
- The `TypeId` of a type made of a type parameter, a closure's in generic code: `fn make<T: 'static>(t: T) -> std::any::TypeId { fn id<C: 'static>(_: &C) -> std::any::TypeId { std::any::TypeId::of::<C>() } let c = move || drop(t); id(&c) } println!("{:?}", make(1u8) == make(2u8));`
- `size_of_val` of a value without one size, a `dyn`'s: `let b: Box<dyn std::fmt::Debug> = Box::new(5u64); println!("{}", std::mem::size_of_val(&*b));`
- A map or set in generic code keyed by a type made of a type parameter, `Key<T>`, `(T, i32)` or `Option<T>`, whose class rust-js can't make of what its caller gives; a `T` itself is its caller's ([ADR 0466](../decisions/0466-a-generic-maps-class-is-its-callers.md), amended): `fn count<T: std::hash::Hash + Eq>(xs: Vec<T>) -> usize { xs.into_iter().map(|a| (a, 0)).collect::<std::collections::HashSet<_>>().len() } println!("{}", count(vec![1, 1]));`
- A map key whose `PartialEq` is the crate's own: `use std::collections::HashMap; #[derive(Hash)] struct K(u32); impl PartialEq for K { fn eq(&self, o: &K) -> bool { self.0 % 10 == o.0 % 10 } } impl Eq for K {} let mut m = HashMap::new(); m.insert(K(1), 1); println!("{}", m.contains_key(&K(11)));`
- A `&mut` to an object given for a type parameter inside what generic code takes or gives, an `Option`, which it has as a box and its caller as the object: `fn some<D>(d: Option<D>) -> Option<D> { d } let mut v = vec![1]; if let Some(r) = some(Some(&mut v)) { r.push(2); }`
- A `RefCell`'s guard mapped to part of what it holds, `Ref::map` or `RefMut::map`: `let c = std::cell::RefCell::new((1, 2)); let r = std::cell::Ref::map(c.borrow(), |p| &p.0); println!("{}", *r);`
- A `&mut` to an object given for a type parameter that's a closure's parameter: `fn apply<D, F: Fn(D) -> usize>(d: D, f: F) -> usize { f(d) } let mut v = vec![1]; println!("{}", apply(&mut v, |w: &mut Vec<i32>| w.len()));`
- `downcast_ref`, `is` or `downcast` of a `dyn Error`, whose `impl` keeps no type to compare, as a `dyn Any`'s `TypeId` does: `let e: Box<dyn std::error::Error> = "x".parse::<i32>().unwrap_err().into(); println!("{:?}", e.downcast_ref::<std::num::ParseIntError>());`
- std's `From` of an `Option` or a `Vec<u8>` given to generic code bound by it, `impl Into<Option<u32>>`'s: `fn limit(l: impl Into<Option<u32>>) -> Option<u32> { l.into() } println!("{:?}", limit(5));`
- `<` of a `PartialOrd` of another type, `A: PartialOrd<B>` in generic code or the crate's `PartialOrd<Mm> for Cm` through references: `fn less<A: PartialOrd<B>, B>(a: &A, b: &B) -> bool { a < b } println!("{}", less(&1, &2));`
- A `&mut` to an object of a type parameter's inside what generic code takes or gives, `Option<&mut T>` or a struct holding a `&'a mut T`: `struct P { x: i32 } struct Stack<T> { items: Vec<T> } impl<T> Stack<T> { fn top(&mut self) -> Option<&mut T> { self.items.last_mut() } } let mut s = Stack { items: vec![P { x: 1 }] }; if let Some(p) = s.top() { p.x += 1; }`
- A `&mut` to a struct in a `Vec` given straight to generic code, which a binding of it compiles: `fn set<T>(t: &mut T, u: T) { *t = u; } struct P { x: i32 } let mut v = vec![P { x: 1 }]; set(&mut v[0], P { x: 2 }); println!("{}", v[0].x);`
- A clone of an `Rc<RefCell<dyn Trait>>` put where its type is written, a `Vec`'s item or a field: `use std::{cell::RefCell, rc::Rc}; trait Obs { fn hear(&self) -> u32; } struct A; impl Obs for A { fn hear(&self) -> u32 { 1 } } let e: Rc<RefCell<dyn Obs>> = Rc::new(RefCell::new(A)); let v: Vec<Rc<RefCell<dyn Obs>>> = vec![e.clone()]; println!("{}", v[0].borrow().hear());`
- A `Weak<RefCell<dyn Trait>>` of a counted `Rc`: `use std::{cell::RefCell, rc::{Rc, Weak}}; trait Obs { fn hear(&self) -> u32; } struct A; impl Obs for A { fn hear(&self) -> u32 { 1 } } let a = Rc::new(RefCell::new(A)); let w = Rc::downgrade(&a) as Weak<RefCell<dyn Obs>>; println!("{:?}", w.upgrade().map(|o| o.borrow().hear()));`
- Copying a `Copy` closure that changes what it captured; its `.clone()` compiles: `let mut n = 0; let mut bump = move || { n += 1; n }; let mut copy = bump; println!("{} {}", bump(), copy());`
- A map or set key that's an `Rc` or an `Arc`, a string interner's `Rc<str>`: `let mut m = std::collections::HashMap::new(); let k: std::rc::Rc<str> = "a".into(); m.insert(k.clone(), 1); println!("{:?}", m.get(&k));`
- Changing an `Option` through the `&mut` a call gives, `get_or_insert` or `insert` of `last_mut().unwrap()` or an entry's `or_insert`, which a binding of the `&mut` compiles: `let mut v = vec![None]; *v.last_mut().unwrap().get_or_insert(1) += 1; println!("{v:?}");`
- `Option<&T>` from a `&Option<T>`, `(&Some(7)).into()`: `let o: Option<&i32> = (&Some(7)).into(); println!("{o:?}");`
- A `RefCell`'s guard cloned, `Ref::clone`: `let c = std::cell::RefCell::new(1); let a = c.borrow(); let b = std::cell::Ref::clone(&a); println!("{} {}", *a, *b);`
- A `RefCell`'s guard split in two, `RefMut::map_split`: `let c = std::cell::RefCell::new([1, 2]); let (a, b) = std::cell::RefMut::map_split(c.borrow_mut(), |s| s.split_at_mut(1)); println!("{} {}", a[0], b[0]);`
- Ordering `RefCell`s, `<` or `cmp`; `==` compiles: `let a = std::cell::RefCell::new(1); println!("{}", a < std::cell::RefCell::new(2));`
- A `OnceCell` made by `From`: `let c = std::cell::OnceCell::from(5); println!("{:?}", c.get());`
- `RefCell::into_inner` of a value with a destructor: `struct D(u32); impl Drop for D { fn drop(&mut self) {} } let c = std::cell::RefCell::new(D(1)); println!("{}", c.into_inner().0);`
- An `Rc` that's counted, `Rc::strong_count`, given on as an `Rc<dyn Trait>`: `trait Obs { fn n(&self) -> u32; } struct L; impl Obs for L { fn n(&self) -> u32 { 1 } } let l = std::rc::Rc::new(L); let v: Vec<std::rc::Rc<dyn Obs>> = vec![l.clone()]; println!("{} {}", v[0].n(), std::rc::Rc::strong_count(&l));`
- `TryFrom` of a type parameter, `T::try_from`: `fn narrow<T: TryFrom<i64>>(x: i64) -> Option<T> { T::try_from(x).ok() } println!("{:?}", narrow::<u8>(300));`
- An `Into` of the crate's, not a `From`, given where any `Into` goes: `struct B; impl Into<String> for B { fn into(self) -> String { "B".into() } } fn takes(s: impl Into<String>) -> String { s.into() } println!("{}", takes(B));`
- A map or a set keyed by `()`: `let mut m = std::collections::HashMap::new(); m.insert((), 1); println!("{:?}", m.get(&()));`
- A map key with a `PhantomData` field: `#[derive(Hash, PartialEq, Eq)] struct U; #[derive(Hash, PartialEq, Eq)] struct Id<T> { raw: u32, t: std::marker::PhantomData<T> } let mut s = std::collections::HashSet::new(); s.insert(Id::<U> { raw: 1, t: std::marker::PhantomData }); println!("{}", s.len());`
- A map key of an associated type: `trait E { type Id: std::hash::Hash + Eq; fn id(&self) -> Self::Id; } struct U; impl E for U { type Id = u32; fn id(&self) -> u32 { 1 } } fn index<T: E>(xs: Vec<T>) -> usize { xs.into_iter().map(|x| (x.id(), x)).collect::<std::collections::HashMap<_, _>>().len() } println!("{}", index(vec![U]));`
- `==` of an associated type in a trait's default method: `trait R { type Id: PartialEq + Copy; fn ids(&self) -> Vec<Self::Id>; fn has(&self, id: Self::Id) -> bool { self.ids().iter().any(|i| *i == id) } } struct A; impl R for A { type Id = u32; fn ids(&self) -> Vec<u32> { vec![1] } } println!("{}", A.has(1));`
- `Rc::make_mut` of an `Rc` a `&mut self` holds: `struct C { cur: std::rc::Rc<Vec<i32>> } impl C { fn push(&mut self, x: i32) { std::rc::Rc::make_mut(&mut self.cur).push(x); } } let mut c = C { cur: std::rc::Rc::new(vec![1]) }; c.push(2); println!("{:?}", c.cur);`
- A `dyn Iterator` of a value with a destructor: `struct D; impl Drop for D { fn drop(&mut self) {} } let it: Box<dyn Iterator<Item = u32>> = Box::new(vec![D].into_iter().map(|_| 1)); println!("{}", it.count());`
- A trait's default method that makes a value of its associated type, `Self::Unit::default()`, where an impl leaves the method to the default: `trait Shape { type Unit: Default + std::fmt::Display; fn unit(&self) -> Self::Unit { Self::Unit::default() } } struct Tri; impl Shape for Tri { type Unit = u8; } println!("{}", Tri.unit());`
- A derived `Default` of a struct holding a map keyed by a type parameter, made in generic code, which isn't given its keys' class; a written `impl Default` is: `#[derive(Default)] struct Node<K> { children: std::collections::HashMap<K, u8> } fn make<K: Default + Eq + std::hash::Hash>() -> Node<K> { Node::default() } println!("{}", make::<u32>().children.len());`

## Modules

- A `pub use m::*;` of a module that holds a module or a class with JS, whose namespace or constructor it would re-export: `pub use` each of its names that has JS by its name ([ADR 0874](../decisions/0874-a-modules-pub-uses-are-re-exported-as-written.md), amended).
- A `pub use` of std's function or constant, in no JS module of rust-js's to be re-exported from: `pub use std::cmp::max; println!("{}", max(1, 2));`
- A `pub use` of another rust-js crate's module, whose namespace no file of this crate's is: `pub use` each of its items by its name.
- Another module's write of a module's variable, a thread-local of a value or a `static mut`, whole, of a field or an item, or a pointer taken to it, which JS lets only its own module set: write a `pub fn` in its module that sets it ([ADR 0096](../decisions/0096-statics.md), amended): `mod counter { pub static mut LINES: usize = 0; } unsafe { counter::LINES = 3; }`
- An exported `static mut` its module gives a pointer to on, `add(&raw mut TOTAL)`, which a JS module importing it would read as its box: read and write it by its name, or through a pointer only where it's taken ([ADR 0096](../decisions/0096-statics.md), amended): `mod total { pub static mut TOTAL: u32 = 0; fn add(t: *mut u32) { unsafe { *t += 1 } } pub fn bump() { add(&raw mut TOTAL) } } total::bump();`
- An exported thread-local whose module needs its cell, which a JS module importing it would read in place of what it holds: it hands on the cell or the `&mut` of what it holds, or borrows it in a `with`. Use what `with_borrow_mut` gives in its closure ([ADR 0270](../decisions/0270-module-variables.md), amended): `mod depth { use std::cell::RefCell; thread_local! { pub static DEPTH: RefCell<Option<u32>> = const { RefCell::new(None) }; } fn fill(d: &mut Option<u32>) { d.get_or_insert(3); } pub fn deepen() { DEPTH.with_borrow_mut(|d| fill(d)); } } depth::deepen(); println!("{:?}", depth::DEPTH.with_borrow(|d| *d));`
- A `pub` thread-local, made as its module loads, whose initializer runs or reads code of a module in an import cycle with its own, reads itself, or calls a `dyn Fn` or a fn pointer that may be any function, which may not be made yet then: move what it runs out of the cycle, or call what it runs by its name, and make one that reads itself of what it reads instead ([ADR 1006](../decisions/1006-a-pub-thread-local-is-made-as-its-module-loads.md)): `thread_local! { pub static N: u32 = twice(); } fn twice() -> u32 { N.with(|n| *n) * 2 } println!("{}", N.with(|n| *n));`
- Two exports of one module with one JS name, of which an importer reads one by that name: give one another with `#[rust_js::name]` ([ADR 0922](../decisions/0922-an-export-is-named-first.md)): `mod order { pub fn sort_index() -> u32 { 1 } #[rust_js::name = "sortIndex"] pub fn index_for_sorting() -> u32 { 2 } } println!("{}", order::sort_index() + order::index_for_sorting());`
- An impl's dictionary made where its module loads, which the crate's
  own code reads, in an import cycle with a module that runs code where it
  loads, a `thread_local!` made by a binding's call, or `js::on_load!`,
  that may reach the dictionary before it's made. Move the impl or that
  code out of the cycle
  ([ADR 0592](../decisions/0592-a-dictionary-is-its-modules-const.md), amended).

## Iterators and collections

- A clone of a lazy iterator generic code makes where its source isn't bound by `Clone`, but a `map()`, std's struct where it's held ([ADR 0871](../decisions/0871-rev-is-stds.md)): `fn evens<I: Iterator<Item = i32>>(it: I) -> std::iter::Filter<I, fn(&i32) -> bool> { it.filter(|x| x % 2 == 0) } let d = evens(1..9); let e = d.clone(); println!("{:?} {:?}", d.count(), e.count());`
- A clone of a `copied()` of an iterator dropped as it's closed: the crate's own with a destructor, or one generic code is given its drop of, `$closing(it, drop)` ([ADR 0525](../decisions/0525-a-cloned-map-is-made-again.md), amended): `#[derive(Clone)] struct Loud(u32); impl Iterator for Loud { type Item = &'static u32; fn next(&mut self) -> Option<&'static u32> { None } } impl Drop for Loud { fn drop(&mut self) { println!("drop"); } } let c = Loud(0).copied(); let d = c.clone(); println!("{}", d.count());`
- A clone of a `copied()` whose items something changes in place, a JS iterator copying each as it steps ([ADR 0525](../decisions/0525-a-cloned-map-is-made-again.md), amended): `#[derive(Clone, Copy, Debug)] struct P { x: u32 } let ps = [P { x: 1 }]; let c = ps.iter().copied(); let mut d = c.clone(); let mut p = d.next().unwrap(); p.x += 1; println!("{p:?}");`
- `eq()` of two `TrustedLen` iterators whose lengths rust-js can't count before stepping them, where stepping them runs code, a map's, as std's `eq` asks their lengths first and steps neither of unequal ones ([ADR 0998](../decisions/0998-eq-of-uncounted-trusted-len-iterators-is-refused.md)): `let v = vec![1u32, 2, 3]; let w = vec![1u32]; println!("{}", v.iter().map(|x| { println!("a"); *x }).eq(w.iter().map(|x| *x)));`
- Giving generic code two `TrustedLen` iterators it compares by `eq()`, where stepping them runs code, a map's: its JS compares them item by item, as it can't count what it's given, and std's asks their lengths first (#386, [ADR 1014](../decisions/1014-generic-eq-of-trusted-len-is-refused-at-its-caller.md)): `fn same<I: Iterator<Item = u32>>(a: I, b: I) -> bool { a.eq(b) } let v = vec![1u32, 2]; let f = |x: &u32| { println!("a"); *x }; println!("{}", same(v.iter().map(f), v[..1].iter().map(f)));`
- Giving generic code a zip it folds, collects or steps, of sides std stops at the shorter of, where a step of its first side runs code: its JS steps it as `next()` does, its first side once more ([ADR 1014](../decisions/1014-generic-eq-of-trusted-len-is-refused-at-its-caller.md)): `fn pairs<A: Iterator<Item = u32>, B: Iterator<Item = u32>>(a: A, b: B) -> Vec<(u32, u32)> { a.zip(b).collect() } let v = vec![1u32, 2, 3]; println!("{:?}", pairs(v.iter().map(|x| { println!("a"); *x }), v[..2].iter().copied()));`
- A collection of the crate's given where any `IntoIterator` goes, which wouldn't call its `into_iter`: `struct Cart(Vec<u32>); impl IntoIterator for Cart { type Item = u32; type IntoIter = std::vec::IntoIter<u32>; fn into_iter(self) -> Self::IntoIter { self.0.into_iter() } } let mut v = vec![1]; v.extend(Cart(vec![2])); println!("{v:?}");`
- Looking a key up by what the crate's own `Borrow` gives: `use std::borrow::Borrow; use std::collections::HashMap; #[derive(PartialEq, Eq, Hash)] struct K(String); impl Borrow<str> for K { fn borrow(&self) -> &str { &self.0 } } let mut m = HashMap::new(); m.insert(K("a".into()), 1); println!("{:?}", m.get("a"));`
- `next_back()` of a lazy `lines()`, `split_whitespace()`, `split_terminator()` or `rsplit()`, whose `.rev()` compiles: `let mut l = "a\nb".lines(); println!("{:?}", l.next_back());`
- `.rev()` of a `take()` or a `skip()` of items with the crate's destructor, which std drops from the back as a `Vec`'s tail: `struct L(u32); impl Drop for L { fn drop(&mut self) { println!("drop {}", self.0); } } let v = vec![L(1), L(2), L(3)]; let r: Vec<L> = v.into_iter().take(2).rev().collect(); println!("{}", r.len());`
- `.rev()` of a `chain()` of a `map()` of a slice's items and another slice's: `let v = [1, 2]; let w = [3]; let r: Vec<i32> = v.iter().map(|x| x * 10).chain(w.iter().copied()).rev().collect(); println!("{r:?}");`
- `VecDeque::remove` in generic code given an `Option` for its item, whose `None` the result's could look like: `fn take<T>(q: &mut std::collections::VecDeque<T>, i: usize) -> Option<T> { q.remove(i) } let mut q = std::collections::VecDeque::from(vec![None, Some(2)]); println!("{:?}", take(&mut q, 0));`
- `dedup_by` in generic code, whose closure takes a `&mut` of a type parameter's items: `fn dd<T, F: FnMut(&mut T, &mut T) -> bool>(v: &mut Vec<T>, f: F) { v.dedup_by(f) } let mut v = vec![1, 1, 2]; dd(&mut v, |a, b| a == b); println!("{v:?}");`
- `len()` of a zip: `let v = [1, 2, 3]; println!("{}", v.iter().zip(v.iter()).len());`
- `size_hint()` of an iterator but a slice's, a range's, a collection's own `into_iter`, and what keeps their length (`skip`, `take`, `map`, `rev`, `enumerate`, `chain` and the like): a `filter`'s, a `str`'s `chars`, a map's `iter`: `let v = [1, 2, 3]; println!("{:?}", v.iter().filter(|x| **x > 1).size_hint());`
- A `&mut` from `flatten()` of an `iter_mut()`: `let mut g = vec![vec![1u64], vec![2]]; for x in g.iter_mut().flatten() { *x += 1; } println!("{g:?}");`
- `sum()` or `product()` of `Wrapping`s: `let s: std::num::Wrapping<u8> = [std::num::Wrapping(200u8), std::num::Wrapping(100)].into_iter().sum(); println!("{s}");`
- `slice::from_mut`; `slice::from_ref` compiles: `let mut x = 5; std::slice::from_mut(&mut x)[0] += 1; println!("{x}");`
- `extend()` of a pair of collections: `let mut p: (Vec<i32>, Vec<char>) = (vec![], vec![]); p.extend([(1, 'a')]); println!("{p:?}");`
- `collect()` into a `Box<str>`: `let b: Box<str> = ['a', 'b'].into_iter().collect(); println!("{b}");`
- `collect()` into an `Rc<[T]>`: `let r: std::rc::Rc<[i32]> = (1..4).collect(); println!("{r:?}");`
- `rposition()` of a lazy iterator held in a variable: `let v = [1, 2, 3]; let mut it = v.iter().map(|x| x + 1); println!("{:?}", it.rposition(|x| x == 3));`
- A map's `into_keys()` of values with a destructor, or its `into_values()` of keys with one, which Rust drops as it steps each entry (#432, [ADR 0422](../decisions/0422-an-iterator-holding-a-value-is-closed-to-drop-it.md), amended): `struct D; impl Drop for D { fn drop(&mut self) {} } let mut m = std::collections::HashMap::new(); m.insert(1, D); println!("{}", m.into_keys().count());`
- `HashMap::get_disjoint_mut`: `let mut m: std::collections::HashMap<&str, i32> = [("a", 1), ("b", 2)].into(); let [a, b] = m.get_disjoint_mut(["a", "b"]); println!("{:?} {:?}", a, b);`
- `rposition` of a `skip()` or a `take()`, of a held `map()`, or of a `map()` of a range or of the crate's iterator, which rust-js can't count yet: `let v = vec![1, 2, 3]; println!("{:?}", v.iter().skip(1).rposition(|&x| x == 2));`
- `rfind` of a `skip()` or a `take()`, which rust-js can't step from its back yet: `let v = vec![1, 2, 3]; println!("{:?}", v.iter().take(2).rfind(|x| **x > 1));`
- Stepping from its back a `chain()` or a `step_by()` of a `Vec`'s items that have a destructor, which is held where it's made: `struct L(u32); impl Drop for L { fn drop(&mut self) {} } let v: Vec<L> = vec![L(1), L(2)].into_iter().chain(vec![L(3)]).rev().collect(); println!("{}", v.len());`

## Numbers and text

- A C string, a `c"..."` literal: `let c = c"hi"; println!("{:?}", c);`
- `CString` and `CStr`: `let c = std::ffi::CString::new("hi").unwrap(); println!("{:?}", c);`

## Formatting

- A placeholder's options, a width or `{:x?}`, for a `dyn Debug`, which is the string it shows already ([ADR 0060](../decisions/0060-debug.md)): `let b: Box<dyn std::fmt::Debug> = Box::new(31u8); println!("{:x?}", b);`
- A `Formatter`'s flags asked by a `Display` of the crate's: `struct X; impl std::fmt::Display for X { fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result { write!(f, "{}", f.sign_minus()) } } println!("{:-}", X);`
- A `ToString` of the crate's with no `Display`: `struct A; impl ToString for A { fn to_string(&self) -> String { "A".into() } } println!("{}", A.to_string());`
- `{:?}` of one item of a collection of a `dyn` whose trait has `Debug` as a supertrait, `v[0]`, which is made where it's shown; the whole collection's compiles: `trait Ev: std::fmt::Debug {} #[derive(Debug)] struct C; impl Ev for C {} let v: Vec<Box<dyn Ev>> = vec![Box::new(C)]; println!("{:?}", v[0]);`

## Hashing

std's hashers and a crate's own hash as native Rust's
([ADR 0463](../decisions/0463-hashing-is-stds.md)); a `HashMap` or a `HashSet`
keys by value and never hashes ([ADR 0121](../decisions/0121-value-keys.md)).

- A `BuildHasher`, `RandomState`'s: `use std::hash::BuildHasher; let s = std::hash::RandomState::new(); println!("{}", s.hash_one(5u32) == s.hash_one(5u32));`
- `{:?}` of a std hasher: `println!("{:?}", std::hash::DefaultHasher::new());`
- Hashing a std type rust-js doesn't write: `use std::hash::{Hash, Hasher}; let mut h = std::hash::DefaultHasher::new(); std::time::Duration::from_secs(1).hash(&mut h); println!("{}", h.finish());`

## Panics, processes and I/O

- `panic::set_hook`: `std::panic::set_hook(Box::new(|_| {}));`
- `process::abort`: `std::process::abort();`
- Standard input: `let mut s = String::new(); let _ = std::io::stdin().read_line(&mut s);`
- The file system: `let _ = std::fs::read_to_string("x");`
- `panic::take_hook`: `let h = std::panic::take_hook(); drop(h);`

## Async, threads and time

There's one thread ([the semantics](semantics.md#async-threads-and-time)).

- Spawning a thread: `let h = std::thread::spawn(|| 1); println!("{}", h.join().unwrap());`
- Sleeping: `std::thread::sleep(std::time::Duration::from_millis(1));`
- A time's `{:?}`, its platform's fields: `println!("{:?}", std::time::Instant::now());`
- `future::poll_fn`: `let f = std::future::poll_fn(|_| std::task::Poll::Ready(1)); let _ = f;`
- An `IntoFuture` of the crate's given to generic code bound by `IntoFuture`, which awaits it as a promise: `struct F; impl std::future::IntoFuture for F { type Output = u32; type IntoFuture = std::future::Ready<u32>; fn into_future(self) -> Self::IntoFuture { std::future::ready(1) } } async fn g<T: std::future::IntoFuture<Output = u32>>(t: T) -> u32 { t.await } let _ = g(F);`
- A closure that holds a value with a destructor and only borrows it, given by value to a binding's type parameter that no `Fn` bound, nor a union's `Function` member, says is called once, as `js::set_timeout`'s handler is, or given to one from a variable, not where it's made ([ADR 1020](../decisions/1020-a-timer-is-the-js-crates-global.md)): `struct D; impl Drop for D { fn drop(&mut self) {} } #[cfg_attr(rust_js, rust_js::link_name = "setTimeout")] fn later<F: 'static>(f: F) { unreachable!() } let d = D; later(move || { let _ = &d; });`
- A `Wake` of the crate's: `struct W; impl std::task::Wake for W { fn wake(self: std::sync::Arc<Self>) {} } let w = std::task::Waker::from(std::sync::Arc::new(W)); drop(w);`

## serde

serde_json's common attributes work ([the semantics](semantics.md#the-javascript-boundary)).
Refused, each held by `test/diagnostics.test.ts`, which compiles them with
serde: a `u128` in JSON; reading a `BinaryHeap` or a `Box<str>`; writing a
`Cow<str>` or a `Duration`; a map keyed by a struct or an enum; an
internally tagged newtype variant of a map; a width for a `Value`;
`Value::pointer`, `pointer_mut` and `take`, assigning to a `Value`'s key,
and collecting into one; `Map::clear` and `map::Entry::or_insert`;
`to_vec`; an `Error`'s `line`, `column`, `is_syntax` and `classify`.

## React and JSX

- A component is a function; a namespace of them, `<AlertDialog.Root>`, is
  a module of them.

## Platforms and builds

- Releases are for macOS on Apple silicon ([the changelog](../../CHANGELOG.md)).
- What a crate compiles of another crate's itself, a generic function it
  gives a value to copy or drop that it takes nothing for, an impl that
  crate's JS hasn't, a destructor among them, or a function that copies
  what the crate changes in place, is refused where it reads its
  library's private `static` or thread-local, or calls or reads a binding
  its library names by a path relative to its own file, or sets its
  library's module variable, `export let`, which JS can't assign; a
  binding's `static` of a global, React's `__DEV__`, it reads as its
  library does
  ([ADR 0914](../decisions/0914-a-librarys-generic-function-is-its-programs.md)).
- A crate that would share what a library it uses represents otherwise
  is refused: counting an `Rc` of a type the library counts none of
  ([ADR 0852](../decisions/0852-each-types-rcs-are-counted-apart.md)), or
  making a `dyn Fn` that may hold a value with a destructor where the
  library holds a `dyn Fn` whose drop nothing
  ([ADR 0626](../decisions/0626-what-a-library-hands-a-crate.md)).
- What else isn't built yet is in [the architecture's "Not yet"](../architecture.md#not-yet)
  and [the roadmap](../../ROADMAP.md).
