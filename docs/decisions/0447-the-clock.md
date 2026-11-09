# 0447. The clock: a `SystemTime` and an `Instant` are their nanoseconds

Status: Accepted. Extends [0188](0188-duration.md), whose `Duration` is
its nanoseconds, a BigInt, and [0177](0177-nonzero.md).

Case: R ([0262](0262-when-rust-and-js-disagree.md)) for what a time
reads, JS's clocks, which a program can see only as a platform's own: a
`SystemTime` to the millisecond, as some platforms' are. Case N for the
rest: the same times, differences and errors as std's.

## Context

`SystemTime::now()` and `Instant::now()` were refused (listed in
[the limitations](../guide/limitations.md)). A UI times what it does, and code
shared with a server stamps what it makes: uuid's v1 and v7 read
`SystemTime::now().duration_since(UNIX_EPOCH)`, chrono's `Utc::now()` reads
it, and a frame's or a request's time is an `Instant`'s `elapsed()`. Each
crate of the shared-model corpus that keeps time stopped there.

## Decision

**A `SystemTime` is its nanoseconds since the Unix epoch, and an `Instant`
its nanoseconds of JS's clock that only goes forward, each a BigInt, as a
`Duration` is its own. A `SystemTimeError` is the `Duration` it holds.**

```rust
let start = Instant::now();
let since = SystemTime::now().duration_since(UNIX_EPOCH)?;
let t2 = start + Duration::from_millis(1500);
println!("{:?}", t2 - start);
```

```js
const start = $instantNow();
const since = $timeSince($systemNow(), 0n); // checked with `?`
const t2 = start + 1500n * 1000000n;
console.log(`${$debugDuration($instantSince(t2, start))}`);
```

1. **`now()` reads JS's clocks**: `SystemTime::now()` is `Date.now()` in
   nanoseconds, `$systemNow`; `Instant::now()` is `performance.timeOrigin
   + performance.now()`, `$instantNow`, which only goes forward, as std's
   `Instant` does. `UNIX_EPOCH` is `0n`.
2. **The time between two** is `a - b`: a `SystemTime`'s `duration_since`
   a `Result`, `Err` of a `SystemTimeError` of the time `b` is after,
   `$timeSince`; an `Instant`'s zero where `b` is after, as std's
   saturates, `$instantSince`, and `None`, `checked_duration_since`'s,
   `$instantCheckedSince`. `elapsed()` is the time since, from `now()`.
3. **`t + d` and `t - d`** of a `Duration` are the BigInt's `+` and `-`;
   `checked_add` and `checked_sub` are `Some` of it, which a BigInt never
   overflows. `t += d` is `t = t + d`, the time's own `+`.
4. **A time compares as its number**, `<` and `===`, its nanoseconds.
5. **A `SystemTimeError`'s `Display` is std's message**, "second time
   provided was later than self", and its `{:?}` is
   `SystemTimeError(1.5s)`. A time's `{:?}` is its platform's fields,
   which JS hasn't: refused (since amended: unix's, below).

### `a += b` of a number whose `+` is its own

`a += b` of a `Duration` or a time is `a = a + b`, of its type's own `+`:
std writes each `+=` as that, and recognition answers what the `+` is,
`Recognition::assigned_by`. A number's `+=` is still its number's,
`has_number_arithmetic`, the one place that says a `Duration`'s and a
time's arithmetic isn't. A `Duration`'s `-=` past zero now panics as its
`-` does, "overflow when subtracting durations"; it was a number's `-`.

### A number has no fields

A type that's a number, `Num::of`, a `Duration`, a `NonZero`, a time, has
no shape of fields: a `&mut Duration` is a box of its nanoseconds, as a
`&mut u64` is (ADR 0074). It was an object whose fields were written,
so `*d = *d + one` through a `&mut` was lost: a wrong answer, fixed. A
`NonZero`, never `0`, is never falsy, so an `Option<NonZero>` is tested
by its value, as it was by its shape before.

## Why

- **It's what UI and shared code reads**: a timestamp, a frame's time,
  uuid's v7, chrono's `now`.
- **JS's building blocks**: `Date.now()` and `performance.now()` are the
  clocks a browser and Node have; a BigInt keeps nanoseconds exact, as a
  `Duration`'s are.
- **One way**: a time is its number, as a `Duration` is, so its `+`,
  comparisons and `&mut` are the ones a `Duration` has.
- **It's tested**: a corpus case reads both clocks, takes their
  differences each way, shifts and compares them, against native Rust;
  another assigns a `Duration` through a `&mut`, and another panics in
  `-=`; a mutation of each part is caught.

## Alternatives

- **A `Date` for a `SystemTime`**: milliseconds only, an object, and a
  second representation of a time beside a `Duration`'s.
- **A number of milliseconds**: what `now()` gives, but not what a time
  plus a `Duration` of nanoseconds is.

## Consequences

- `SystemTime::now()` is to the millisecond, `Date.now()`'s; std's is to
  the platform's, a microsecond or finer. A program sees only a coarser
  clock, as on some platforms (case R).
- `Instant::now()` counts from the epoch, `timeOrigin`, where a platform's
  counts from its boot; what an `Instant` is is opaque in std, so only
  its differences are read.
- A time's `{:?}` is refused, listed in [the limitations](../guide/limitations.md).

## Amendment: a time's `{:?}` is unix's and wasip1's (2026-11-10)

A time's `{:?}` was refused as its platform's fields (#1159), which
chrono's `test_from_system_time` reaches through `assert_eq!`. **rust-js's
std platform for time is wasip1's, which is unix's: a `SystemTime`'s
`{:?}` is `SystemTime { tv_sec: 1700000000, tv_nsec: 123000000 }`, an
`Instant`'s `Instant { tv_sec: .., tv_nsec: .. }`, its `Timespec`'s
seconds, before the epoch below zero, and the nanoseconds past them,
`$timeDebug(name, nanos, options)`; each field is an integer given the
placeholder's options, as std's `debug_struct` gives them, `{:#?}`'s
lines, a width, a sign, zeros and `{:x?}`'s hex, an `i64`'s and a `u32`'s.**
Why unix's, not wasm32-unknown-unknown's `SystemTime(1700000000.123s)`,
the `_` arm of std's `sys/time` that rustc checks rust-js against: rust-js's
time already isn't unknown-unknown's, whose `now()` panics "time not
implemented on this platform", where rust-js's reads the host's clock, as
wasip1's and unix's do. Its model is a platform with a clock, whose native
reference is wasip1, which the corpus runs (`//@ native: wasm32`), and it's
what the same type prints on a native server, Rust across the stack. Case
1 of ADR 0262 for the target's form: no result changes, `Debug`'s output
being unstable, and the time shown the same. An `Instant` begins at the
host's `performance` origin, past the epoch, as a platform's begins past
its boot: as in std, only the difference of two is meaningful, and its
fields' values are not the native platform's.

A clock's time and error are their nanoseconds in JS, a `Num`, and a
struct to their `fmt`s, so a placeholder's options of one aren't applied to
its text as a whole, as a number's are (`display::shown_num`, the one
answer `format_value` and `is_debug_leaf` ask): `{:>20?}` of a
`SystemTimeError` was its text padded, `[    SystemTimeError(1.5s)]`,
where std's gives the width to its `Duration`, `[SystemTimeError(
1.5s)]` padded inside, a wrong answer; it's refused, as its other options
were. Held by `time_debug_as_unix.rs` against wasip1's std and the Mac's,
`system_time_error_width_refused.rs`, and the mutations
`time-debug-as-its-error`, `time-shown-as-a-number`,
`time-debug-options-dropped`, `time-debug-before-epoch-unborrowed` and
`time-debug-hex-unread`.

