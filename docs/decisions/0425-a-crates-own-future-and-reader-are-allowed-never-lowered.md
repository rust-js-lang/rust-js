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

## Amendment: `chain`, `take` and `bytes` are read (#1156)

`Read::chain`, `Read::take` and `Read::bytes` were refused. They are std's
structs and std's bodies now, compiled into `@rust-js/std`'s `io.js` as
`Cursor` is. The coordinator decided this for the user on 2026-11-11.
Case C of [0262](0262-when-rust-and-js-disagree.md): what's read is the
program's data.

- **`Chain<T, U>` is `{ first, second, doneFirst }` and `Take<T>` is
  `{ inner, limit }`.** Their impls of `Read` and `BufRead` are std's
  bodies, as `chainRead_read` and `takeBufRead_fillBuf`. So are
  `into_inner`, `get_ref`, `get_mut`, `limit` and `set_limit`, and Chain's
  `read_to_end` and `read_until`, which its impl writes itself.
  `COMPILED_STD_IO` lists them, and `core::io::{chain, take}` are
  `COMPILED_STD_IO_FNS`.
- **Each impl is a dictionary, its factory's in `io.js`**
  (`chainRead($sliceReader, $sliceReader)`), given the dictionaries of
  what it holds. `byte_source_dictionary` gives a std reader
  (`recognition::is_std_reader`) its impl's factory, as `impl_dictionary`
  gives the crate's impls theirs. A known `Chain`'s or `Take`'s
  `DICTIONARY_METHODS` go through the dictionary, as a slice's do, in the
  same form. Repeated dictionaries are declared once (ADR 0979), including
  a runtime helper's of one, `const ChainRead =
  $readerDefaults(chainRead(..))`.
- **What a `Chain` or a `Take` doesn't write itself is std's default over
  its `read`, `$readerDefaults(reader)`**: `read_exact` is
  `default_read_exact`, `read_to_end` is `default_read_to_end`'s reads,
  and `read_to_string` is `default_read_to_string`, which is
  `$appendToString` over that `read_to_end`. Each keeps the bytes, errors
  and stopping point of std's, and retries an interrupted read.
  `$readerDefaults` reads in 8192-byte chunks, not as
  `default_read_to_end` does (a 32-byte probe, then spare capacity
  growing by `max_read_size`). That can't be seen for a reader whose
  sources are std's, which is the only kind it's given: a `Chain`'s or a
  `Take`'s dictionary (`byte_source_dictionary`'s std-reader branch,
  which an architecture test holds as the only writer of
  `"$readerDefaults"`). **A reader of the crate's own, #1297's, would see
  each `read`'s size, so `$readerDefaults` must never serve one unless
  its read sizes are std's.** Its `capacity()` does show: a `Vec` that
  `$readerDefaults`' `read_to_end` grows past its room is `$grewUnsized`,
  and its `capacity()` raises #1180's error naming #1340, never another
  number. One that fits its room keeps std's capacity. The coordinator
  decided this for the user (A), and #1340 is std's number. A `Chain`'s
  own `read_to_end` calls each source's, which reserves as std's does
  (ADR 0419, #1338).
- **`Bytes<R>` is std's struct, and `Bytes::next` std's body**. That body
  calls `SpecReadByte::spec_read_byte`, whose general impl calls alloc's
  `inlined_slow_read_byte`. That function reaches memory
  (`slice::from_mut`), so it is a recognizer: `IoOp::ReadByte`, the
  runtime's `$readByte(r, RRead)`, std's body word for word. It reads one
  byte through the reader's `read` of a one-byte buffer, retries
  `Interrupted`, gives `undefined` at 0 and otherwise the byte or the
  error. The coordinator decided this (B). **Each byte is one `read` of
  one byte, as std calls it, so a reader of the crate's own (#1297) will
  see std's own calls.** `SpecReadByte` is `OPTIMIZING_BY_TYPE`:
  `BufReader`'s impl only reads from its buffer what the general one
  would.
- **A call of a trait method that std's compiled impl writes itself**,
  such as a known `Chain`'s `read_until`, gets the impl's own forms. It
  is given the object, not the provided method's box of a `Self`:
  `call_callee` asks the resolved instance before the trait's compiled
  provided method, so `form_differs` crosses into the impl's signature.
- `recognition::tried_as_written` is the one owner of "a `?` of an
  `Option` or a `Result` is written by the lowering". Both dispatch and
  the walk that finds std's compiled bodies ask it, so std's
  `<Result as Try>::branch` isn't compiled. `same_kinds_of_generics`
  keeps `fact_specialization` from comparing `SpecReadByte`'s
  `StdinLock<'_>` impl, a lifetime, with `R`'s, a type.

Held by `test/corpus/io_read_chain_take_bytes.rs`, against native Rust in
both builds and through its metamorphic rewrites. It covers a chain of
slices read, `read_line`, `read_to_string` and `read_until` across its
seam, a character split between its parts, a take of a slice, of a
repeat and of a chain, `bytes` of each, generic `R: Read` and
`B: BufRead`, a `&mut dyn Read` and a `Box<dyn BufRead>`.
`test/lowering-readers.test.ts` holds the capacity refusal (#1340), and
`test/architecture-traits.test.ts` holds that `$readerDefaults` is given
std's readers alone. Mutations: `read-byte-reads-two`,
`reader-defaults-read-exact-short-quiet`,
`reader-defaults-over-its-own-read-to-end` and
`reader-defaults-capacity-kept` (`runtime/readers.ts`); `read-byte-compiled`,
`take-no-std-reader` and `std-reader-own-methods-compiled`
(`lower/recognition/io.ts`); `own-reader-given-a-dictionary`
(`lower/analysis.ts`); `compiled-impl-method-not-the-callee`
(`lower/mir/assign.ts`); `std-reader-read-no-defaults` and
`helper-made-dictionary-unnamed` (`lower/traits/dictionaries.ts`);
`std-reader-dictionary-takes-std-defaults` (`lower/traits/impls.ts`);
`specialization-generics-counted` and `spec-read-byte-not-optimizing`
(`lower/recognition/specialization.ts`); `io-chain-fn-not-compiled` and
`question-mark-try-compiled` (`lower/recognition/compiled_std.ts`);
`question-mark-written-of-from-residual-misread`
(`lower/recognition/types.ts`); and `read-byte-of-the-source-read`
(`lower/std_types/io.ts`). `BufReader` follows, below; `File` comes last.

## Amendment: a helper only a type's own `io::Read` calls is held by its snapshot (2026-11-11)

A private function only a type's own `io::Read` calls, `unlowered_helpers`'
`read_from_delegate`, was refused where it was lowered, so mutation
`unlowered-helpers-lowered` failed its case. Generic code's `R: Read` has
a dictionary now (`RRead.read(self, buf)`), so the helper lowers and runs:
the mutation's only effect is JS nothing calls, and it survived on main.
It's checked against the case's snapshot (`snapshots: true`, #864).

## Amendment: `BufReader` is read (#1156)

`BufReader::new`, `with_capacity` and its reads were refused. The user
decided its form on 2026-11-11 (#1156): std's fields flattened,
`{ inner, buf: new Array(cap).fill(0), pos: 0, filled: 0 }`, `initialized`
left out because no reader rust-js compiles can see it, 8192 for
`BufReader::new`; one dictionary factory, `$bufReader(R)`, declared once
per function as `chainRead` is; inherent methods in place as `Cursor`'s are;
`{:?}` refused for now, naming #1156. Case C of
[0262](0262-when-rust-and-js-disagree.md): what's read is the program's data.

- **`$bufReader(R)` is std's `impl Read`, `BufRead` and `Seek for
  BufReader<R>`, each method std's body word for word** over `{ inner, buf,
  pos, filled }` (std's `io/buffered/bufreader.rs` and `bufreader/buffer.rs`), `R` the
  dictionary of its inner's `Read`, or of its `Seek`
  (`byte_source_dictionary` asks the one the trait is). `read` goes past the
  buffer to its inner where nothing's buffered and the read is as large as
  the buffer. `fill_buf` is one `read` of the whole buffer, what `read_buf`'s
  default is, where `pos >= filled`; the buffer isn't refilled otherwise.
  `read_exact` takes what's buffered, else std's `default_read_exact`, now
  `$defaultReadExact`, shared with `$readerDefaults`. `read_to_end` reserves
  room for what's buffered, adds it, discards it and asks its inner's
  `read_to_end`; `read_to_string` into an empty `String` is
  `$appendToString` over that, into another it reads all, checks it as UTF-8
  and adds it. `consume` stops at `filled`. `seek` and `stream_position`
  count what's buffered, and `stream_position`'s `expect` panics with std's
  words. The inner is given to `R` in the box generic code is given a `&mut`
  in, whose `value` is the BufReader's `inner`, written back.
- **`seek_relative` is a method of every source's dictionary.** `BufReader`'s
  stays within what's buffered where it can and is the only one that isn't
  `Seek`'s default over its `seek`, which each other dictionary has
  (`$defaultSeekRelative`). It was a compiled provided method, called by a
  generic `S: Seek` whatever `S` was, so `BufReader`'s own, which asks its
  inner nothing, was lost: `("Seek", "seek_relative")` is no longer in
  `COMPILED_STD_IO_PROVIDED`, and `io_cursor.rs`'s snapshot calls
  `read.seekRelative(..)`. `BufReader::seek_relative`, its inherent method,
  is the dictionary's, and an `io::Result<()>` that may fail
  (`of_io_reading`).
- **`BufReader::new`, `with_capacity`, `get_ref`, `get_mut`, `into_inner`,
  `buffer` and `capacity` are read in place** (`IoOp::BufReaderNew` and the
  rest; `Cursor`'s and `BufReader`'s `get_ref`, `get_mut` and `into_inner`
  are one op, `IoOp::Inner`). `with_capacity` checks std's room
  (`$roomFor(n, 1)`, ADR 0774): `usize::MAX` is std's `capacity overflow`,
  not a JS `Array` that size. Its room is asked once both its arguments are
  evaluated: where the room is asked, each is spilled to a `const` where
  it isn't read the same, so `BufReader::with_capacity(usize::MAX, make())`
  calls `make()` before it panics, as std does (found in review: the inner
  was evaluated after the check, a wrong answer). Where the room surely
  fits, `with_capacity(4, cursor(..))`, nothing is spilled.
- **A `BufReader` of a `&mut` to an object holds generic code's box of
  it** (#1374, amended), `BufReader::new(&mut cursor)`: `$mutReader`, `&mut
  R`'s `Read`, takes generic code's box (`Form::Box`), so `new` and
  `with_capacity` hold `$cellOf(cursor)` (`buf_reader_inner_given`, `form_given`),
  and `get_ref` and `into_inner` give the reader back out of it, `.value`
  (`buf_reader_inner_back`, `form_back`). It was refused, as the object
  held where the box is read. What it reads moves the reader it borrows,
  which is read on once the `BufReader` is gone. Its `get_mut`, a `&mut`
  to the `&mut` it holds, is refused at compile time.
- **`{:?}` of a `BufReader` is refused**, naming #1156: std shows its
  inner's and `filled - pos` of its capacity.
- **A body that writes a `BufReader`'s dictionary twice declares it once**,
  `const BufReaderRead = $bufReader(..)`, named as a generic impl's is
  (`is_impl_call`, ADR 0979), as a `Chain`'s is.

Held by the corpus case `io_buf_reader.rs`, against native Rust in both
builds and through `METAMORPHIC=all`, and by `diagnostics.test.ts`'s two
refusals. Mutations (`runtime/readers.ts`): `buf-read-never-bypasses`,
`buf-read-bypasses-a-short-read`, `buf-fill-buf-always-refills`,
`buf-fill-buf-reads-half`,
`buf-consume-unbounded`,
`buf-read-exact-leaves-pos`, `buf-read-to-end-drops-buffered`,
`buf-read-to-end-keeps-buffered`, `buf-read-to-end-reserves-nothing`,
`buf-read-to-string-into-text-unchecked`,
`buf-seek-current-ignores-buffer`,
`buf-seek-keeps-buffer`, `buf-seek-overflow-moves-inner-once`,
`buf-stream-position-ignores-buffer`, `buf-stream-position-underflow-quiet`,
`buf-seek-relative-always-seeks`, `buf-seek-relative-forward-past-filled` and
`buf-seek-relative-back-past-start`; (`lower/recognition/io.ts`)
`buf-reader-no-byte-source`, `buf-reader-new-unanswered`,
`buf-reader-with-capacity-as-new`, `buf-reader-capacity-is-buffer`,
`buf-reader-get-ref-unanswered`, `buf-reader-seek-relative-unanswered`,
`seek-relative-not-in-dictionary` and `buf-reader-seek-relative-never-fails`;
(`lower/std_types/io.ts`) `buf-reader-default-capacity`,
`buf-reader-capacity-is-filled`, `buf-reader-buffer-from-start`,
`buf-reader-inner-made-after-room` and `buf-reader-held-mut-allowed`; (`lower/traits/dictionaries.ts`)
`buf-reader-dictionary-unmade` and `buf-reader-seek-asks-read`;
(`lower/traits/dictionaries/declared.ts`)
`buf-reader-dictionary-not-an-impl-call`; (`lower/display/debug.ts`)
`buf-reader-debug-shown`. `File` comes last.

## Amendment: `File` is read and written through Node's fs (#1156, phase 1)

The user's decisions of 2026-11-11, on #1156. **A `File` is `{ fd, pos }`**,
its descriptor and the position std's would keep, a `u64` the runtime
passes to every read and write (Node's `readSync` at a `bigint`, its
`writeSync` at a `number`, below 2^53, past which a write is refused where it
runs), so `Seek` moves it as `lseek` would: `Start(n)`, `End(n)` from
`fstat`'s size, `Current(n)`, and `EINVAL`, os error 22, below 0 or past an
`off_t`. A file opened to append is remembered by its descriptor, and writes
at its end, where its position is then. One that can't seek, a pipe, reads
and writes where it is, and its `seek` is `ESPIPE`.

- **Node's fs, where it runs:** `$nodeFs(what)` reads
  `process.getBuiltinModule("node:fs")` through `$process`, no static
  import, so in a browser it panics naming what needed it (ADR 0391,
  amended). `File::open`, `File::create`, `OpenOptions::open`, `fs::read`,
  `fs::read_to_string`, `fs::write` and `fs::remove_file` are `NODE_ONLY`
  entries, each a helper of `fs.js`.
- **`File` is one of std's byte sources,** `ByteSource::File`, its `Read` and
  `Seek` `$fileReader`'s, std's own methods over Node's calls, `read_exact`
  std's default over its `read`, `read_to_end` reserving what `fstat` says is
  left first, as `buffer_capacity_required` does. Its `Write` is
  `$fileWriter`'s: its `write`, and `write_all` and `write_fmt` over it,
  `$ioWriteAll`'s; `flush` does nothing, as a file isn't buffered.
- **`OpenOptions` is the runtime's `$OpenOptions`** (the user's decision): a
  class whose setters set its flags and give it back, std's `&mut Self`, so
  `OpenOptions::new().append(true).create(true).open(p)` is
  `$openOptions().append(true).create(true).open(p)`, a stored builder too.
  Its `open` is std's `get_access_mode` and `get_creation_mode` word for
  word: options that open nothing are std's `InvalidInput` and message.
- **A write of a file may fail,** so a crate that calls std's files is one
  whose writes may (`of_files`, ADR 0462's `any_failing`), and a `File`'s
  `write_all` the OS refuses is its caller's error, though generic code
  writes it (`fs_write_only_fails`).
- **Closed once it's collected,** one `FinalizationRegistry` (the user's
  decision, difference 51, a rare and harmless exception of ADR 0262): rustc checks a
  program for wasm32-unknown-unknown, whose `File` is
  `sys::fs::unsupported::File(!)`, which drops nothing, so its MIR has no
  drop of a `File` for a close to be written at. Exact close at Rust's drop
  is #1391 (P2).
- `{:?}` of a `File` or an `OpenOptions`, which shows what the OS says of
  it, is refused, naming #1156.

Corpus cases `fs_file` (written, read whole, by line through a `BufReader`
and by generic code; read in parts, sought from each end and before its
start; a hole past its end; appended through stored options, `create_new`
of one there, `truncate`; std's four refusals of options; the OS's errors of
a missing file, a write to a reader, a read of a writer, a directory, and
text that isn't UTF-8) and `fs_write_only_fails`, against native Rust in
both builds and through `METAMORPHIC=all`, each writing beside its outcome's
file; `test/process.test.ts` holds the `NODE_ONLY` entries and their gates
and a browser's panic, and `test/asserts.rs` a `#[cfg(browser)]` test of it
in Chromium; `test/diagnostics.test.ts` the two `{:?}` refusals.

## Amendment: a `&File`, a file's metadata and `{:?}`, and directories (#1397, phase 2)

**A `File` is `{ fd, pos, path, read, write, append }`:** beside its
descriptor and position, the path it was opened by, made absolute where it's
opened, and the modes it was opened with, `O_ACCMODE`'s, read where asked
and written where written or appended to, which `{:?}` shows; and whether
it appends, where `$appending`, a set of descriptors, held it apart.

- **A `&File` reads, writes and seeks as the `File` does** (std's `impl Read
  for &File` and the rest): `is_file_handle`, a `File` or a `&File`, is its
  byte source and writer, `$fileReader` and `$fileWriter` given the same
  object, so each handle moves the one position.
- **`{:?}` of a `File` is std's of unix** (the user's decision, 2026-11-11):
  `File { fd, path, read, write }`, `$fileDebug(f, options)`, its descriptor
  Node's, its path `fs.realpathSync` of the one it was opened by, at print
  time, left out where that resolves to none, as std's `get_path` leaves it
  out where it can't say; a number and a `bool` given the placeholder's
  options as `debug_struct` gives each field them. Rust's own descriptor
  differs by environment too. A file renamed or removed since it was opened
  shows no `path`, where macOS's `F_GETPATH` shows a renamed one's new path
  and a removed one's last, and Linux's `/proc/self/fd` the new path or the
  old one with ` (deleted)`: the second case's, the user's decision, the
  guide's difference 54. So `unwrap_err()` of an `io::Result<File>` that's
  `Ok` panics with std's message.
- **A `Metadata` is Node's `Stats` of `bigint`s**, `fstat`'s, `stat`'s or
  `lstat`'s: `len()` its `size`, `is_file()` and the rest its methods,
  `modified()` and `accessed()` its nanoseconds, a `SystemTime`. A
  `FileType` is the name of its kind, `"file"`, `"dir"`, `"symlink"` or
  another of `S_IFMT`'s, so its `==` and `Hash` are JS's, and its `{:?}` is
  std's, `FileType { is_file, is_dir, is_symlink, .. }`.
- **`set_len`, `sync_all` and `sync_data`** are `ftruncate`, `fsync` and
  `fdatasync`; a length past an `off_t` is std's error of its conversion, a
  `TryFromIntError`'s, and one past 2^53 is refused where it runs, as a
  write there is (#1418).
- **Directories:** `fs::create_dir`, `create_dir_all` (std's `DirBuilder`'s
  steps word for word), `remove_dir` and `read_dir`. A `ReadDir` is a
  generator that reads each entry as it's asked, in the OS's order,
  `Dir.readSync`'s, `readdir`'s as std's, not `readdirSync`'s sorted one; its
  directory is opened where it's made, as std's `opendir` is, and closed once
  every entry is read, or once it's collected, as a file is (difference 51).
  A `DirEntry` is `{ path, name, type }`: `path()` the directory's joined
  with its name, `file_name()` its name, `file_type()` what `readdir` said,
  `lstat`'s where it said nothing, and `metadata()` `lstat`'s.
- `{:?}` of an `OpenOptions`, a `Metadata`, a `DirEntry` or a `ReadDir` is
  refused, naming #1418, which holds what's left of std's files.

Corpus cases `fs_file_shared` (two `&File`s of one file, a `BufReader` of
one, generic code given one; `{:?}`, `{:#?}` and `{:5?}` of files opened in
each mode, their descriptor and the outcome's path masked; `unwrap_err()` of
one opened) and `fs_dirs_and_metadata` (directories made, found, refused;
entries in the OS's order, each one's path, name, type and length; metadata
of a file and a directory; `set_len`, its position kept, past an `off_t`, of
a reader; syncs; `remove_dir` of a full one and of none) run as native Rust
in both builds and through `METAMORPHIC=all`.
