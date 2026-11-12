# Known limitations

What rust-js doesn't do yet. It never writes JS that runs differently from
Rust by accident: what it can't compile as Rust runs, it refuses, with an
error that names it, `rust-js does not support .. yet`
([ADR 0006](../decisions/0006-errors-and-unsupported-features.md)). This page
lists what's refused. How a program that compiles behaves is
[the semantics](semantics.md), area by area, and where it deliberately
differs from native Rust, [its one list](semantics.md#differences-from-native-rust-in-one-list).

A refusal an open issue covers names it, `(#N)`, at the end of what it
says isn't supported, before `yet`, beside a decision it names, `(ADR
1114, #1283)`: ``rust-js does not support calling `std::hint::spin_loop`
(#1285) yet``; and the line here says it too, so
whoever meets one finds what's left to do. The words are the refusal's
own, as its owner writes them; the issue goes with the fix that lifts it.

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
into an array, its `add`, `offset_from` and `byte_offset_from`, a slice's
`as_ptr_range`, `ptr::swap_nonoverlapping`, `ptr::swap`, `ptr::replace`
and `ptr::copy` of what overlaps, each source read whole before it's
written, as std's are, and of more than one `[T; N]` through a pointer
to part of an array cast to an array's type, which `add` steps by its
array, a `transmute` between types of
numbers alone (`u16` to `[u8; 2]`, a struct of two `u32`s to a `u64`),
`slice::from_raw_parts` of an array's pointer, its items read as the
pointer's type, and `from_raw_parts_mut`'s, written where their bytes are,
a slice's pointer cast to an array's, `ptr::read(input as *const _ as
*const [u8; 16])` and its `ptr::write`, the slice's first items read or
written where they are (#1402),
a `u32` array's bytes written through a `&mut [u8]` (#1257), an allocation of `std::alloc::alloc`'s used as its bytes
([ADR 0417](../decisions/0417-an-allocation-is-an-array-of-slots.md)). A
field's address is in the allocation its object is stored in, however the
pointer reached it, through a reference, a call, an iterator: a crate that
asks an address records where its objects are stored where a reference to
them is made ([ADR 0562](../decisions/0562-a-pointer-is-a-place-and-an-address.md), amended). What a `RefCell`, a
`Cell`, a `Mutex` or an `RwLock` lends, an `Option`'s payload and a
variant's field are inside it, at the offset rustc lays them out at, past
a borrow flag, a lock's state or a tag; a pointer to an `Option` of a
struct, or a `RefCell` of one, which JS holds as the struct, made of a
reference to it, is the struct's address less that offset (#1114). A `#[repr(transparent)]`
struct is its field, so a `transmute` between them, by value or through
either reference, is the value itself
([ADR 0584](../decisions/0584-a-transparent-struct-is-its-field.md)). What
reads memory by where it is, it refuses:

- A raw address of an immutable `static`: `static N: u32 = 7; let p = &raw const N; println!("{}", unsafe { *p });`
- A `&mut` to a `static mut`, which Rust 2024 denies too (`static_mut_refs`); a pointer to it, `&mut *addr_of_mut!(N)`, compiles: `static mut N: u32 = 0; #[allow(static_mut_refs)] let r = unsafe { &mut N }; *r += 1;`
- A `transmute` of an enum, whose bytes a niche or a tag lays out: `#[repr(u8)] enum Size { One = 1 } let n: u8 = unsafe { std::mem::transmute::<Option<Size>, u8>(None) }; let _ = (n, Size::One);`
- A `static` of a reference to another `static`'s array or item; one to a literal or a `const` compiles: `static NAMES: [&str; 2] = ["x", "y"]; static REF: &[&str] = &NAMES; println!("{}", REF.join(""));`
- A shared reference to a number given to generic code that may let it out as a raw pointer, which would be of a copy ([1114](../decisions/1114-a-reference-a-pointer-may-leave-with-is-its-places-pointer.md)): `fn ptr_from_ref<T: ?Sized>(r: &T) -> *const T { r } let n = 5u32; let p = ptr_from_ref(&n); println!("{}", unsafe { *p });`
- `==` or `{:?}` of raw pointers in generic code to a type that holds a type parameter, `NonNull<Node<T>>`, whose size rust-js can't give: `struct Node<T>(T); fn same<T>(a: *const Node<T>, b: *const Node<T>) -> bool { a == b } let n = Node(1); println!("{}", same(&n, &n));`
- In a crate that asks an address, a pointer made of a reference in generic code, `r as *const T`, where the crate gives `T` both an `Option` of a struct, which JS holds as the struct, past its tag, and a type that isn't held so: one JS can't be both. Make it where the type is known (#1114): `#[repr(C)] struct P(u8, u32); fn at<T>(r: &T) -> usize { r as *const T as usize } let p = P(1, 2); let o = Some(P(3, 4)); println!("{} {}", at(&p), at(&o));`
- In a crate that asks an address of what it lends, std's function that lends a reference into its receiver's own storage where rust-js doesn't know it, `OnceCell::get`, `Result::as_ref`, `Peekable::peek`: take what it holds by value, keep it in a `RefCell`, an `Option` or a struct's field, or reach it by a pattern, `if let Ok(x) = &r` (#1114): `#[repr(C)] struct P(u8, u32); let c = std::cell::OnceCell::new(); let _ = c.set(P(1, 2)); println!("{}", c.get().unwrap() as *const P as usize);`
- An `Rc` of a `fn` pointer or item cloned, or upgraded from a `Weak`, before it's made an `Rc<dyn Fn>`, where the crate reads a `dyn Fn`'s address: each would be a function of its own, two of one allocation ([ADR 0959](../decisions/0959-ptr-eq-of-a-closure-made-with-its-rc.md), amended): `fn double(x: u32) -> u32 { x * 2 } let held: std::rc::Rc<fn(u32) -> u32> = std::rc::Rc::new(double); let a: std::rc::Rc<dyn Fn(u32) -> u32> = held.clone(); let b: std::rc::Rc<dyn Fn(u32) -> u32> = held; println!("{}", std::ptr::addr_eq(&*a, &*b));`

And refused when it runs, as a panic that says so, since which it is
depends on what the program does:

- A `TypeId`'s hash shown, which rust-js doesn't keep. Only an assertion
  that fails shows one: `assert_eq!` of two compiles. A pointer's address
  is shown, a number of rust-js's own ([ADR 0562](../decisions/0562-a-pointer-is-a-place-and-an-address.md)).
- Reading through a pointer with no place, an integer made a pointer, or
  the address of a value JS holds as itself, a number reached through a
  `&`; a pointer made of a reference to an `Option` that holds `None`,
  which JS holds as `undefined`, no place (#1114).
- A union's field read while it holds one whose bytes JS has none of, a
  `&str`'s or a `String`'s; plain data's bytes read as another type's
  ([ADR 0562](../decisions/0562-a-pointer-is-a-place-and-an-address.md)).
- A pointer to what has no bytes, a `String`, read as bytes; a pointer
  stepped outside an array or into what JS holds as a copy.
- An allocation of `alloc`'s used as its bytes and as another type's
  items both, or cast to another type's pointer past its first byte
  ([ADR 0417](../decisions/0417-an-allocation-is-an-array-of-slots.md)).

## Types

- A struct whose last field is a `dyn Debug`: `struct W<T: ?Sized> { n: u32, v: T } let w: Box<W<dyn std::fmt::Debug>> = Box::new(W { n: 1, v: 5u8 }); println!("{:?}", &w.v);`
- A struct whose last field is a `dyn`, behind a `Mutex`, which holds its value apart from the struct's `impl` (#375): `trait N { fn n(&self) -> u32; } struct A; impl N for A { fn n(&self) -> u32 { 1 } } struct W<T: ?Sized> { k: u32, v: T } let m = std::sync::Mutex::new(W { k: 0, v: A }); let d: &std::sync::Mutex<W<dyn N>> = &m; println!("{}", d.lock().unwrap().v.n());`
- A `RefCell` of a `dyn` of the crate's trait given to generic code as a cell of a type parameter, which its pair isn't: `trait N { fn n(&self) -> u32; } struct A; impl N for A { fn n(&self) -> u32 { 1 } } fn get<T: ?Sized + N>(c: &std::cell::RefCell<T>) -> u32 { c.borrow().n() } let c: &std::cell::RefCell<dyn N> = &std::cell::RefCell::new(A); println!("{}", get(c));`
- The `TypeId` of a type made of a type parameter, a closure's in generic code: `fn make<T: 'static>(t: T) -> std::any::TypeId { fn id<C: 'static>(_: &C) -> std::any::TypeId { std::any::TypeId::of::<C>() } let c = move || drop(t); id(&c) } println!("{:?}", make(1u8) == make(2u8));`
- `mem::discriminant` of a type only a caller knows, a type parameter's, whose variant generic code can't tell from its value alone (#1014, #1040): `fn same<T>(a: &T, b: &T) -> bool { std::mem::discriminant(a) == std::mem::discriminant(b) } println!("{}", same(&Some(1), &None));`
- `size_of_val` of a value without one size, a `dyn`'s: `let b: Box<dyn std::fmt::Debug> = Box::new(5u64); println!("{}", std::mem::size_of_val(&*b));`
- A map or set in generic code keyed by an enum made of a type parameter, `Option<T>`, whose class rust-js can't make of what its caller gives; a `T` itself is its caller's, a tuple's is its parts', an array's its item's, and a struct's its fields' where it derives what finds it, else its own impls' ([ADR 0466](../decisions/0466-a-generic-maps-class-is-its-callers.md), amended, #1346, #1360, #1371): `fn distinct<T: std::hash::Hash + Eq>(keys: Vec<Option<T>>) -> usize { keys.into_iter().collect::<std::collections::HashSet<_>>().len() } println!("{}", distinct(vec![Some(1), None]));`
- A `&mut` to an object given for a type parameter inside what generic code takes or gives, an `Option` or a field's part, which it has as a box and its caller as the object; a struct's field that holds it whole, regex-syntax's `ParserI<'s, P> { parser: P }` of a `&mut Parser`, holds generic code's box wherever the struct is (#1405), but a `&mut` to that field given to a function, `repoint(&mut slot.at, q)`, which would be to the box (#1421): `fn some<D>(d: Option<D>) -> Option<D> { d } let mut v = vec![1]; if let Some(r) = some(Some(&mut v)) { r.push(2); }`
- A `&mut` to an object given for a type parameter that's a closure's parameter: `fn apply<D, F: Fn(D) -> usize>(d: D, f: F) -> usize { f(d) } let mut v = vec![1]; println!("{}", apply(&mut v, |w: &mut Vec<i32>| w.len()));`
- A downcast of a `dyn Error` to a type whose `Error` impl is generic, whose dictionary keeps no `type_id` ([ADR 0331](../decisions/0331-dyn-any.md), amended): `#[derive(Debug)] struct W<E>(E); impl<E: std::fmt::Debug> std::fmt::Display for W<E> { fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result { write!(f, "{:?}", self.0) } } impl<E: std::fmt::Debug> std::error::Error for W<E> {} let e: Box<dyn std::error::Error> = Box::new(W(1u8)); println!("{}", e.is::<W<u8>>());`
- std's `From` of an `Option` or a `Vec<u8>` given to generic code bound by it, `impl Into<Option<u32>>`'s: `fn limit(l: impl Into<Option<u32>>) -> Option<u32> { l.into() } println!("{:?}", limit(5));`
- A `&mut` to an object of a type parameter's inside what generic code takes or gives, `Option<&mut T>` (#1421), though not a struct's field of a `&'a mut T`, which holds generic code's box wherever the struct is (#1405), nor a `PhantomData<&'a mut T>`, which holds no value: `struct P { x: i32 } struct Stack<T> { items: Vec<T> } impl<T> Stack<T> { fn top(&mut self) -> Option<&mut T> { self.items.last_mut() } } let mut s = Stack { items: vec![P { x: 1 }] }; if let Some(p) = s.top() { p.x += 1; }`
- A `&mut` to an object of a type parameter's inside what a method of a trait it's bound by takes or gives, which generic code, or a generic impl, calls through its dictionary, but within an `Option` or a tuple, which its impl's entry adapts: `Vec<&mut Self>` (#682, #685): `struct P { x: i32 } trait Pick { fn pick(&mut self, o: Vec<&mut Self>); } impl Pick for P { fn pick(&mut self, o: Vec<&mut Self>) { for o in o { o.x += 1; } } } fn go<S: Pick>(a: &mut S, b: &mut S) { a.pick(vec![b]) } let (mut a, mut b) = (P { x: 1 }, P { x: 2 }); go(&mut a, &mut b); println!("{}", b.x);`
- A clone of an `Rc<RefCell<dyn Trait>>` put where its type is written, a `Vec`'s item or a field: `use std::{cell::RefCell, rc::Rc}; trait Obs { fn hear(&self) -> u32; } struct A; impl Obs for A { fn hear(&self) -> u32 { 1 } } let e: Rc<RefCell<dyn Obs>> = Rc::new(RefCell::new(A)); let v: Vec<Rc<RefCell<dyn Obs>>> = vec![e.clone()]; println!("{}", v[0].borrow().hear());`
- A `Weak<RefCell<dyn Trait>>` of a counted `Rc`: `use std::{cell::RefCell, rc::{Rc, Weak}}; trait Obs { fn hear(&self) -> u32; } struct A; impl Obs for A { fn hear(&self) -> u32 { 1 } } let a = Rc::new(RefCell::new(A)); let w = Rc::downgrade(&a) as Weak<RefCell<dyn Obs>>; println!("{:?}", w.upgrade().map(|o| o.borrow().hear()));`
- Copying a `Copy` closure that changes what it captured, or what holds it, a tuple or a struct, where the crate never clones it; once it's cloned, or given to a `Copy` bound, each copy is its clone: `let mut n = 0; let mut bump = move || { n += 1; n }; let mut copy = bump; println!("{} {}", bump(), copy());`
- A lookup of a hash map or set whose keys are found by their own `Hash` and `Eq` by what a counted `Rc` key's `Borrow` gives, its value, which in JS is no key but the counts' object, `{ value, strong }` ([ADR 1120](../decisions/1120-a-hash-key-found-by-its-own-hash-and-eq.md), #1433); an `Rc` nothing counts, a `Box`, a `Vec` and a `String` are looked up by their `Borrow`'s value as std looks them up (#1288): `let k = std::rc::Rc::new(1u32); let s: std::collections::HashSet<std::rc::Rc<u32>> = [k.clone()].into(); println!("{} {}", s.contains(&1), std::rc::Rc::strong_count(&k));`
- Changing an `Option` through the `&mut` a call gives, `get_or_insert` or `insert` of `last_mut().unwrap()` or an entry's `or_insert`, which a binding of the `&mut` compiles: `let mut v = vec![None]; *v.last_mut().unwrap().get_or_insert(1) += 1; println!("{v:?}");`
- `Option<&T>` from a `&Option<T>`, `(&Some(7)).into()`: `let o: Option<&i32> = (&Some(7)).into(); println!("{o:?}");`
- A `RefCell`'s guard cloned, `Ref::clone`: `let c = std::cell::RefCell::new(1); let a = c.borrow(); let b = std::cell::Ref::clone(&a); println!("{} {}", *a, *b);`
- Ordering `RefCell`s, `<` or `cmp`; `==` compiles: `let a = std::cell::RefCell::new(1); println!("{}", a < std::cell::RefCell::new(2));`
- A flattened state's cell borrowed where another borrow of it may meet it, and a `replace_with` whose function may borrow it: it's its class's own keys, which count no borrows; borrow it for a moment (ADR 1044, #687): `#[cfg_attr(rust_js, rust_js::tag = "status")] enum S { #[cfg_attr(rust_js, rust_js::name = "a")] A, #[cfg_attr(rust_js, rust_js::name = "b")] B { n: u32 } } #[cfg_attr(rust_js, rust_js::class)] struct T { #[cfg_attr(rust_js, rust_js::flatten)] state: std::cell::RefCell<S> } #[cfg_attr(rust_js, rust_js::class)] impl T { fn new(state: S) -> Self { T { state: std::cell::RefCell::new(state) } } } fn reset(t: &T, now: bool) { if now { *t.state.borrow_mut() = S::A; } } let t = T::new(S::B { n: 1 }); let held = t.state.borrow(); reset(&t, false); println!("{}", matches!(*held, S::B { .. }));`
- A `OnceCell` made by `From`: `let c = std::cell::OnceCell::from(5); println!("{:?}", c.get());`
- `RefCell::into_inner` of a value with a destructor: `struct D(u32); impl Drop for D { fn drop(&mut self) {} } let c = std::cell::RefCell::new(D(1)); println!("{}", c.into_inner().0);`
- An `Rc` that's counted, `Rc::strong_count`, given on as an `Rc<dyn Trait>`: `trait Obs { fn n(&self) -> u32; } struct L; impl Obs for L { fn n(&self) -> u32 { 1 } } let l = std::rc::Rc::new(L); let v: Vec<std::rc::Rc<dyn Obs>> = vec![l.clone()]; println!("{} {}", v[0].n(), std::rc::Rc::strong_count(&l));`
- An `Into` of the crate's, not a `From`, given where any `Into` goes: `struct B; impl Into<String> for B { fn into(self) -> String { "B".into() } } fn takes(s: impl Into<String>) -> String { s.into() } println!("{}", takes(B));`
- A map key of an associated type: `trait E { type Id: std::hash::Hash + Eq; fn id(&self) -> Self::Id; } struct U; impl E for U { type Id = u32; fn id(&self) -> u32 { 1 } } fn index<T: E>(xs: Vec<T>) -> usize { xs.into_iter().map(|x| (x.id(), x)).collect::<std::collections::HashMap<_, _>>().len() } println!("{}", index(vec![U]));`
- `Rc::make_mut` of an `Rc` a `&mut self` holds: `struct C { cur: std::rc::Rc<Vec<i32>> } impl C { fn push(&mut self, x: i32) { std::rc::Rc::make_mut(&mut self.cur).push(x); } } let mut c = C { cur: std::rc::Rc::new(vec![1]) }; c.push(2); println!("{:?}", c.cur);`
- An `Rc<dyn Iterator>` of a value with a destructor; a `Box<dyn Iterator>` of one compiles ([ADR 0521](../decisions/0521-a-dyn-fn-carries-its-drop.md), amended): `struct D; impl Drop for D { fn drop(&mut self) {} } impl Iterator for D { type Item = u32; fn next(&mut self) -> Option<u32> { None } } let it: std::rc::Rc<dyn Iterator<Item = u32>> = std::rc::Rc::new(D); println!("{}", std::rc::Rc::strong_count(&it));`
- A derived `Default` of a struct holding a map keyed by a type parameter, made in generic code, which isn't given its keys' class; a written `impl Default` is: `#[derive(Default)] struct Node<K> { children: std::collections::HashMap<K, u8> } fn make<K: Default + Eq + std::hash::Hash>() -> Node<K> { Node::default() } println!("{}", make::<u32>().children.len());`
- A module's export whose JS name is `then`, a `pub fn`, `static` or `thread_local!` of a module or a `pub use` of one, which makes the module a thenable that `import()` of it calls as `then(resolve, reject)`; give it another JS name, `#[cfg_attr(rust_js, rust_js::name = "thenChunk")]` ([ADR 1106](../decisions/1106-a-module-exports-nothing-named-then.md)). A method named `then`, its type's, and a function the module keeps to itself are no export of it.
- A field whose JS name is `__proto__`, `Object.prototype`'s accessor, which would set or read the object's prototype where the field isn't its own; give it another JS name, `#[cfg_attr(rust_js, rust_js::name = "proto")]` ([ADR 1084](../decisions/1084-a-field-named-proto-is-refused.md)): `struct Linked { __proto__: Option<u32> } let l = Linked { __proto__: None }; println!("{}", l.__proto__.is_none());`

## Modules

- A `pub use m::*;` of a module that holds a module or a class with JS, whose namespace or constructor it would re-export: `pub use` each of its names that has JS by its name ([ADR 0874](../decisions/0874-a-modules-pub-uses-are-re-exported-as-written.md), amended).
- A `pub use` of another rust-js crate's module, whose namespace no file of this crate's is: `pub use` each of its items by its name.
- Another module's write of a module's variable, a thread-local of a value or a `static mut`, whole, of a field or an item, or a pointer taken to it, which JS lets only its own module set: write a `pub fn` in its module that sets it ([ADR 0096](../decisions/0096-statics.md), amended): `mod counter { pub static mut LINES: usize = 0; } unsafe { counter::LINES = 3; }`
- An exported `static mut` its module gives a pointer to on, `add(&raw mut TOTAL)`, which a JS module importing it would read as its box: read and write it by its name, or through a pointer only where it's taken ([ADR 0096](../decisions/0096-statics.md), amended): `mod total { pub static mut TOTAL: u32 = 0; fn add(t: *mut u32) { unsafe { *t += 1 } } pub fn bump() { add(&raw mut TOTAL) } } total::bump();`
- An exported thread-local whose module needs its cell, which a JS module importing it would read in place of what it holds: it hands on the cell or the `&mut` of what it holds, or borrows it in a `with`. Use what `with_borrow_mut` gives in its closure ([ADR 0270](../decisions/0270-module-variables.md), amended): `mod depth { use std::cell::RefCell; thread_local! { pub static DEPTH: RefCell<Option<u32>> = const { RefCell::new(None) }; } fn fill(d: &mut Option<u32>) { d.get_or_insert(3); } pub fn deepen() { DEPTH.with_borrow_mut(|d| fill(d)); } } depth::deepen(); println!("{:?}", depth::DEPTH.with_borrow(|d| *d));`
- A `pub` thread-local whose initializer prints, may panic, reads state a function that may run later sets, or changes state one reads, which JS's importers would read made as its module loads, where Rust makes it at its first use: make it `pub(crate)`, made where it's first used, and export a `pub fn` that reads it ([ADR 1006](../decisions/1006-a-pub-thread-local-is-made-as-its-module-loads.md), amended, #554): `use std::cell::Cell; thread_local! { static LEN: Cell<u32> = const { Cell::new(1) }; pub static N: u32 = LEN.get() * 2; } LEN.set(5); println!("{}", N.with(|n| *n));`
- A `pub` thread-local, made as its module loads, whose initializer runs or reads code of a module in an import cycle with its own, reads itself, or calls a `dyn Fn` or a fn pointer that may be any function, which may not be made yet then (a `dyn Fn` its call was given a function or a closure, coerced there, is followed to it, and a closure it clones isn't run by its clone, rd542): move what it runs out of the cycle, or call what it runs by its name, and make one that reads itself of what it reads instead ([ADR 1006](../decisions/1006-a-pub-thread-local-is-made-as-its-module-loads.md)): `thread_local! { pub static N: u32 = twice(); } fn twice() -> u32 { N.with(|n| *n) * 2 } println!("{}", N.with(|n| *n));`
- Two exports of one module with one JS name, of which an importer reads one by that name: give one another with `#[rust_js::name]` ([ADR 0922](../decisions/0922-an-export-is-named-first.md)): `mod order { pub fn sort_index() -> u32 { 1 } #[rust_js::name = "sortIndex"] pub fn index_for_sorting() -> u32 { 2 } } println!("{}", order::sort_index() + order::index_for_sorting());`
- An impl's dictionary made where its module loads, which the crate's
  own code reads, in an import cycle with a module that runs code where it
  loads, a `thread_local!` made by a binding's call or a global's read, or
  `js::on_load!`, that may reach the dictionary before it's made; a
  constant there, made of numbers, runs none. Move the impl or that code
  out of the cycle
  ([ADR 0592](../decisions/0592-a-dictionary-is-its-modules-const.md), amended).

## Iterators and collections

- A clone of a lazy iterator generic code makes where its source isn't bound by `Clone`, but a `map()`, std's struct where it's held ([ADR 0871](../decisions/0871-rev-is-stds.md)): `fn evens<I: Iterator<Item = i32>>(it: I) -> std::iter::Filter<I, fn(&i32) -> bool> { it.filter(|x| x % 2 == 0) } let d = evens(1..9); let e = d.clone(); println!("{:?} {:?}", d.count(), e.count());`
- In the default build, generic code's adapter made only of its type parameters, a `chain()` of two `I`s, stepped from its back where its parameters aren't bound by `Clone`, beside another generic function that clones such an adapter of its own, where the first's callers aren't all seen, a `pub` function's or a library's, or where a caller gives both the same types: it may be an instance that's cloned apart, which it can't make without `Clone` ([ADR 0525](../decisions/0525-a-cloned-map-is-made-again.md), amended): `pub fn last<I: DoubleEndedIterator<Item = i32>>(a: I, b: I) -> Option<i32> { a.chain(b).map(|x| x + 1).next_back() } fn cycled<J: Iterator<Item = i32> + Clone>(a: J, b: J) -> Vec<i32> { a.chain(b).cycle().take(5).collect() }`. Where its callers are all seen and give it other types than the clone's, it compiles.
- A clone of a `copied()` of an iterator dropped as it's closed: the crate's own with a destructor, or one generic code is given its drop of, `$closing(it, drop)` ([ADR 0525](../decisions/0525-a-cloned-map-is-made-again.md), amended): `#[derive(Clone)] struct Loud(u32); impl Iterator for Loud { type Item = &'static u32; fn next(&mut self) -> Option<&'static u32> { None } } impl Drop for Loud { fn drop(&mut self) { println!("drop"); } } let c = Loud(0).copied(); let d = c.clone(); println!("{}", d.count());`
- `eq()` of two `TrustedLen` iterators whose lengths rust-js can't count before stepping them, where stepping them runs code, a map's, as std's `eq` asks their lengths first and steps neither of unequal ones ([ADR 0998](../decisions/0998-eq-of-uncounted-trusted-len-iterators-is-refused.md)): `let v = vec![1u32, 2, 3]; let w = vec![1u32]; println!("{}", v.iter().map(|x| { println!("a"); *x }).eq(w.iter().map(|x| *x)));`
- Giving generic code that compares it by `eq()` a `TrustedLen` iterator JS can't count before stepping it, where stepping it runs code, a map's, or its items have a destructor: std's asks its length first; an array, a `Vec`, an array's iterator or a range is counted (#386, #1182, [ADR 1014](../decisions/1014-generic-eq-of-trusted-len-is-refused-at-its-caller.md), amended): `fn same<I: Iterator<Item = u32>>(a: I, b: I) -> bool { a.eq(b) } let v = vec![1u32, 2]; let f = |x: &u32| { println!("a"); *x }; println!("{}", same(v.iter().map(f), v[..1].iter().map(f)));`
- Giving generic code a zip it folds, collects or steps, of sides std stops at the shorter of, where a step of its first side runs code: its JS steps it as `next()` does, its first side once more ([ADR 1014](../decisions/1014-generic-eq-of-trusted-len-is-refused-at-its-caller.md)): `fn pairs<A: Iterator<Item = u32>, B: Iterator<Item = u32>>(a: A, b: B) -> Vec<(u32, u32)> { a.zip(b).collect() } let v = vec![1u32, 2, 3]; println!("{:?}", pairs(v.iter().map(|x| { println!("a"); *x }), v[..2].iter().copied()));`
- A collection of the crate's given where any `IntoIterator` goes, which wouldn't call its `into_iter`: `struct Cart(Vec<u32>); impl IntoIterator for Cart { type Item = u32; type IntoIter = std::vec::IntoIter<u32>; fn into_iter(self) -> Self::IntoIter { self.0.into_iter() } } let mut v = vec![1]; v.extend(Cart(vec![2])); println!("{v:?}");`
- Looking a key up by what the crate's own `Borrow` gives: `use std::borrow::Borrow; use std::collections::HashMap; #[derive(PartialEq, Eq, Hash)] struct K(String); impl Borrow<str> for K { fn borrow(&self) -> &str { &self.0 } } let mut m = HashMap::new(); m.insert(K("a".into()), 1); println!("{:?}", m.get("a"));`
- `next_back()` of a lazy `lines()`, `split_whitespace()`, `split_terminator()` or `rsplit()`, whose `.rev()` compiles: `let mut l = "a\nb".lines(); println!("{:?}", l.next_back());`
- `VecDeque::remove` in generic code given an `Option` for its item, whose `None` the result's could look like: `fn take<T>(q: &mut std::collections::VecDeque<T>, i: usize) -> Option<T> { q.remove(i) } let mut q = std::collections::VecDeque::from(vec![None, Some(2)]); println!("{:?}", take(&mut q, 0));`
- `size_hint()` of an adapter (`filter_map` and the like; a `filter`'s and a `take`'s are std's of their source's, [0170](../decisions/0170-size-hint.md), amended, and a `skip_while`'s, a `take_while`'s, a `map_while`'s and a `scan`'s, the runtime's own, #1169; of an adapter made in place inside another, as JS's own, `v.into_iter().skip(1).filter(f)` held, #1427) of a slice's, a collection's or a type parameter's iterator, and of what's made of a `str`'s `chars` in place, `s.chars().map(f)`, whose `chars()` itself hints as std's ([1117](../decisions/1117-a-strings-chars-are-lazy.md)); of an adapter of the crate's iterator or a range it's std's ([1092](../decisions/1092-the-iterator-protocol-is-one-contract.md)), and of one of `repeat_n`, `repeat`, `repeat_with`, `successors`, `from_fn`, `once` or `once_with` too ([0170](../decisions/0170-size-hint.md), amended, #1357, #1375): `let v = [1, 2, 3]; println!("{:?}", v.iter().filter_map(|x| Some(*x)).size_hint());`
- A `&mut` from `flatten()` of an `iter_mut()`: `let mut g = vec![vec![1u64], vec![2]]; for x in g.iter_mut().flatten() { *x += 1; } println!("{g:?}");`
- `slice::from_mut` of an object, whose place `s[0] = other` would replace (#1336); of a number or text it compiles, a one-item view: `let mut v = vec![1]; std::slice::from_mut(&mut v)[0] = vec![2]; println!("{v:?}");`
- `collect()` into a `Vec` of what has no end only as what it holds says: a `cycle()` chained with what has an end, a `cycle()` of what's partly filtered, or a `step_by(n)` of what has no end. std's hint makes these panic `capacity overflow` or not as they run (#433, [ADR 0940](../decisions/0940-collect-asks-room-of-its-hint.md), amended): `let v = [1, 2]; let c = v.iter().cycle(); let all: Vec<&i32> = c.chain([3].iter()).collect(); println!("{}", all.len());`
- In a crate that reads no `Vec`'s capacity, a call that gives generic code's `extend()` (`C: Extend<T>`) a `Vec`, a `VecDeque` or a `BinaryHeap` and an iterator whose `size_hint()` is the crate's own, where both are known: std asks that hint as the collection fills, as its capacity says, which such a crate doesn't keep; collect the iterator first and give that, or extend a collection made there. A caller of the caller that knows them is refused where they're its own type parameters ([1097](../decisions/1097-when-a-collect-asks-its-hint-is-hint-asks.md), the user's decision): `struct L(u32); impl Iterator for L { type Item = u32; fn next(&mut self) -> Option<u32> { if self.0 == 0 { None } else { self.0 -= 1; Some(self.0) } } fn size_hint(&self) -> (usize, Option<usize>) { println!("hint"); (0, None) } } fn fill<C: Extend<u32>>(c: &mut C) { c.extend(L(2)); } let mut v = Vec::new(); fill(&mut v); println!("{}", v.len());`
- In a crate that reads a `Vec`'s capacity, a `Vec` grown by an iterator that isn't `TrustedLen` and whose JS hint hasn't std's lower bound, by which std grows it: an adapter of a string's `chars()` or `char_indices()`, `chars().skip(n)`, whose JS adapters of the runtime's `$chars` keep no hint, where `chars()` itself hints as std's ([1117](../decisions/1117-a-strings-chars-are-lazy.md), #1200, #1316), and a slice's `split(..)`, which JS counts ([0419](../decisions/0419-a-vecs-capacity-kept-where-asked.md), amended, #1180), a `chain` of one among them. A B-tree set's `difference(..)` and `union(..)` and a hash set's `union(..)`, whose lower bound std's merge gives as it's stepped, compile, made as std steps them (#1219). An exact-sized one collected alone, `b.iter().skip(1)`, compiles, given knowing its length, and so does a `chain` or a `zip` of them, `b.iter().skip(1).chain(rest)`, each part made knowing its hint (#1216), and so does one whose hint `size_hint()` finds std's where it's made, a `take(n)` of `successors` or a `rev()` of a `flat_map` (#1316): `let v: Vec<char> = "héllo".chars().skip(1).collect(); println!("{}", v.capacity());`
- In a crate that reads a `Vec`'s capacity, a bound's `extend()` (`C: Extend<T>`) of a `Vec<T>` whose items are a type parameter's, whose size std grows it by and its dictionary isn't given; call the `Vec`'s own `extend` (#1180): `fn fill<C: Extend<T>, T>(c: &mut C, x: T) { c.extend(Some(x)); } fn f<T>(x: T) -> usize { let mut v = Vec::new(); fill(&mut v, x); v.capacity() } println!("{}", f(1u8));`
- In a crate that reads a `Vec`'s capacity, compiled without `--std-iterators`, a `collect()` std makes in place of a chain an `impl Iterator` function gives, `fn evens(v: Vec<u32>) -> impl Iterator<Item = u32> { v.into_iter().filter(..) }` then `evens(v).map(f).collect()`: JS's own iterator helpers keep no link to the `Vec` they step, so its capacity isn't known, and `capacity()` of it is refused where it runs, naming #1180, never another number. With `--std-iterators`, std's structs link each adapter to its source and it has std's capacity. Won't be lifted: links would cost the helpers' readability everywhere such a crate collects, for a capacity read (#1428, closed; [0419](../decisions/0419-a-vecs-capacity-kept-where-asked.md)). Collect where the `Vec` is a variable of the body, `v.into_iter().filter(..).map(f).collect()`.
- `extend()` of a pair of collections: `let mut p: (Vec<i32>, Vec<char>) = (vec![], vec![]); p.extend([(1, 'a')]); println!("{p:?}");`
- `collect()` into a `Box<str>`: `let b: Box<str> = ['a', 'b'].into_iter().collect(); println!("{b}");`
- `collect()` into an `Rc<[T]>`: `let r: std::rc::Rc<[i32]> = (1..4).collect(); println!("{r:?}");`
- A map's `into_keys()` of values with a destructor, or its `into_values()` of keys with one, which Rust drops as it steps each entry (#432, [ADR 0422](../decisions/0422-an-iterator-holding-a-value-is-closed-to-drop-it.md), amended): `struct D; impl Drop for D { fn drop(&mut self) {} } let mut m = std::collections::HashMap::new(); m.insert(1, D); println!("{}", m.into_keys().count());`
- `rposition` of a `skip()` or a `take()`, or of a `map()` of a range or of the crate's iterator, which rust-js can't count yet: `let v = vec![1, 2, 3]; println!("{:?}", v.iter().skip(1).rposition(|&x| x == 2));`

## Numbers and text


## Formatting

- A `Formatter`'s flags asked by a `Display` of the crate's: `struct X; impl std::fmt::Display for X { fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result { write!(f, "{}", f.sign_minus()) } } println!("{:-}", X);`
- A `ToString` of the crate's with no `Display`: `struct A; impl ToString for A { fn to_string(&self) -> String { "A".into() } } println!("{}", A.to_string());`
- `{:?}` of one item of a collection of a `dyn` whose trait has `Debug` as a supertrait, `v[0]`, which is made where it's shown; the whole collection's compiles: `trait Ev: std::fmt::Debug {} #[derive(Debug)] struct C; impl Ev for C {} let v: Vec<Box<dyn Ev>> = vec![Box::new(C)]; println!("{:?}", v[0]);`

## Hashing

std's hashers and a crate's own hash as native Rust's
([ADR 0463](../decisions/0463-hashing-is-stds.md)); a `HashMap` or a `HashSet`
keys by value and never hashes ([ADR 0121](../decisions/0121-value-keys.md)).
std's wrappers and enums, `Reverse`, `Wrapping`, `ManuallyDrop`, a `Pin` of
std's pointer, a `Cow`, `Bound`, `Poll` and `ControlFlow`, hash as std's
impls write them, a crate's own `Hash` inside called as std calls it (ADR
0463, amended; #1289).

- A `BuildHasher`, `RandomState`'s: `use std::hash::BuildHasher; let s = std::hash::RandomState::new(); println!("{}", s.hash_one(5u32) == s.hash_one(5u32));`
- `{:?}` of a std hasher: `println!("{:?}", std::hash::DefaultHasher::new());`
- Hashing a std type rust-js doesn't write, a `Duration`, alone or in a key whose `Hash` runs code (#1447): `use std::hash::{Hash, Hasher}; let mut h = std::hash::DefaultHasher::new(); std::time::Duration::from_secs(1).hash(&mut h); println!("{}", h.finish());`

## Panics, processes and I/O

- `panic::set_hook`: `std::panic::set_hook(Box::new(|_| {}));`
- `process::abort`: `std::process::abort();`
- Standard input: `let mut s = String::new(); let _ = std::io::stdin().read_line(&mut s);`
- The file system past a file's reads and writes, its metadata and directories (#1418): `let _ = std::fs::rename("x", "y");`
- `{:?}` of a `Metadata` (#1418): `let m = std::fs::metadata("x").unwrap(); println!("{m:?}");`
- A `VecDeque<u8>`'s `read` and `fill_buf`, which read its front slice alone (#1302): `use std::io::Read; let mut d = std::collections::VecDeque::from(vec![1u8]); let _ = d.read(&mut [0u8; 1]);`
- A `VecDeque<u8>` as generic code's reader (#1302): `fn first<R: std::io::Read>(mut r: R) -> u8 { let mut b = [0u8]; r.read_exact(&mut b).unwrap(); b[0] } let _ = first(std::collections::VecDeque::from(vec![1u8]));`
- `{:?}` of a `BufReader` (#1418): `let r = std::io::BufReader::new(&b"x"[..]); println!("{r:?}");`
- `get_mut` of a `BufReader` of a `&mut` to an object, a reader: `let mut c = std::io::Cursor::new(vec![1u8]); let mut r = std::io::BufReader::new(&mut c); r.get_mut().set_position(0);`
- `panic::take_hook`: `let h = std::panic::take_hook(); drop(h);`

## Async, threads and time

There's one thread ([the semantics](semantics.md#async-threads-and-time), [ADR 1126](../decisions/1126-a-program-has-one-thread.md)).

- Spawning a thread: `let h = std::thread::spawn(|| 1); println!("{}", h.join().unwrap());`
- Sleeping: `std::thread::sleep(std::time::Duration::from_millis(1));`
- `future::poll_fn`: `let f = std::future::poll_fn(|_| std::task::Poll::Ready(1)); let _ = f;`
- A future of the crate's held while code that may be seen runs, then awaited, which runs as it's made in JS (#965, [ADR 0029](../decisions/0029-async-await.md), amended): `async fn greet() { println!("hi"); } async fn later() { let f = greet(); println!("first"); f.await; }`
- A future of the crate's dropped unawaited, whose body may be seen: `async fn greet() { println!("hi"); } async fn never() { let f = greet(); drop(f); }`
- A future of the crate's, whose body may be seen, given to a function of the crate's or another's that may await it later, which rust-js can't follow (give it to `js::promise` or `js::spawn`, or await it where it's made): `async fn greet() { println!("hi"); } async fn later<F: std::future::Future>(f: F) { println!("first"); f.await; } async fn outer() { later(greet()).await; }`
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
  crate's JS hasn't, a destructor among them, a function that copies
  what the crate changes in place, or one whose JS may give one function
  where Rust makes two, which the crate compares, is refused where it reads its
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
  ([ADR 0626](../decisions/0626-what-a-library-hands-a-crate.md)); the
  refusal names the `dyn Fn` and what it holds, and of callbacks' cell,
  the call whose return doesn't own them
  ([ADR 0317, amended](../decisions/0317-once-cells.md#amendment-a-callback-a-public-field-holds-isnt-the-returns-alone-rd553)).
- What else isn't built yet is in [the architecture's "Not yet"](../architecture.md#not-yet)
  and [the roadmap](../../ROADMAP.md).
