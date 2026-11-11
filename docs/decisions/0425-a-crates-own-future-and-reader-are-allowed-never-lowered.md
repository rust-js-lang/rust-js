# 0425. A type's own `Future`, `io::Read`, `BufRead` and `Seek` are allowed, and never lowered

Status: Accepted. Extends [0168](0168-user-hash.md), whose `Hash` impls
are allowed and never lowered, and [0029](0029-async-await.md), whose
futures are JS promises. Amended by
[0463](0463-hashing-is-stds.md): `Hash` is lowered now, and no longer
`unlowered_trait`'s; these four still are. Amended (#1156): std's byte
sources' `Read`, `BufRead` and `Seek` are lowered, a crate's own still
isn't (#1297).

Case: A, D ([0262](0262-when-rust-and-js-disagree.md)): an impl nothing
calls is no JS a program can see; what would call one, or await one as a
promise, is a compile error.

## Context

either's `Either<L, R>` forwards std's traits to the side it holds: among
them `Future`, `io::Read`, `BufRead` and `Seek`.

```rust
impl<L: Future, R: Future<Output = L::Output>> Future for Either<L, R> {
    type Output = L::Output;
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> { .. }
}
```

Each was an error, "user implementations of `Future`", at the impl. That
stopped either whole, so itertools as well, though a program that uses
`Either` as a value, `is_left()` or `map_left`, never calls them.

## Decision

**A type's own `impl Future`, `impl io::Read`, `impl BufRead` and `impl
Seek` are allowed and, like its `Hash` (ADR 0168), never lowered:** their
methods are no JS. Which std traits are like this is
`recognition::unlowered_trait`'s one list. `implementable` allows an impl
of one, and `is_unlowered_impl` keeps its methods out of what's lowered.

- **What would call one is refused where it's called.** A call of `read`,
  `fill_buf`, `seek` or what std provides of them, `read_to_string` or
  `bytes`, is "calling `std::io::Read::read`", as it was. So is a `dyn
  Read`, and a `BufReader` of one. JS has no stream these would be.
- **A future of the crate's own is no JS promise, so nothing may treat
  it as one.** In JS, `.await` is `await` (ADR 0029). Of a value that
  isn't a promise, `await` gives the value itself: `S(1).await` would be
  `await [1]`, which is `[1]` where Rust gives `1`. So
  `recognition::unlowered_future` says which types are such a future:
  ones whose `Future` resolves to an impl rust-js compiled, the crate's or
  a library's, seen through a reference, a `Box` or a `Pin`. Each of these
  is an error at its span:
  - a call whose bounds ask a `Future` or an `IntoFuture` of one
    (`awaited_unlowered_future`): `.await` of it, `into_future`'s, and a
    generic function's `F: Future`, a binding's (`js::promise`) too, as
    that function awaits `F` as a promise;
  - unsizing one to a `dyn Future`, `Box::pin(s)` as a `Pin<Box<dyn
    Future>>` (`unsized_unlowered_future`).

  ```text
  error: rust-js does not support `S` as a promise yet: its `Future` is the crate's own, which rust-js never compiles (ADR 0425)
  ```

  A binding's future, `js::Promise<T>`, is a promise: its crate is a
  binding crate, not one rust-js compiled. So are an `async` block and
  what an `async fn` returns.

## Why

- **It's exact.** The `unlowered_impls` corpus case is either's `Either`,
  with all four impls, run against native Rust: its own methods are
  called, and the impls are no JS. `diagnostics.test.ts` refuses its
  `.await`, a `Pin` of it awaited, it given to an `F: Future`, it made a
  `dyn Future`, and a `read` of a crate's reader.
- **It's no JS at all,** as a `Hash` impl is.
- **The silent case is closed.** Allowing the impl would have made
  `.await` of one compile to the wrong value. The refusal is where a
  future is used as a promise, so a type that only has the impl, as
  `Either` mostly is, costs nothing.

## Alternatives

- **Lower `poll` and drive it from JS**, an executor of the crate's
  futures. Not now: it's ADR 0029's rejected alternative, a state machine
  in the output, for a future that `async fn`s already write as a promise.
- **Refuse the impl, as before.** It stops either, and itertools with it,
  for impls that a program using `Either` as a value never calls.
- **Make `.await` of a crate's future call its `poll` once.** A future
  that's `Pending` once would never wake: it needs the executor above.

## Consequences

- A future of the crate's own can't be awaited, pinned and awaited, made a
  `dyn Future`, or given to what takes an `F: Future`. A type that needs
  that writes an `async fn` instead, as ADR 0029 has it.
- A type parameter `F: Future` is a promise in its generic code, as
  before: the call that gives it a crate's future is what's refused.
- either compiles past its impls. Its next refusal is generic code calling
  `Extend::extend` on an `L: Extend<A>` (`docs/crate-corpus.md`).

## Amendment: what only an unlowered impl calls

base64's `DecoderReader` reads its delegate in a private helper,
`read_from_delegate`, that only its own `io::Read::read` calls: its call
of `R::read` was refused, though no JS could reach it. A private
function only an unlowered impl's methods call, or only functions found
so, is never lowered either (`reached_only_unlowered`): it's no JS, as
the impl is. One a consumer may call, or that nothing calls, is lowered
as it was. `test/corpus/unlowered_helpers.rs` holds it.

## Amendment: std's byte sources are read (#1156)

A read of std's own byte sources was refused as a crate's reader's was:
`src.read(&mut buf)` of a `&[u8]`, a `Cursor`'s `read_exact`, its
`lines()`. byteorder's tests read a `Cursor` throughout. `io::Read`,
`io::BufRead` and `io::Seek` are lowered now, by the coordinator's
decisions for the user (2026-11-11), case C of ADR 0262: what's read is
the program's data, so it's Rust's.

- **Each source's own methods are a runtime dictionary**, as `$vecWriter`
  is (ADR 0392): `$sliceReader` of a `&[u8]`, `$cursorReader(TAsRef)` of a
  `Cursor<T>`, made of `T`'s `AsRef<[u8]>` dictionary. They hold what each
  impl writes itself: `read`, `read_exact`, `read_to_end`,
  `read_to_string`, `fill_buf`, `consume`, `seek` and `stream_position`
  (`recognition::io`'s `DICTIONARY_METHODS`). std's bodies of these reach
  memory once inlined, `copy_from_slice` and `split_at`, so each is the
  runtime's, std's word for word: `read_exact` past the end empties a
  slice, or puts a cursor at its end, then fails `UnexpectedEof`, "failed
  to fill whole buffer"; `read_to_string` checks all that's left first,
  `InvalidData`, "stream did not contain valid UTF-8", and takes nothing
  of what isn't UTF-8; `consume` past a slice's end panics as slicing
  does; a cursor's position wraps as std's build does, and a `seek` below
  0 or past `u64` is `InvalidInput`.
- **One form for a known source and generic code's** (decision 1, A): the
  dictionary's method, given the source in a box as generic code's `&mut`
  is, `$sliceReader.read(src$1, buf)`, `cursorRead.readExact({ value: c },
  b)`. A dictionary made of what's fixed is named once at the top of the
  function that asks it, as any dictionary is (ADR 0979).
- **What the traits provide is std's own body**, compiled into
  `@rust-js/std`'s `io.js` (decision 5) and given the source's dictionary:
  `BufRead`'s `read_until`, `read_line`, `skip_until`, `lines` and `split`,
  the iterators `Lines` and `Split`, and `Seek`'s `rewind` and
  `seek_relative`. Where std's body reaches memory its callee is a
  recognizer: alloc's `append_to_string`, `$appendToString`, and core's
  `memchr`, the runtime's `$memchr`. ADR 1123's `alloc` is built for it.
- **`Cursor<T>` is std's struct, `{ inner, pos }`** (decisions 2 and 3):
  `new`, `position`, `set_position`, `get_ref`, `get_mut` and `into_inner`
  in place, `pos` a `u64`'s `BigInt`; its `Clone`, `Debug`, `PartialEq`
  and `Default` are std's bodies, compiled.
- **A generic `R: Read` is given the dictionary, a `dyn Read` is the
  pair**, and std's forwarding impls are dictionaries too: `&mut R`'s,
  `$mutReader`, as `$mutWriter` (ADR 0454), a `Box<R>`'s its `R`'s, and a
  `dyn`'s, `$dynReader`, given the pair in the box generic code is given.
  A provided method of a `dyn BufRead`, its `read_line`, is std's
  compiled body given `$dynReader`.
- **`read_exact` and what the traits provide of an `io::Result<()>` may
  fail**, as a crate's own writer may (ADR 0462): its error is thrown,
  caught where the result is a value.
- **A reader of the crate's own is still never lowered**, nor its impl
  given a dictionary, until #1297, which a refusal names: a call of its
  method, "calling `std::io::Read::read` (#1297)", and it given to generic
  code, "`S` as an `io::Read`, a reader of the crate's own (#1297)". Its
  provided methods, `read_to_end`'s `BorrowedBuf` among them, are #1297's.
- **A bound `R: Read` is a dictionary now**, so a function bounded by it
  takes one, `Counting::new(inner, RRead)` of `unlowered_helpers.rs`,
  even where its body asks nothing of it, as any bound's does.

Held by the corpus cases `io_read_slice.rs`, `io_cursor.rs`,
`io_buf_read.rs` and `io_read_generic.rs`, against native Rust in both
builds, their metamorphic rewrites, and `diagnostics.test.ts`'s two
refusals of a crate's reader. `VecDeque<u8>`, `io::empty`, `io::repeat`,
`chain`, `take`, `bytes` and `BufReader` are phase 2; `File` comes last.

## Amendment: a `VecDeque<u8>` is read, but its front slice (#1156, #1302)

alloc's `impl Read for VecDeque<u8>` and `impl BufRead` take what they
read from the deque's front. A deque is a JS array (ADR 0052), so its
`read_exact`, `read_to_end`, `read_to_string` and `consume` are
`$dequeReader`'s, std's bodies over the array, changed in place where
the deque's type is known: a `read_exact` that can't fill its buffer
empties the deque and fills nothing; `read_to_string` takes all of it,
as its `read_to_end` does, and adds nothing of text that isn't UTF-8;
`consume` past its end panics as `drain(..amt)` does. Its `read` and
`fill_buf` read its front slice alone, `as_slices().0`, which depends on
where the ring buffer wraps and an array has none of: they're refused,
naming #1302, and so is the deque given as generic code's `Read` or
`BufRead`, or asked what the traits provide, `read_line`, whose
dictionary would hold them (the coordinator's decision for the user,
2026-11-11). Case ([0262](0262-when-rust-and-js-disagree.md)): of what's
supported, none; of the front slice, Rust's behavior can't be kept
without the ring's layout, so a compile error. Held by the corpus case
`io_read_deque.rs`, against native Rust in both builds and its
metamorphic rewrites; `diagnostics.test.ts`'s three refusals; mutations
`deque-*` (`runtime/readers.ts`, `lower/std_types/io.ts`,
`lower/traits/dictionaries.ts`, `lower/recognition/io.ts`).

## Amendment: `io::empty()` and `io::repeat(byte)` are read (#1156)

`io::empty()` and `io::repeat(byte)` were refused, as calls of std's
functions rust-js didn't lower. **They're byte sources now
(`ByteSource::Empty` and `ByteSource::Repeat`, `recognition/io.rs`), each
with a runtime dictionary of what std's impls write
(`src/runtime/readers.js`), approved by the coordinator for the user on
2026-11-11.**

- `$emptyReader` is `Empty`'s `Read`, `BufRead` and `Seek`. It reads
  nothing. Its `read_exact` of a buffer that isn't empty is `UnexpectedEof`,
  and of one that is, `Ok`. Every position it seeks or reports is 0. It
  also holds `read_until`, `skip_until` and `read_line`, which std's impl
  of `Empty` writes itself (`ByteSource::writes`). Generic code and a
  `dyn` reach those through `BufRead`'s provided bodies instead, which
  read nothing from it just the same.
- `$repeatReader` is `Repeat`'s `Read`. It fills each buffer with its
  `byte`. Its `read_to_end` and `read_to_string` are
  `Err(Kind(OutOfMemory))`, as std's are, never a loop without end.

Their values are std's structs: `io::empty()` is `undefined`, a unit
struct's JS, and `io::repeat(5)` is `{ byte: 5 }`, as `Cursor::new` is
`{ inner, pos }`. A known reader is given its dictionary's receiver in a
box, as generic code gives it, though neither reader writes it; the
noise is P3 #1329. Case ([0262](0262-when-rust-and-js-disagree.md)):
none, each result std's. Held by the corpus case
`io_read_empty_and_repeat.rs`, known, generic, a `&mut dyn Read` and a
`Box<dyn BufRead>`, against native Rust in both builds and through its
metamorphic rewrites; mutations `empty-read-exact-of-nothing-fails`,
`repeat-read-fills-nothing` and `repeat-read-to-end-ends`
(`runtime/readers.ts`), and `empty-no-byte-source`,
`repeat-made-unanswered` and `empty-own-lines-unwritten`
(`lower/recognition/io.ts`).
