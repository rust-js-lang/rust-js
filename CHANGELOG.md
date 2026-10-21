# Changelog

Each release of rust-js's compiler and the packages released with it, at its
version: `@rust-js/native`, `@rust-js/build`, `@rust-js/typescript`,
`@rust-js/runtime`, `@rust-js/create`, `@rust-js/react` and `@rust-js/next`
([ADR 0120](docs/decisions/0120-first-npm-release.md)). `@rust-js/builtins`,
`@rust-js/webapi` and `@rust-js/node` are released on their own, each at its
version ([ADR 0116](docs/decisions/0116-binding-versions.md)).

## 0.0.4

For macOS on Apple silicon, with Rust 1.99.0, from 1.98.1. `@rust-js/builtins`
is 0.0.2 and `@rust-js/webapi` 0.0.3, each its own release (ADR 0116). New
with it: `@rust-js/std`, std's own bodies compiled to JS, which the JS
rust-js writes imports as it imports `@rust-js/runtime`, so an app installs
both (ADR 0557); `@rust-js/typescript`, which prints each module's `.d.ts` with
TypeScript 7.1.0-beta, a beta, pinned (ADR 0206);
`@rust-js/next`, Next.js's crate, at 0.0.4; and `@rust-js/node`, Node.js's,
at its own 0.0.1. `@rust-js/vite-plugin` and `@rust-js/resources` aren't
released any more: `rust-js build` is the one way to build an app.

### New

- **std's bodies are std's own** (ADR 0557): a std function whose body
  reaches no memory is compiled from the MIR the toolchain ships into
  `@rust-js/std`, a library built into the compiler, and a `Peekable` is
  std's struct, `{ iter, peeked }`: `peekable()` of any iterator, the
  crate's own too, `next`, `next_back`, `count`, `nth`, `last`, `fold`,
  `rfold`, `next_if`, `next_if_eq`, its clone and its `Debug` are std's;
  `peek`, `peek_mut`, `size_hint` and `len`, whose bodies reach memory,
  are rewritten over it. Generic code that gives back a `peekable` of a
  type with a destructor compiles. `--std-coverage` says what each std
  body is made of, `safe`, `spec`, `opaque`, `memory` or `none`, and
  measures `Iterator`'s provided methods and the adapters.
- **A `copied()` given back as an `impl Iterator`**: regex-syntax
  compiles (ADR 0556).
- **A library's struct with a derived `Ord`**, compared and sorted: uuid
  runs as native Rust, and hex too (ADR 0555).
- **A filter or a map an `Either` holds, `rev()`'d**: either runs as
  native Rust (ADR 0554).
- **`mem::replace` of a field holding a generic iterator**, in a method
  or a closure (ADR 0553).
- **An `IntoIterator` whose iterator a generic function holds**, cloned
  and stepped as it is: itertools runs as native Rust does (ADR 0552).
- **A `&mut Self` given on where an `IntoIterator` goes**, through a
  dictionary too: itertools' `collect_tuple` (ADR 0551).
- **A `Fuse` a struct holds, stepped from its back** where it's known to
  have one, wherever it was made (ADR 0549); and an impl's `as_mut()` of
  an array it knows (ADR 0550).
- **A library's generic iterator and its extension trait's defaults**,
  stepped by what its consumer gives (ADR 0548).
- **`copied()` and `cloned()` of an iterator with a destructor**, dropped
  where Rust drops it (ADR 0547).
- **A map the crate steps from its back, held in a variable**, a library's
  too: itertools compiles whole (ADR 0545).
- **A `Cycle` a struct holds**: its `Debug` and its clone; and `{:?}` of
  an `iter::Empty` (ADR 0544).
- **A `Peekable` a struct holds**: its `Debug`, its clone and its size
  hint; and a generic iterator's `Fuse` keeps a `FusedIterator` as it ends
  (ADR 0543).
- **An `Enumerate` a struct holds**: its size hint, its back where its
  impl's bounds give it, and its `Debug`, as Rust's (ADR 0542).
- **A trait's default that calls its trait on an adapter of `Self`**,
  `self.enumerate().minmax()`, of an impl over `?Sized` types (ADR 0541).
- **A closure that writes to its writer's `Formatter`**, `|disp| disp.fmt(f)`;
  **`try_for_each`**; and `try_fold` into an `Option` of what could look
  like `None` (ADR 0539).
- **A writer called through a `fn` pointer** given its `Formatter`,
  `cb(x, f)`; std's trait method as such a value is its function (ADR 0538).
- **A `Cell`'s `take()`, `replace()` and `get()` of an `Option` of what
  could look like `None`**, `Cell<Option<I>>`'s (ADR 0537).
- **A blanket impl over `?Sized` types**, `impl<T: Iterator + ?Sized> Ext
  for T {}`, that keeps its trait's defaults (ADR 0536).
- **`rev()` of a lazy `map`** (ADR 0535).
- **A bound of `Index<J>` or `IndexMut<J>` on a `Vec`, an array or a
  slice**, of a generic index (ADR 0534).
- **`as_mut()` of an associated type**, `T::Buffer` (ADR 0533).
- **A `Debug` builder held in a variable**, stepped in a branch or a loop
  (ADR 0532).
- **`try_rfold`** (ADR 0531).
- **A clone of a string's `chars()` and `char_indices()`, an `Option`'s
  and a `Result`'s `iter()`, a `VecDeque`'s and a `LinkedList`'s, and a
  `rev()`; `next_back` of a `rev()` of an array's iterator; and a
  `Box<[T]>` collected and cloned** (ADR 0530).
- **memchr 2.8.3's searches**, `memchr`, `memchr2`, `memchr3`, their
  `memrchr`s and their iterators, over the byte arrays slices are; another
  memchr, a fork or another version, is refused where it's called, saying
  which to depend on (ADR 0529).
- **A `fuse` and an `enumerate` of a double-ended or exact-size
  iterator, from either end** (ADR 0528).
- **An impl whose associated type is an iterator over its type
  parameter, in a library** (ADR 0527).
- **A `filter` or `filter_map` of a double-ended iterator, stepped from
  its back** (ADR 0526).
- **A clone of a lazy `map` or `filter`** (ADR 0525).
- **A counted `Rc` in a library**: every `Rc` a library, or a crate
  using one, has is counted, one representation they share; libraries
  are ABI 3 (ADR 0523). And an `Rc` written through a `&mut`.
- **A library's `dyn Trait` in its consumer** (ADR 0524).
- **A clone of a closure that changes what it took, holds a `Cell`, or
  holds a counted `Rc`** (ADR 0522).
- **A `Box<dyn Fn>` of a function holding a value with a destructor**
  (ADR 0521).
- **`Rc::downcast` of a counted `Rc<dyn Any>`, and a `dyn Any`'s
  downcasts of what may hold a value with a destructor** (ADR 0520).
- **A `Cell` or a `OnceCell` of a value with a destructor** (ADR 0519).
- **A held `filter_map` or `map_while` of items with a destructor** (ADR 0518).
- **`scan` of items with a destructor** (ADR 0517).
- **`rev()` of an array's items with a destructor** (ADR 0516).
- **`map_while`** (ADR 0515).
- **`peekable()` of what has a destructor** (ADR 0514).
- **`iter::repeat_n` and a `Peekable`'s `peek_mut`** (ADR 0513).
- **`flat_map` and `flatten` of what has a destructor** (ADR 0510).
- **An array's `map`, `unwrap_or_default`, `Option::as_mut`, `pop_if`,
  `pop_front_if` and capacity changes, of values with a destructor** (ADR
  0509).
- **`fill`, `resize`, `clone_from_slice`, `dedup`, `extend_from_within`
  and a slice's lending views, of values with a destructor; `dedup()` of
  structs** (ADR 0507).
- **`extract_if`, `HashMap::from` and `collect()` into a map, of values
  with a destructor** (ADR 0506).
- **`rc.as_ref()` of an `Rc`** (ADR 0505).
- **`{}` and `{:?}` of a counted `Rc` of a reference** (ADR 0504).
- **`splice`, and `inspect` of an `Option` or a `Result`, of values with
  a destructor** (ADR 0501).
- **`VecDeque::from(v)` and the like of values with a destructor** (ADR
  0500).
- **`retain_mut` of values with a destructor**, dropping each it removes
  (ADR 0499).
- **A generic `a + b`, or another std operator's, whose `Output` has a
  destructor**, dropped by its dictionary's `$dropOutput` (ADR 0497).
- **`and`, `xor` and `zip` of values with a destructor**, dropping what
  they give up (ADRs 0496, 0498).
- **An `Rc` of a type parameter's value with a destructor**, itertools'
  `tee`'s buffer, counted and dropped by its last clone, and `pop_front`
  of a type parameter's items (ADR 0494).
- **`rev()` of a zip of two arrays' iterators**, `iter_mut()`'s too, paired
  from their backs as std's `Zip` pairs them (ADR 0493).
- **`cycle()` of a range, the crate's own iterator or a type parameter's**,
  a clone kept and stepped afresh each round, as std's `Cycle` (ADR 0491).
- **`unwrap_or`, `ok_or`, `or` and `then_some` of a value with a
  destructor**, the one they leave unused dropped there (ADR 0490).
- **Items with a destructor of an iterator without one**, dropped where
  generic code discards them, where they leaked (ADR 0489).
- **A type holding an iterator bound by `DoubleEndedIterator` or
  `ExactSizeIterator` alone**, given as it is, as one bound by `Iterator`
  is, either's `IterEither` among them (ADR 0488).
- **`next_back()`, `len()` and `rev()` of an iterator given as it is**, by
  its `DoubleEndedIterator` and `ExactSizeIterator` dictionaries, and one
  given on where its back or length is asked, or to `chain` (ADR 0487).
- **`get_or_insert_with`, `insert`, `take_if` and the rest of an
  `Option<Option<T>>`'s place-changing methods**, through its `Some`'s box
  (ADR 0486).
- **`map(f)`, `filter(p)` and `skip_while(p)` of a function holding a value
  with a destructor**, closed to drop it with the adapter (ADR 0485).
- **A closure holding a value with a destructor given back by its maker**,
  `-> impl Fn`, carries its drop, `Symbol.dispose`, run where it's dropped
  (ADR 0484).
- **`borrow_mut()` through a bound, and `borrow()` through a `BorrowMut`
  supertrait**, as itertools' `PoolIndex` has (ADR 0483).
- **A slice's pointer cast between `MaybeUninit<T>` and `T`**, the slice
  itself (ADR 0482).
- **`*b = other` of a `&mut` to an iterator given as it is**, written in
  place, as of any object (ADR 0481).
- **An `I::IntoIter: DoubleEndedIterator` stepped from its back**, `rev()`
  and `rfind`, and a `Copied` of a slice's iterator given as one (ADR 0480).
- **`reduce`**, std's, `None` of none (ADR 0479); and values of an
  associated type of an associated type, `<I::Item as IntoIterator>::Item`,
  dropped as std drops them (ADR 0478).
- **`From::from` through a bound**, `W::from(u)` of a `W: From<U>` and of an
  associated type a trait bounds by `From`, its caller's dictionary's
  (ADR 0477).
- **`sum()` and `product()` into a type parameter**, and `collect()` into a
  `Result<U, E>` a bound names, through their caller's dictionaries (ADR
  0476).
- **A `Fuse`'s `size_hint`, `Clone` and `Debug`**, std's, in generic code
  too, and `Debug` of an iterator over an array, `IntoIter([2, 3])` (ADR
  0473).
- **What a `Some` holds, an `Option` of a type parameter**, by `unwrap`,
  `unwrap_or_else` and the rest, and `max_by`'s and `min_by`'s pick of one;
  and no `Iterator.from` around a lazy adapter's generator (ADR 0472).
- **An unstable sort of a type parameter's items**, `sort_unstable`,
  `select_nth_unstable` and their `_by` and `_by_key`, sorted as its
  caller's type picks, given as `TSort` (ADR 0471).
- **A closure called by `&` or `&mut` that holds a value with a
  destructor, given by value**: to std's `all`, `fold`, `sort_by` and the
  rest, dropped as they return, and to a generic function's `F: FnMut`,
  dropped by it (ADR 0470).
- **`ok_or_else`, `or_else` and `then` of a function that holds a value
  with a destructor**, dropped where it's left unrun (ADR 0469).
- **`by_ref().take(n)` of items with a destructor**, in generic code too:
  what it hasn't given is its lender's, stepped and dropped after; and an
  iterator is lent once, `$lent(it).take(2)` (ADR 0468).
- **base64 compiles and runs as native**: text as `AsRef<[u8]>` (ADR 0162);
  what only a type's own `io::Read` calls is never lowered (ADR 0425);
  every trait impl of a library is exported (ADR 0100).
- **A map's and a set's keys with a destructor**, dropped as Rust's are:
  the one `insert` and `entry` are given where an equal's there, the one
  `remove` takes out, and each as the map's cleared or dropped (ADR 0467).
- **A map or a set of a type parameter's keys**, in a generic function, a
  trait's generic method and its default, `HashMap<K, V>` and
  `HashMap<Self::Item, usize>`: of the class its caller's keys say, given as
  `KMap`; and `type_name` of a trait method's own type parameter (ADR 0466).
- **std's lazy adapters of items with a destructor**: `map`, `filter`,
  `zip`, `chain`, `enumerate`, `take_while`, `skip_while`, `step_by`,
  `inspect` and `fuse` drop what they haven't given when they're dropped,
  and each they discard as they discard it; in generic code too, of what
  its caller gives; and `eq` drops each pair it compares (ADR 0465).
- **`max`, `min`, `max_by`, `min_by` and their `_by_key` forms of items
  with a destructor**, each passed over dropped as std's fold drops it; and
  generic code that consumes an iterator of them, `fold`, `last`, `count`
  (ADR 0464).
- **Hashing is std's**: `DefaultHasher` and `SipHasher`, a crate's own
  `Hasher`, derived and hand-written `Hash` impls, and generic code's `T:
  Hash` and `H: Hasher`, each `finish()` as native Rust's for `wasm32`
  (ADR 0463).
- **An `io::Error` is a value**: `io::Error::new(kind, e)`, `other(e)`,
  `kind.into()`, its `kind()`, and its `{}` and `{:?}` as std shows them;
  **an `io::Result<()>` may be an `Err`**, thrown as a `fmt::Result`'s is,
  and a crate's writer may fail (ADR 0462).
- **`rev()` of a lazy chain over a range**, `(1..4).map(f).rev()`, its
  closures called from the back as Rust calls them (ADR 0164).
- **`peekable()` of a lazy iterator**, `chars().filter(f).peekable()`, its
  closures run in Rust's order (ADR 0164).
- **`==` of two maps or two sets**, `$mapEq(a, b)` (ADR 0059).
- **`into_iter()` of a type parameter's associated type**, `H::Item:
  IntoIterator`, itertools' (ADR 0061).
- **A number's `LowerExp` and `UpperExp` called by a type's own**, deranged's
  `{:e}` (ADR 0165); and **`{:.2e}`, an exponent with a precision**, rounded
  half to even as Rust rounds (ADR 0058).
- **A `Formatter` kept in a struct**, base64's `FormatterSink`, that
  generic code writes to: a handle on the writer's text (ADR 0461).
- **An edition 2018 `unreachable!(x)`**, base64's, panics as edition
  2021's does (ADR 0189).
- **heck compiles**: a closure or a callable that writes to the
  `Formatter` it's given is a writer, `f += withWord(word)`, and `{}` of
  `c.to_uppercase()` is `c.toUpperCase()` (ADR 0460).
- **num-traits compiles**: a float's `log(base)`, `sin_cos`, `asinh`,
  `acosh`, `atanh`, `rem_euclid`, `div_euclid`, `abs_sub` and `copysign`;
  and a float's `midpoint` (ADR 0064).
- **rust_decimal compiles**: `splitn` of a set of `char`s or a closure
  (ADR 0157), `Formatter::pad_integral` (ADR 0143), and an impl for
  `&mut Decimal`, whose `&mut self` is a cell of the object (ADR 0454).
- **A hand-written `Serialize`**, its serializer's calls the JSON writer's,
  and **a hand-written `Deserialize`** that reads through another's,
  `String::deserialize(d)?`, or by its own `Visitor`, `deserialize_any` and
  the other hints, its `visit_seq` and `visit_map` too, with serde's
  errors; a field's `#[serde(with = "module")]`, `serialize_with` and
  `deserialize_with`; `collect_seq`, `collect_map`, a variant's tuple and
  struct, and `deserialize_option` (ADR 0459).
- **`?` of an `Ok` made where it's tried**, `Ok(3)?`, is what it holds: no
  test of an object just made (ADR 0459).
- **`hint::assert_unchecked`, `unchecked_add`, `_sub` and `_mul`**, promises
  a right program keeps (ADR 0399), and **`saturating_neg` and
  `saturating_abs`** (ADR 0155).
- **`try_fold`** into an `Option` or a `Result`, stopped at the first
  `None` or `Err` with the rest left to step (ADR 0458).
- **`B::from_iter(items)`** of any collection, as `items.collect()` is:
  `Vec::from_iter(it)`, `HashMap::from_iter(pairs)` (ADR 0160).
- **An `IntoIterator` bound on a reference to a type parameter**, `for<'a>
  &'a mut C: IntoIterator`, as either's `iter_mut` writes it: a dictionary,
  std's or the crate's, so a `&mut C`'s items are written through and a
  crate's collection is taken through a `&C` too (ADR 0457).
- **A `P: DerefMut` in generic code**: a dictionary, `{ Deref, deref_mut
  }`, std's of a `Box`, text, a `Vec`, a `&mut` or a guard (ADR 0394).
- **`as_mut()` of a generic `T: AsMut<str>` or `T: AsMut<[U]>`**: a
  dictionary, std's the text or the array a `&mut` reaches (ADR 0162).
- **`description()` and `cause()` of a generic `E: Error` or a `dyn
  Error`**: each `Error` dictionary holds them, std's the deprecation
  text and the error's `source` (ADR 0141).
- **`len()` of a generic `ExactSizeIterator`**: the JS iterator it's given
  knows its length (ADR 0456).
- **`clone_from`**, std's `*dest = source.clone()`, of any type, a
  generic one's too (ADR 0147).
- **`nth_back(n)`, `rfold(init, f)` and `rfind(p)`** of any double-ended
  iterator, a generic one's too, a map of one too, and `find_map` of items
  with a destructor (ADR 0455).
- **litemap compiles**: a `&mut V` a generic store gives inside what it
  gives, handed on and replaced through (ADRs 0099, 0147), and
  `filter_map` of items with a destructor (ADR 0098).
- **`map(f)` of a `fmt::Result`** (ADR 0187), and **`checked_ilog2()` and
  `checked_ilog10()`** (ADR 0154): writeable compiles.
- **A `&mut` given as a type parameter's value is a box**, as a generic
  `&mut T`'s is: an `impl Trait for &mut T` used through generic code,
  std's `impl Write for &mut W` of any writer, and stable_deref_trait's
  `&mut T`, which compiles (ADR 0454).
- **`rev()` of a chain over a string's `char_indices()`**, writeable's:
  its items listed and reversed, each callback called from the back
  (ADR 0164).
- **A slice's `to_owned()`**, its `to_vec()`; and `to_vec()` of an array
  made where it's called is that array, not a copy of it (ADR 0153).
- **`{:?}` of a slice's iterator**, `Iter([2, 3])`: what's left of it, a
  derived `Debug`'s field too (ADR 0453).
- **`{:?}` of a `PhantomData` of a type parameter**, a generic struct's
  derived `Debug`: `PhantomData<` and the name its caller gives (ADR 0452).
- **std's `Deref` of a guard in a dictionary**, `Ref<T>`'s and `RefMut<T>`'s
  as stable_deref_trait's `StableDeref` takes them: `(x) => x.value`
  (ADR 0451).
- **uuid compiles.** A guard of a lock of a type parameter's,
  `MutexGuard<C>`, and a `Cow<'_, [T]>`, deref in generic code as a known
  type's do (ADR 0450).
- **A lock taken whether or not it's poisoned**,
  `m.lock().unwrap_or_else(|e| e.into_inner())`, uuid's: its guard,
  `$lock(m, true)`; a `PoisonError`'s `into_inner()` and `get_ref()` are
  the guard it holds (ADR 0449).
- **`unwrap_or_else`, `unwrap_or` and `unwrap_or_default` of an `Ok` just
  made** are what it holds, as `unwrap()`'s is, and a closure called where
  it's made is its value, `r._0.data`; and a comparison of two strings
  written out is its answer, as one of two numbers is (ADR 0449).
- **A `transmute` between a `#[repr(transparent)]` struct and its field**,
  by value or through a `&`, as uuid's `from_bytes_ref` does: the struct
  made, `[bytes]`, or its field read, `uuid[0]` (ADR 0448).
- **The clock**: `SystemTime::now()` and `Instant::now()`, the time between
  two, `elapsed()`, a `Duration` added or taken, and a `SystemTimeError`,
  each its nanoseconds, a BigInt, as a `Duration` is (ADR 0447).
- **A `&mut dyn fmt::Write`**, a function that writes to any writer, and
  std's `fmt::write(w, args)`, anyhow's (ADR 0446).
- **Generic code extends and collects into what it's given**: `c.extend(items)`
  of a `C: Extend<A>` and `collect()` into a `B: FromIterator<A>`, through
  their dictionaries; and `s.extend(chars)` of a `String` (ADR 0445).
- **`nth`, `find` and `partition` of items with a destructor**, each it
  passes over dropped, as std's are (ADR 0445).
- **An `Option` compared with a `Some(..)` is `===`**, `p === s`, as a person
  writes it (ADR 0030).
- **The one field a rest's pattern leaves out is `_`**, `const {currentIndex:
  _, ...meta} = ctx`, as a person writes it (ADR 0205).
- **A `.d.ts` has each item's doc comments as JSDoc**, a function's, a
  type's and each field's, which a TypeScript caller's editor shows
  (ADR 0444).
- **A `pub use` of a JS module's binding re-exports it from that module**,
  `export { Challenges } from "./Challenges"`, in the JS and the `.d.ts`
  (ADR 0240).
- **A function's last `return;` isn't written**: it falls off its end, as
  a person ends one, `function half(n) { if (..) { return n / 2; } }`
  (ADR 0299).
- **A `thread_local!`'s memo or context is named as JSX names it**,
  `static IconCopy = memo(..)`, `static TocContext`, with no `#[allow]`, and
  so is the component function written in it; and a component's props
  may be private, as TypeScript's interface needn't be exported (ADR 0439).
- **A `.d.ts` names each type alias the Rust writes**, the crate's own too,
  wherever it's written: `export const TocContext: Context<Toc>;`, where it
  was `Context<TocItem[]>` (ADR 0196).
- **Every comment of the Rust is in the JS**, before the statement its code
  is, or at the end of its line, a `///` doc comment as a JSDoc, which an
  editor shows a JS caller; a branch with one stays a statement, not a
  `c ? a : b`; and a blank line between statements in the Rust is one in
  the JS (ADR 0444).
- **`collect()` into a `Result` or an `Option` of a map, a set, text or a
  collection of the crate's own**, `Result<BTreeMap<String, i64>, String>`
  of parsed pairs (ADR 0036).
- **`Iterator::rposition`** of a slice's items, and of one kept and stepped
  (ADR 0164).
- **A map entry's `and_modify(f)`**, `m.entry(k).and_modify(|n| *n += 1)
  .or_insert(1)`, of a `HashMap` and a `BTreeMap` (ADR 0325).
- **`rev()` of a lazy chain over a slice**, `v.iter().map(f).rev()`,
  `.filter(p).rev()` and `.enumerate().rev()`: `v.toReversed().map(f)`, each
  closure called in the order Rust calls it (ADR 0164).
- **webapi's `localStorage`, `sessionStorage`, `Storage::set_item`, `cookie`,
  IndexedDB's `open`, a canvas's `to_data_url`, `to_blob` and `get_image_data`, a
  stylesheet's rules and `history.push_state` return a `Result`**: each throws
  for what a program can't check first, the page's permissions, a cross-origin
  image or stylesheet, a browser's limit; TodoMVC uses them (ADR 0442).
- **`Default` of `[T; N]`** through its where clause, `[T; N]: Default`, and a
  bound's dictionary named for what it's of, `TArrayDefault`, `WrappingMul` (ADR 0107).
- **A `()` given a function value is no argument**: `resolve()` of `resolve(())`,
  `f()` of a `dyn Fn(())`'s call (ADR 0441).
- **A binding needs no `#[allow]`**: its parameters, which its body never reads,
  and the fields of a struct given one, which JS reads, aren't flagged; the
  playground and examples drop about sixty `#[allow]`s and their stand-in
  bindings of what the crates bind (ADR 0440).
- **A component needs no `#![allow(non_snake_case)]`**: rust-js allows the lint for
  each component it recognizes; the examples and READMEs drop the line (ADR 0439).
- **An optional prop is given `Some(..)`, or left out** (ADR 0438): decided, as
  Rust can't take a value bare and still coerce it; no `{..Default::default()}`
  is needed for `None`s, and the next.js example writes none.
- **`Cell::from_mut`, `as_slice_of_cells` and `as_array_of_cells`** (ADR 0153), and
  a `static` of a `TypeId` (ADR 0331).
- **A scope's drops on a panic are one `try { .. } finally { .. }`**, not a
  `try` and `catch` around each statement: about 6,300 lines fewer across the
  corpus (ADR 0437).
- **A `dyn` of no trait but auto traits**, `Box<dyn Send>`, `Rc<dyn Send + Sync>`,
  its value dropped where Rust drops it; and a `Box` of a struct whose last
  field is a `dyn`, `Box<Fat<dyn Show>>`, which drops that field (ADRs 0409, 0436).
- **A `Box<dyn Any>`, `Box<dyn Display>` or `Box<dyn Error>` of a value with a
  destructor**, dropped where Rust drops it (ADR 0141).
- **`as_mut()` of an `Option` of a generic `T`**, through a `&mut`, and a
  `&mut` to a unit struct in what a call gave (ADRs 0051, 0099).
- **A closure's `TypeId`**, its own, and `assert_eq!` of two function
  pointers or two `TypeId`s: showing one's address or hash is rust-js's error
  where it's shown, not where it's written (ADRs 0331, 0407).
- **`rust-js build` is how an app's Rust is compiled**, a command of its
  own beside `vite` or `next dev`, run once each time: each module's JS and
  `.d.ts` beside its Rust, to commit, and `rust-js build --check`, an app's
  `build` script's first step, fails where what's committed is stale
  ([ADR 0412](docs/decisions/0412-rust-js-build-is-its-own-process.md)).
- **A Rust library is shared on npm, as ReScript's are**: one package of
  its crate and the JS its author's `rust-js build` wrote. A JS app imports
  that JS; a rust-js app names the crate in its `Cargo.toml` and compiles it
  by its own rust-js into the package, wherever npm, bun or pnpm put it,
  and imports it by the package's name, `@acme/greet/src/lib.js`. A
  package's files are replaced, never written through, so a store sharing
  them keeps its own, and `rust-js build --check` is of the app's JS alone,
  a failed build of the app's errors, not a package's warnings. A binding
  crate says it's one, `[package.metadata.rust-js] bindings = true`, and is
  rustc's to check
  ([ADR 0414](docs/decisions/0414-a-library-on-npm-as-rescripts-are.md)).
- **Next.js**: an app's routes and components in Rust, beside its JS, one
  file at a time; the next crate binds all of Next.js, `next/link`,
  `next/router`, `next/image`, `next/font`'s Google Fonts, metadata,
  `getStaticProps` and the rest, typed as Next.js types them, and an app's
  `next.config` may be Rust (ADRs 0192, 0346, 0350, 0354, 0359, 0378).
- **TypeScript declarations**: a `.d.ts` beside each module's JS, printed by
  TypeScript, so the TypeScript of an app moving to Rust is checked against
  it, under any `tsconfig.json` (ADRs 0196, 0207, 0413).
- **The bindings cover what TypeScript types**: `webapi` TypeScript's DOM,
  `js` its ES library, `react` @types/react, `node` @types/node, each
  measured against them, and coverage only grows (ADRs 0281, 0283, 0347,
  0351).
- **rustc's own tests that rust-js compiles and runs as Rust does: 2,062 of
  2,214**, from 1,899 of 2,206. Bodies are lowered from rustc's MIR (ADR
  0364). Among what compiles now: unions; raw pointers, a handle on their
  place; plain data as its bytes, `transmute` and `zeroed` of it; `i128`
  and `u128`; `Duration`; `Wrapping` and `NonZero`; every std data
  structure, or an error that names it, `LinkedList`, `BTreeMap`'s entries,
  `Cow`, `Rc`'s counts, `RefCell`'s borrows; an iterator of the crate's
  given to generic code; a struct of a `dyn` last field; functions that
  never return.
- **JS as a person writes it**: destructuring, `const`, labeled blocks,
  loops, spreads and calls, conditionals for two-arm `match`es, imports in
  one block, laid out by the app's Prettier, a file's leading comments, a
  license's, kept (ADRs 0291, 0278, 0313, 0381, 0383).

- **A `NonNull<T>` is its raw pointer**: `new`, `as_ptr`, `as_ref`,
  `as_mut`, `cast`, `from(&mut x)` and the rest, as a raw pointer's; and
  an array's items are reached as arrayvec reaches them: casts where JS has
  the same place, `ptr::copy`, `slice::from_raw_parts`, and a generic
  constant, `[const { MaybeUninit::uninit() }; CAP]`, as code. A pointer to
  a slice, `*mut [T]` or `NonNull<[T]>`, is the slice; a `&mut` slice's
  `as_mut_ptr()` is no longer refused; and a pointer only read through,
  arrayvec's `ptr::read(elt as *const _)`, is a cell of a value with no
  place
  ([ADR 0407](docs/decisions/0407-a-raw-pointer-is-a-handle-on-its-place.md)).
- **A writer of bytes of the crate's, `impl io::Write`**, where a `W: Write`
  goes or of its own type: its `write_all` and `write_fmt` std's, its
  `write` till all's written (ADR 0392).
- **arrayvec 0.7 runs as natively**, `ArrayVec` and `ArrayString` alike
  (`docs/crate-corpus.md`): a crate built on raw pointers into an array of
  `MaybeUninit`s.
- **`size_hint()` of a generic `I::IntoIter`**, as a type parameter's is
  (ADR 0170).
- **`Option::from(x)`, `Vec::from([..])`, `Rc::from(x)`, `Rc<str>::from(s)`,
  `Arc<[T]>::from(v)` and `Cow::from(x)`**, and the `into()` of each: std's conversions
  are one list of what's converted into what, either way it's asked
  ([ADR 0431](docs/decisions/0431-std-conversions-are-one-list.md)).
- **`take(n)` and `skip(n)` of an iterator that owns values with
  destructors**, `v.into_iter().take(2)`: what it hasn't given is dropped
  when it ends or is dropped, and what `skip` passes over as it's first
  stepped (ADR 0422).
- **A `BinaryHeap` of values with destructors**: `push`, `pop`, `append`,
  `from`, `into_sorted_vec` and `drain` move them and drop none, and
  `retain` drops each it doesn't keep (ADR 0098).
- **`last()` of an iterator of values with destructors**: each before it
  dropped as the next comes (ADR 0098).
- **A vector of values with destructors**: `insert`, `remove`,
  `swap_remove`, `split_off`, `append`, `reverse`, sorts, rotations and
  `mem::take` move its items and drop none; `retain`, `dedup_by`,
  `truncate`, `clear` and `resize_with` drop what they take out, as std
  does, in its order; each was refused (ADR 0098).
- **A map's `retain`, `extend`, `clear`, `entry(k).or_insert(v)` and
  `drain()` of values with destructors**: each drops what it takes out, as
  Rust's does (ADR 0321).
- **`take`, `replace` and `take_if` of an `Option` of a value with a
  destructor**, and a `VecDeque`'s `push_front`, `pop_front`, `insert` and
  `remove` of one: each moves it and drops none; and the `Option`'s
  `insert` and `get_or_insert`, which drop what they don't keep, as Rust's
  do (ADR 0098).
- **`drain(..)` of a vector of values with destructors**: an iterator
  that owns what it took out, whose drop drops what it hasn't given yet,
  as `into_iter()`'s does (ADR 0098).
- **`{:?}` of a `Mutex` and an `RwLock`**, as std's shows them (ADR 0328).
- **`{:?}` of `Bound` and `ControlFlow`**, as a derived `Debug` shows them
  (ADR 0033).
- **Fixed: an `Rc<dyn T>` of a value with a destructor** was a pair around
  the counted `Rc`, which each use misread: it's the counted `Rc` of the
  pair; one made elsewhere and unsized is refused (ADR 0320).
- **An `Rc` or `Arc` of a value with a destructor**, which was refused: it's
  counted, `{ value, strong, weak }`, and the last of it drops what it
  points at, where Rust's does (ADR 0320).
- **Fixed: `fold` of a chain whose stage does something, given a start
  that does too,** ran the stage first; a generated program found it
  (ADR 0139).
- **A blanket impl over every iterator**, `impl<I: Iterator> Trait for
  I`, whose `&mut self` method steps it, through a dictionary and a
  copied default, which was refused (ADR 0411).
- **A trait's C-variadic `&mut self` method as a function pointer**: its
  arrow gathers the rest and passes it on (ADR 0428).
- **`#[track_caller]` through a trait's dictionary**, generic code's or a
  `dyn`'s: each entry of a method one of whose impls reads its caller's
  location takes it, and each call gives it, as rustc's does; an impl
  that tracks its caller where its declaration doesn't is given its own
  definition's through a `dyn`, as rustc's shim gives it (ADR 0427).
- **A B-tree's range of a pair of `Bound`s**, `m.range((Excluded(&2),
  Unbounded))`: `$treeRange` takes each end's `Bound`, of any range alike
  (ADR 0325).
- **`a.mul_add(b, c)`**, of `f64`s and `f32`s: `$mulAdd(a, b, c)`, `a * b
  + c` rounded once, exactly, as a fused multiply-add is
  ([ADR 0430](docs/decisions/0430-mul-add-rounds-once.md)).
- **A trait's default method whose `where` clauses an impl can't meet**,
  `where Self::Item: Deref`, is left out of that impl's dictionary, as
  rustc leaves it out of a vtable; one that meets them derefs through its
  dictionary (ADR 0049).
- **`next_back()` and `rev()` of a generic `T: DoubleEndedIterator`, and
  of a `dyn DoubleEndedIterator`**: a JS iterator that knows its back,
  an array's `$iter`, or `$iterator(it, next, false, nextBack)` of a range
  or of the crate's own
  ([ADR 0429](docs/decisions/0429-a-double-ended-iterator-knows-its-back.md)).
- **`type_name` in generic code** of a type that holds a type parameter,
  `Cow<'_, T>`, and of a `T: ?Sized` (ADR 0145).
- **C-variadic functions**, `extern "C" fn f(n: i32, mut args: ...)`: a
  rest parameter, `...args`, `next_arg()` its `shift()`
  ([ADR 0428](docs/decisions/0428-c-variadic-functions.md)).
- **Fixed: a trait's `&mut self` method as a function pointer** gave the
  method the object unboxed, and crashed (ADR 0099).
- **`#[track_caller]` and `Location::caller()`**: a function that reads
  where it was called takes it, `{ file, line, column }`, and each call
  gives it; one that reads nothing of it, an `Index` of the crate's among
  them, is as it was
  ([ADR 0427](docs/decisions/0427-track-caller.md)).
- **`env::var_os(key)` and `env::vars_os()`**, in Node, an `OsString`
  being its text (ADR 0391).
- **A `dyn ToString`**, a value and its `to_string`, as a `dyn Display` is
  (ADR 0141).
- **A constant of `iter::Empty`**, an iterator over an empty array, as
  `iter::empty()` is (ADR 0128).
- **A channel of values with a destructor**, a channel of senders among
  them: what's still queued is dropped with the receiver, as Rust's is
  (ADR 0142).
- **A struct of no fields, and `()`, are ordered**, `Equal` to themselves,
  as their derives have it: a `BTreeMap` keyed by one (ADR 0057).
- **A `BinaryHeap`'s and a `VecDeque`'s capacity** is its array's, as a
  `Vec`'s is, and a heap
  of `()` is an error only where it's popped (ADRs 0419, 0068).
- **A test or value of literals is its answer**: `0 < 0` is `false`,
  `true ? a : b` is `a`, and a template's `${false}` its text
  ([ADR 0426](docs/decisions/0426-a-literal-test-is-its-answer.md)).
- **`it.cycle()` of std's iterator over an array**: `$cycle(items)`, a
  generator over what it has left, again and again (ADR 0181).
- **`convert::identity(x)`** is `x`.
- **Fixed: rustc took a program's panics to abort, wasm32's, where
  rust-js's unwind**: a future's size, `size_of_val`, is native Rust's now,
  and `cfg(panic = "unwind")` holds
  ([ADR 0090](docs/decisions/0090-wasm32-front-end.md)).
- **A type's own `impl Future`, `io::Read`, `BufRead` and `Seek`**, as
  either's `Either` has, are allowed and never lowered, as its `Hash` is:
  what would call one is refused where it's called, and so is what would
  take a future of the crate's own for a JS promise, its `.await` among them
  ([ADR 0425](docs/decisions/0425-a-crates-own-future-and-reader-are-allowed-never-lowered.md)).
- **indexmap's `IndexMap` and `IndexSet` are a `Map` and a `Set`**, which
  keep the order they're added in, as indexmap's do; `get_index`,
  `swap_remove`, `insert_full` and the rest are the runtime's
  ([ADR 0424](docs/decisions/0424-indexmaps-maps-are-js-maps.md)).
- **hashbrown's `HashMap` and `HashSet` are std's**, a `Map` and a `Set`:
  rustc builds hashbrown, never rust-js, and what std's maps haven't is
  refused where it's called
  ([ADR 0423](docs/decisions/0423-hashbrowns-maps-are-stds.md)).
- **`array::from_fn(f)`**: `Array.from({ length: N }, (_, i) => ..)`
  (ADR 0324).
- **`Layout::new::<T>()`**, of a type parameter too, its size and alignment
  given by its caller, and `Layout::from_size_align_unchecked` (ADR 0417).
- **Fixed: std's functions named through a `no_std` crate's own name for
  `alloc`**, hashbrown's `extern crate alloc as stdalloc`, weren't known as
  std's, `Vec::capacity`, `alloc::alloc` and the rest (ADR 0417).
- **`[const { MaybeUninit::uninit() }; N]` of a type that isn't `Copy`**:
  `Array.from({ length: N })`, a constant with no byte written being
  `undefined` where it's used (ADRs 0127, 0332).
- **smallvec 1.16 runs as natively** (`docs/crate-corpus.md`): its
  `SmallVec` inline and spilled to the heap, and what it drops, and when.
- **`iter::repeat(value)` of a value with a destructor**, and `take` and
  `skip` of it: `value` is dropped when the iterator is closed, JS's
  `return()`; and `v.extend(items)` of items with a destructor
  ([ADR 0422](docs/decisions/0422-an-iterator-holding-a-value-is-closed-to-drop-it.md)).
- **Generic code given a `T` with a destructor** that swaps its items, lends
  one, or extends from a slice, or `set_len`s its vector: it drops none,
  so it takes one (ADR 0422).
- **`MaybeUninit::<A>::uninit()` of a type parameter that's an array**,
  smallvec's `A: Array`: an array of slots, written an item at a time
  (ADR 0417).
- **A std trait's associated type dropped in generic code**, an
  `I: IntoIterator`'s `I::IntoIter`, an `I::Item`: its caller gives its
  drop, `dropIIntoIter`, as it gives a type parameter's
  ([ADR 0421](docs/decisions/0421-std-associated-type-drops-are-given.md)).
- **Fixed: an iterator of the crate's with a `Drop`, given where a generic
  iterator goes**, was dropped as the JS iterator made of it, which its
  `drop` read as its own (ADR 0421).
- **`Vec::set_len`**: what it shortens a vector by stays where it was, read
  through a pointer as smallvec's `from_vec` does, and what it lengthens it
  over is what was written there
  ([ADR 0420](docs/decisions/0420-what-set-len-leaves-stays.md)).
- **A `Vec`'s `capacity()`**, kept where the crate or a library it uses
  asks it, as std promises it
  ([ADR 0419](docs/decisions/0419-a-vecs-capacity-kept-where-asked.md)).
- **`v[i]` of an index of a type parameter's**, `I: SliceIndex<[T]>`, an
  `Index<I>` of the crate's: its dictionary's, the runtime's for `usize` and
  each kind of range
  ([ADR 0418](docs/decisions/0418-a-generic-slice-index-is-a-dictionary.md)).
- **An iterator whose `Item` is another trait's associated type**,
  `A::Item`, looped over by `&mut` (ADR 0364).
- **`==` of two types' items**, `[A::Item] == [B::Item]` by `A::Item:
  PartialEq<B::Item>` (ADR 0053), and `clone_from_slice` of an associated
  type's items (ADR 0324).
- **A `MaybeUninit`'s `as_ptr()` and `as_mut_ptr()`**, of a type parameter's
  too (ADR 0332).
- **The allocator**: `alloc`, `realloc`, `dealloc`, `handle_alloc_error`,
  `Layout`, a pointer cast to bytes and back, and `Vec::from_raw_parts`. An
  allocation is a JS array of slots
  ([ADR 0417](docs/decisions/0417-an-allocation-is-an-array-of-slots.md)).
- **`size_of::<A::Item>()` in generic code**, of an associated type of the
  crate's trait: its dictionary's, `AArray.ItemSize` (ADR 0145).
- **`p.offset_from(q)`** of two pointers into one array (ADR 0407), and
  **`next_power_of_two()`** and its checked one (ADR 0064): hashbrown's.
- **An iterator of the crate's with a destructor, consumed**, `collect()`,
  dropped after, as Rust's method drops it (ADR 0098); and
  `ptr::drop_in_place` of a slice drops each item (ADR 0407).
- **A `str`'s `as_ptr()`, only read through**, a copy's source: a pointer to
  its UTF-8 bytes, as arrayvec's `ArrayString` copies them in (ADR 0407).
- **`as_ref()` of text as a `Path`**, `s.as_str().as_ref()` in an
  `AsRef<Path>` impl: the text itself (ADR 0173).
- **A zeroed `[MaybeUninit<u8>; CAP]`**, of a length generic code is given,
  `new Array(CAP).fill(0)` (ADR 0332).
- **`map_err` of a `fmt::Result`**, `write_fmt(args).map_err(..)?` of a
  writer that may fail (ADR 0187).
- **A trait's default method calling its `Self`'s**, where `Self` is
  generic, `Holder<T, CAP>`: given the impl's `CAP`, and another default
  of it found in the impl's dictionary (ADR 0135).
- **`clone_from_slice` in generic code** calls each item's own
  `clone_from`, where its impl writes one, through its `Clone` dictionary,
  as std's does (ADR 0324).
- **A `&mut &mut u32`**, a `&mut` to a `&mut` of a number or a string, is a
  cell of the cell, as arrayvec's `ScopeExitGuard` holds one
  ([ADR 0338](docs/decisions/0338-slice-split-off.md)).
- **A generic function or impl that asks `size_of::<T>()` is given it** in a
  trait impl's method, through the impl's dictionary, called through its
  trait from another impl's, or given that dictionary for a bound or
  `collect()`, and across crates, by
  its library's manifest ([ADR 0145](docs/decisions/0145-type-facts.md)).
- **A range's bounds in generic code**, `R: RangeBounds<usize>`'s
  `start_bound()` and `end_bound()`, of every range of std's
  ([ADR 0416](docs/decisions/0416-range-bounds-in-generic-code.md)).
- **A crate of crates.io's is compiled into the app**, where rust-js can
  compile it: its JS committed in `rust-js/crates/<name>-<version>/`,
  imported from there, and checked by `rust-js build --check`. One it
  can't is its types alone, as before, and a call into it says why
  ([ADR 0415](docs/decisions/0415-a-crate-of-crates-io-in-an-app.md)).

### Changed

- **`rev()` of what knows its back is `$rev(it)`**, not `$iterator(it,
  $nextBackSome, true)` (ADR 0516).
- **No list with holes**: `println!("{:?} {:?}", f(a), g(b))` reads `g(b)`
  as itself, not `[undefined, g(b)][1]` (ADR 0512).
- **No `.map((item) => item)`**: `{:?}` of a list of unit enums or of
  numbers is `` `[${v.join(", ")}]` ``, and `cloned()` of strings is the
  strings (ADR 0508).
- **Nested loops read as written**: a `let` in an inner loop is declared
  there, `const score = ..`, not before the outer one; an inner loop's
  way out that ends the outer turn is its `break`, not `continue bb4`; and
  a labeled block's jump to the jump after it is that one (ADR 0364).
- **A callback is given the index, one function for each JS method**: a typed
  array's `map(|x, i| ..)`, `filter`, `find`, `for_each`, `every`, `some` and
  `reduce`, and React's `Children::for_each(children, |child, i| ..)`, each
  as TypeScript declares it; `a.map(|x, _| x * 2)` is `a.map((x) => x * 2)`.
  The `_with_index` forms are gone (ADR 0443).
- **A binding's optional parameter is an `Option`**, one Rust function for
  each JS one, as TypeScript declares it: `history.push_state(data, "",
  None)` is `history.pushState(data, "")`, and `push_state_with_url`, like each
  `_with_` form of an optional argument, is gone: webapi's, `EventTargetExt`'s
  `add_event_listener_with_options`, react's `memo_with`, `create_root_with` and
  `render_to_string_with`, next's `push_with_options` and `json_with_init`,
  node's `slice_with_start`, `alloc_with_fill` and `exit_with_code`, and the
  rest. A union's or an `any`'s is borrowed, `Some(&blob)`, a callback's boxed,
  `Some(Box::new(|a, b| ..))`; a trailing `None` is no argument, and a Rust
  function's trailing `Option`s are declared optional, `greeting?:`. The js
  crate's `slice_to_end`, `index_of_from`, `fill_range`, `sort_by` and Date's
  `new_with_ymd`, react's `use_effect_on_every_render` and webapi's
  `alert_with_message` are their one function's too: what stays apart differs
  in a parameter or its result, `Buffer::from_with_str(s, None)`, and a test
  holds it (ADR 0443).
- **Rust's world and React's each have their own names, JSX the boundary**
  ([ADR 0435](docs/decisions/0435-rust-and-react-two-worlds.md)). In JS, a
  component's props are camelCase, `onClick` of `on_click`, and a hook, a
  `use_x` that calls one, is `useX`; everything else keeps its Rust name.
  `jsx!` takes a prop only by its JSX name: `on_click=` is an error that
  says to write `onClick`. `js::camel_case!` is gone, a compile error.
- **`@rust-js/vite-plugin`, its `rust-js:<package>` imports, `rust-js-patch`
  and its `postinstall`** are gone: `rust-js build` writes the patch Cargo
  reads, and an app imports the JS beside its Rust (ADR 0412).
- **A module's header names its Rust from its crate's directory**,
  `src/api.rs`, whichever build wrote it, not `frontend/src/api.rs` from
  the workspace's (ADR 0414).
- **`react::Ref<T>` is @types/react's `Ref<T>`**, a ref a prop holds, an
  object or a callback, and `react::DetailedHTMLProps<E, T>` and
  `react::SVGProps<T>` an element's props, its `ref` too, which a
  component's props flatten. The trait of what JSX's `ref` takes is
  `IntoRef` ([ADR 0384](docs/decisions/0384-a-ref-prop-is-reacts-ref.md)).
- **`[package.metadata.rust-js.format]` is gone**: the app's Prettier lays
  out what rust-js writes; move its options to `.prettierrc`
  ([ADR 0381](docs/decisions/0381-the-apps-prettier-lays-it-out.md)).
- **A temporary of a variable is the variable** where nothing sets it
  between, `args.push(x)` where it was `const value = args; value.push(x)`
  ([ADR 0434](docs/decisions/0434-a-temporary-of-a-variable-is-the-variable.md)).
- **What's written after a loop is after it**: what MIR moved into a
  loop's one way out is a `break` there and the statements after the loop,
  and a `while let`'s exit is its first test, `if (c == null || ..) {
  break; }` ([ADR 0433](docs/decisions/0433-whats-written-after-a-loop-is-after-it.md)).
- **A `return` is written where its value is made**: a `?` or a `return
  Err(..)` inside a loop or a `match`, which went through a variable, a
  labeled block and a `return` after it, is `return v;` where it's made,
  and each `?` a guard, `if (r.TAG === "Err") { return r; }`
  ([ADR 0432](docs/decisions/0432-a-return-is-written-where-its-value-is-made.md)).
- **`v.some(f) && v.every(g)` has no `!!`**: a call of JS's `some`,
  `every`, `includes`, `has`, `startsWith` or `endsWith` is a `bool` already
  (ADR 0235).

### Fixed

- **A function named `new`, `delete` or another word JS keeps** was
  declared by that name, which TypeScript can't read; it's its JS's, `new$`
  (ADR 0546).
- **rust-js crashed** on a trait's default with parameters of its own,
  `collect_array<const N: usize>`, kept by an impl whose parameters are
  numbered apart from its trait's (ADR 0540).
- **rust-js crashed** on `{}` of a value holding a `dyn for<'a> FnMut(&'a
  dyn Display)` (ADR 0539).
- **A range mapped by a function it's given, folded from its back**,
  `(a..b).map(f).rfold(..)`, gave nothing (ADR 0535).
- **`extend`, `chain` and `zip` of an `Option` or a range**: a `Some` was
  dropped, and a range given to `chain` or `zip` threw (ADR 0528).
- **`scan` after a lazy chain** ran the chain's closures for every item,
  past where `scan` ended: it's lazy, as Rust's is (ADR 0517).
- **`filter_map` and `flat_map` of what holds what looks like `None`**,
  `Some(None)`'s or a type parameter's, kept their `Some`s boxed: `Some([object
  Object])` (ADR 0515).
- **An adapter a generic function makes of a type parameter given as it
  is**, closed, dropped the JS iterator stepping it, not the value (ADR
  0514).
- **A type parameter bound by the crate's trait that extends `Iterator`**,
  `T: Sorted`, of the crate's iterator, was stepped as its fields: it's
  given as it is, as its trait's impl takes it (ADR 0511).
- **A `&mut dyn` of a trait that extends `Iterator`** gave its method the
  pair, which it stepped as the value: it's given the value (ADR 0511).
- **`flat_map` of a function that gives an `Option`** threw, `flatMap
  called on non-object`: it's the `Some`s (ADR 0510).
- **`flat_map` whose inner iterators own what has a destructor**, stopped
  early, never dropped what they hadn't given (ADR 0510).
- **`extend_from_within` of objects** pushed the same objects again, so a
  change to one changed both: it pushes clones (ADR 0507).
- **`fill` of a type with its own `clone_from`** cloned each: it calls
  `clone_from`, as std's does (ADR 0507).
- **`rc.borrow()` of a counted `Rc`** gave the `Rc`, shown `[object
  Object]`: it's what it points at (ADR 0503).
- **`xor` of two `Some`s whose first value's destructor panicked** never
  dropped the second: it's dropped in a `finally` (ADR 0502).
- **A trait's default that calls its impl's own method, or another default,
  of an impl for a generic type of the crate's**, gave it its `Self`'s drop
  where its type parameter's goes, and threw: it's given the impl's (ADR
  0495).
- **A trait's default that steps its `Self`, kept by a blanket impl or an
  impl for the crate's iterator**, stepped it as a JS iterator and threw;
  it's stepped as the impl's own methods step it (ADR 0492).
- **A `Vec`'s iterator a generic function made and gave back**, its
  `len()`, clone and `size_hint()`, read `NaN` or threw: generic code's
  `into_iter()` is the iterator concrete code has (ADR 0475).
- **The crate's iterator given to a generic `I: Iterator + Clone` or
  `Debug`** was cloned and shown as its JS iterator: it's given as it is
  (ADR 0474).
- **`fuse()` of an iterator that starts again after its `None`** stepped it
  again: it's std's `Fuse`, ended for good; and a crate iterator with a
  `Drop` under an adapter, `map` or `fuse`, is dropped as std drops it
  (ADR 0473).
- **`into_iter()` of an `Option` of another iterator's item, collected**,
  `option::IntoIter<I::Item>`, threw: it's made a JS iterator (ADR 0472).
- **A function an `Option`'s or a `Result`'s method leaves unrun is
  dropped**, with what it holds, as std drops it: `map`'s of a `None`,
  `unwrap_or_else`'s of a `Some`, `entry().or_insert_with`'s of a key that's
  there, in generic code too (ADR 0469).
- **A library's trait default that gives `Self::Item` to its own generic
  method** read a drop of a dictionary entry that isn't there, and threw as
  it ran (ADR 0466).
- **`max_by(compare)` and `min_by(compare)`** called `compare(item, kept)`,
  where std calls `compare(kept, item)`, and **`max_by_key(key)`** made each
  item's key again for each comparison, where std makes it once: a
  comparison or a key that prints or counts showed it (ADR 0464).
- **A hand-written serde impl** was written twice, once as if derived: a
  `Deserialize` read the wrong shape, or called a function that isn't there
  (ADR 0459).
- **A type of a const parameter, `Buffer<SIZE>`, given to generic code or
  in a trait's default its impl copies** compiles: rust-js crashed (ADR 0107).
- **A value read in a branch, then a drop after it**, a map's `len()` in a
  loop and a value dropped after the loop, ran: JS threw a `ReferenceError`
  (ADR 0451).
- **A trait method's `&mut Self::Out` of an object, called through a
  dictionary**, `g.get().push(1)` of a `G: Get<Out = Vec<u32>>`, is the
  impl's `Vec`: it threw (ADR 0454).
- **An iterator a variable holds, given to generic code as a
  `DoubleEndedIterator`**, `last(it)`, is that iterator: it was wrapped
  again and stepped nothing (ADR 0456).
- **`f(&mut v)` of a `f<D: Trait>` where `Trait` is implemented for
  `&mut T`** reads `v`: it threw (ADR 0454).
- **A clone of a struct whose text or number is changed through a `&mut`
  of it**, `copy[0].0.push('?')`, is a copy: it shared the original's
  parts, which changed with it (ADR 0052).
- **A guard given to generic code is dropped there**, `take(cell.borrow())`:
  its cell stayed borrowed, and a later `borrow_mut()` panicked (ADR 0451).
- **`d.len()` of a `D: Deref` returned is read before `d` is dropped**:
  read after, it saw what `d`'s destructor changed (ADR 0451).
- **A `RefCell<dyn T>` given to generic code as a cell of a type
  parameter is a compile error**: it threw (ADR 0451).
- **A `Duration` assigned through a `&mut`**, `*d = *d + one`, is the
  caller's: it was written to fields a number hasn't, and lost; and a
  `Duration`'s `-=` past zero panics as its `-` does (ADR 0447).
- **`o == Some(())`**, of an `Option` whose `Some` is boxed, compares what's
  in it: it was never `true` (ADR 0051).
- **`fold(None, f)`** kept its start, `reduce(f, undefined)`: it was left
  out, so an empty slice's fold threw and another's started from its first
  item (ADR 0330).
- **A shared `Rc<RefCell<dyn T>>`**, a listener a bus and its owner both
  hold, compiles beside a `Weak` elsewhere, and shares its counts with the
  `Rc` it's made of (ADR 0320).
- **A `let .. else` is a guard first**, `if (!isValidElement(child)) { ..;
  return; }`, after another one too, or where its `else` does more than
  return (ADR 0432).
- **A zero divisor panics before what's evaluated after the check**:
  `v[f()] = a % b` checked `b` only once it divided, after `f()` ran, which
  native Rust doesn't run (ADR 0372; a generated program, seed 12445863).
- **A `Mutex` or an `RwLock` holding a value with a destructor** no longer
  crashes the compiler: it drops what it holds as a `RefCell` does, and its
  `into_inner` gives the value back (ADR 0144).
- **rustc no longer crashes** where a generic function calls another whose
  parameter is a projection, `<T as Deref>::Target`: whether the callee's
  value is boxed is asked in the callee's environment, not its caller's.

- **A declaration means the same under every `tsconfig.json`.** A field a
  caller may leave out is `?: T | undefined`, as @types/react writes one,
  where `?: T` refused `undefined` under `exactOptionalPropertyTypes`,
  which Rust's `Option` takes as `None`. The declarations are checked under
  `bundler`, `nodenext` and the strictest settings, Vite's template's
  `erasableSyntaxOnly` among them
  ([ADR 0413](docs/decisions/0413-a-declaration-means-the-same-under-every-tsconfig.md)).
- **A `match` of a call's value calls it once.** With three arms or more,
  `match note(x) { 1 => .., 2 => .., _ => .. }` called `note` once per arm
  it tested. Found by a generated program
  ([ADR 0364](docs/decisions/0364-mir-lowering.md)).
- **A loop left by a `break` of a choice compiles.** `loop { if a || b {
  break if c { 1 } else { 2 }; } }` never finished compiling: rust-js turned
  its two exits around, and back, without end. Found by a generated program
  ([ADR 0370](docs/decisions/0370-loop-exits-first.md)).
- **A private props struct an exported component names is declared in its
  `.d.ts`**, not exported, where it was `any`
  ([ADR 0385](docs/decisions/0385-a-private-type-a-declaration-names.md)).
- **A crate root of `mod`s alone writes no JS**, an app's list of its files;
  and a module of a `js::import!` alone writes its import, which was lost
  ([ADR 0382](docs/decisions/0382-a-module-has-a-file-for-what-it-has.md)).
- **A `&mut` of what a generic `Option`'s `Some` holds compiles**, `Some(x)`
  of a `&mut Option<T>`, read, written or read in a guard, as chrono's
  `set_if_consistent` does; **another crate's generic writer is given a
  `Formatter`**, bitflags' `to_writer`; and **an enum of no variants is
  declared `never`**, where its `.d.ts` didn't parse. chrono compiles and
  bitflags runs again ([ADRs 0051](docs/decisions/0051-generic-options.md),
  [0180](docs/decisions/0180-generic-writers.md)).
- **Switching an app's rust-js, up or down, builds its crates again.** Cargo
  had the crates another rust-js built as done, as an installed compiler's
  file is as old as its package says, and the build stopped at
  `incompatible dependency compiler identity`. Now `rustc -vV`, through
  rust-js, says which rust-js it is, and Cargo keeps each one's build apart
  ([ADR 0101](docs/decisions/0101-cargo-workspace-wrapper.md)).

### May now be refused

What 0.0.3 compiled wrongly, or not at all, is now right or an error that
names it:

- Memory addressed by number, a pointer cast to an integer or read at an
  offset, is an error that says so (ADR 0390).
- A crate's `[package.metadata.rust-js]` `declarations` key or `format`
  table is an error that says what replaces it.
- An npm package with a crate installed under an alias of another name,
  `"x": "npm:@acme/greet"`, or whose `exports` hide the JS an app imports,
  is an error that says what to do (ADR 0414).

### Upgrade from 0.0.3

1. Install Rust 1.99.0, which rust-js 0.0.4 runs with:
   `rustup toolchain install 1.99.0 --profile minimal --target wasm32-unknown-unknown`.
2. In `package.json`, set `@rust-js/runtime` and `@rust-js/react` to `0.0.4`
   in `dependencies`, `@rust-js/builtins` to `0.0.2` and `@rust-js/webapi`
   to `0.0.3`, and `@rust-js/native` and `@rust-js/build` to `0.0.4` in
   `devDependencies`. Remove `@rust-js/vite-plugin`, and the `postinstall`,
   `rust-js-patch`. Make the `build` script `rust-js build --check && vite
   build`.
3. In `vite.config.js`, remove `rustJs(..)` and its import: Vite serves the
   JS beside your Rust as any JS.
4. In `Cargo.toml`, name the crates' releases:
   `react = { package = "rust-js-react", version = "~0.0.4" }`, and
   `~0.0.2` and `~0.0.3` for `js` and `webapi`. Remove a `declarations`
   key: a `.d.ts` is always written.
5. `bun install` (or npm's, pnpm's), then `bunx rust-js build`: the JS and
   `.d.ts` beside your Rust are 0.0.4's. Commit them with the upgrade.
6. Remove `js::camel_case!();` if your crate has it, and write each `jsx!`
   prop by its JSX name, `onClick={..}` not `on_click={..}`: the compiler
   names each one to change. A JS caller of your component gives its props
   camelCase, `Button({ onClick })`; one of a function or a data field that
   `js::camel_case!` named the JS way uses its Rust name now.
7. Call each binding's one function where you called a `_with_` form of an
   optional argument, its optional arguments `Option`s: `push_state(data,
   "", Some(url))` for `push_state_with_url(data, "", url)`,
   `use_effect(f, Some(&[n]))` for `use_effect(f, [n])`, and a `None` for
   each one you leave out, `fetch(url, None)`. rustc names each call, and
   the argument it misses.
8. From now on, run `rust-js build` after each change to your Rust, beside
   `bun run dev`.

### Roll back to 0.0.3

Revert the upgrade's commit, its `package.json`, `vite.config.js`,
`Cargo.toml` and the JS, then `bun install`, and `cargo clean --target
wasm32-unknown-unknown`: Cargo keeps each rust-js's build apart from 0.0.4
on, but not 0.0.3's.

## 0.0.3

For macOS on Apple silicon, with Rust 1.98.1, as 0.0.2. `@rust-js/builtins`
stays 0.0.1 and `@rust-js/webapi` 0.0.2.

### New

rustc's own tests that rust-js compiles and runs as Rust does: 1,899 of
2,206, from 1,583. Among what compiles now:

- **Text by its bytes:** a string's `len()`, slices and `find` count UTF-8
  bytes, as Rust's do; `s.bytes()`, `as_bytes()` and byte strings.
- **Numbers:** `f32`, each result rounded as Rust rounds it; bitwise
  operators in generic code and a type's own, as a set of flags has them.
- **Patterns:** slice and array patterns, bindings inside `|`, `if let` as a
  value anywhere, if-let guards, inline `const` blocks.
- **Formatting:** `{:#?}` and the Debug builders; a placeholder's width,
  precision and sign reach a generic or `dyn` value's `fmt`, which can ask
  its `Formatter` for them.
- **Traits and generics:** const generics of traits, a `dyn Display` or
  `dyn Error` as a value, `Box<dyn Error>` from `?`, generic associated
  types of lifetimes, `size_of` and `type_name` of a type parameter.
- **Iterators:** std's sources, lazy `chain`, `zip`, `take_while`,
  `skip_while`, `fuse`; ranges as values; iterator trait objects; a generic
  iterator stepped through or lent as a `&mut`; an iterator's `len()`;
  `collect()` into a `Result` or an `Option`.
- **Shared state:** `Mutex`, `RwLock` and `Arc` on one thread, channels on
  one thread, a `HashMap` or `HashSet` keyed by a struct.
- **Destructors** run where Rust runs them for closures, loops, trait
  objects and temporaries too.

### Fixed: programs that gave a wrong answer

Each of these compiled in 0.0.2 and ran differently than Rust. A program that
did any of them prints Rust's answer now:

- **A function written in a trait's default**, of an impl in another
  module, crashed the compiler, its copy calling what wasn't there (ADR
  0308).
- **Two structs that each held a map or a set**, of a derived `PartialEq`,
  were equal whatever their maps held (ADR 0059).
- **A struct of a const parameter with a destructor**, itertools'
  `ArrayBuilder<T, N>`, in a crate that borrows a `RefCell` field, crashed
  the compiler (ADR 0362).
- **A `fmt::Result` through generic code**, an impl whose `Error` is
  `fmt::Error` called through `Result<(), S::Error>`, failed reading its
  `TAG`; and **a writer's last call that failed**, `check(n)` of a
  `fmt::Result`, gave `Ok` (ADR 0461).
- **A library's `Display` shown with a width**, `{:>12}` of heck's
  `AsKebabCase`, was given its consumer's options where its dictionaries
  go, and failed: a writer takes options as its own crate does, which its
  manifest says (ADR 0460).
- **A `&mut` of a local a function keeps in another parameter**,
  `keep<'a>(v: &mut Vec<&'a mut u32>, x: &'a mut u32)`, or in `self`, is a
  handle on the local: a box of it was copied back when the call returned,
  and the writes through it after were lost (ADR 0099).
- **A reborrow of a reference reassigned before it's used**, `add(&mut *x,
  { x = &mut q; 10 })`, wrote what `x` referred to after, not before
  (ADR 0099).
- `collect::<Result<Vec<_>, _>>()` and `collect::<Option<Vec<_>>>()` gave the
  array of `Result`s or `Option`s, not the first `Err` or `None`.
- `println!("{} {:?}", v.len(), v.pop())` read `v.len()` after the pop.
- `next()` of an iterator chain could take more than one item, and a chain
  whose closures do what can be seen, kept in a variable or given to `zip`
  or `chain`, could run them in another order than Rust's.
- A width or a sign for a generic or `dyn` value was dropped: it reaches the
  value's `fmt` now.

### May now be refused

What 0.0.2 compiled wrongly is now either right or an error that names it.
A program that relied on one of these stops at the error, where it ran
wrongly before: a width or a sign for a `&dyn Debug` made elsewhere, or for
serde_json's types, whose `fmt`s are rust-js's own.

### Upgrade from 0.0.2

1. In `package.json`, set each package released with the compiler to
   `0.0.3`: `@rust-js/runtime` and `@rust-js/react` in `dependencies`,
   `@rust-js/native`, `@rust-js/build` and `@rust-js/vite-plugin` in
   `devDependencies`. Leave `@rust-js/builtins` and `@rust-js/webapi`.
2. In `Cargo.toml`, name the react crate's release:
   `react = { package = "rust-js-react", version = "~0.0.3" }`.
3. `bun install` (or npm's, pnpm's): its `postinstall`, `rust-js-patch`,
   points Cargo at the new crates in `node_modules`.
4. `bun run dev` or `bun run build`: rust-js writes the JS beside your Rust
   again. Commit it with the upgrade: its diff is what the new compiler
   writes differently.

If the build stops at `incompatible dependency compiler identity`, Cargo
kept crates the other rust-js built: `cargo clean --target
wasm32-unknown-unknown`, then build again. 0.0.3 and 0.0.2 don't tell Cargo
which rust-js built a crate; the next release does.

### Roll back to 0.0.2

The same steps, with `0.0.2` and `~0.0.2`: `bun install` again, then
`cargo clean --target wasm32-unknown-unknown`, which this direction needs,
then build, and the JS beside your Rust is 0.0.2's again (`git checkout`
of it does the same).

## 0.0.2

An app is a Cargo package with its crates from npm, `@rust-js/react` among
them, first released here; its patch is written as it's installed and as
Vite starts, and a Cargo build runs the compiler's Rust. A release build
has no path of the building machine's home in it.

## 0.0.1

The first release on npm, for macOS on Apple silicon: `bun create @rust-js`
makes a Vite and React app written in Rust, with `@rust-js/builtins` and
`@rust-js/webapi`.
