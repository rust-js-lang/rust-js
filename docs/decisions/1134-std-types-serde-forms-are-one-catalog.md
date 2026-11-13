# 1134. std types' serde forms are one catalog

Status: Accepted (2026-11-13, #1623). m6-react set the conditions: one
catalog, extended row by row; each row serde's own impl's form, compared
with native both ways, its errors among them; landed by family. Beside
[0077](0077-serde-json.md), JSON by serde_json's rules, and
[0459](0459-hand-written-serde-impls.md), hand-written impls.

Case ([0262](0262-when-rust-and-js-disagree.md)): results change if the
form isn't serde's, so each row is serde's own, compared with native
serde_json; a std type with no row is refused, as before.

## Context

serde implements `Serialize` and `Deserialize` for many of std's types
itself (`serde_core`'s `ser/impls.rs` and `de/impls.rs`): `Range` as
`{"start","end"}`, `Duration` as `{"secs","nanos"}`, `Bound` externally
tagged, `Cell<T>` as its `T`. rust-js wrote and read a few by hand in
serde.rs's and de.rs's type dispatch (`Option`, `Result`, the
collections) and refused the rest: 24 of the 89 serde probes still
refused were one of these (#1623). Writing each in both dispatches would
be two places per type, each a one-off.

## Decision

**`src/lower/serde/std_forms.rs` is the one catalog of std types whose
impl is serde's own: `std_form(ty)` gives a type's row, and the writer
and the reader each ask it before anything else of an ADT.** A row is a
form:

- **A struct form**, serde's `serialize_struct(name, ..)` of named
  fields, each held by the JS value under its own name: written
  `beginObject`, each `key` and value, `endObject`; read by the runtime's
  `struct` reader, which takes a map or a sequence, with serde's own
  visitor's rules: any other field refused (`deny`, `unknown field `x`,
  expected `start` or `end``), and a sequence too short in its visitor's
  words, `struct Range`, not a derive's `struct Range with 2 elements`
  (`expecting`). Made of the values in the fields' order, `([start,
  end]) => ({ start, end })`.
- **An enum form**, externally tagged, each variant written and read as a
  derive of the enum's own definition does (`write_adt`, `read_tagged`),
  its variants named in serde's order, which serde's messages list:
  `expected one of `Unbounded`, `Included`, `Excluded``, not the enum's
  declared `Included, Excluded, Unbounded`. serde's impl reads a variant's
  index too, with other words, but serde_json reads a variant from a
  string only, so no message differs.

A new row is a match arm; a form a row needs that neither gives (a
`Duration`'s, held as a BigInt of nanoseconds, made by a check of its own)
extends the forms, in this file.

The first family, ranges and bounds: `Range<T>` and `RangeInclusive<T>`,
struct forms of `start` and `end` (`RangeInclusive`'s `start()` and
`end()` are its fields, ADR 0129, so an exhausted one writes them as
serde's `start()` and `end()` give them); `Bound<T>`, an enum form.

## Consequences

- The serde probes `std_04`, `std_05`, `std_16` and `std_28` give
  native's answer in all three modes.
- `test/serde-std.test.ts` holds each row against native serde_json, to
  and from text and from a `Value`, its errors among them.
- Later families extend the catalog: time, cells and locks, numbers,
  paths and strings (#1623).

## Amendment: time (2026-11-13, #1623)

`Duration` and `SystemTime` are struct forms whose value doesn't hold
their fields by name: a `Duration` is its nanoseconds, a BigInt (ADR
0188), and a `SystemTime` its nanoseconds since the epoch (ADR 0447). The
struct form says how its value holds its fields (`Held`): by their names,
or as a `Duration`'s nanoseconds, whose `secs` and `nanos` are what
`as_secs()` and `subsec_nanos()` give, the one lowering of those
(`std_types::number::{duration_as, duration_subsec}`, which `DurationOp`'s
arms call too). Three runtime helpers of serde's checks, in
`json_std_forms.js`, each in serde's words:

- written, a `SystemTime` first gives its time since the epoch, or
  `SystemTime must be later than UNIX_EPOCH` before it
  (`$systemTimeSinceEpochForJson`);
- read, a `Duration` is made of its parts by `$durationOfJson`, `overflow
  deserializing Duration` where its nanoseconds carry past `u64::MAX`
  seconds, else `$durationNew`, which carries them, so `{"secs":1,
  "nanos":2000000000}` is 3s; a `SystemTime`'s offset so, `overflow
  deserializing SystemTime epoch offset`, then, of the struct read whole,
  `overflow deserializing SystemTime` past unix's `i64` seconds
  (`$systemTimeOfJson`), which serde checks past its reader, so its error
  has no position, as serde_json's hasn't.

The catalog is asked before any other kind of type, as a `Duration` is held
as a number, which the numbers' rules would take. Probes `std_03`,
`std_15` and `std_22` give native's answer in all three modes.

## Amendment: wrappers (2026-11-13, #1623)

A newtype form, what a `Reverse<T>` or a `Wrapping<T>` holds, written and
read as that, `Wrapping(3)` as `3`, its tuple struct's field, `value[0]`
(ADR 0023), as `{:?}` reads it; a unit form, `PhantomData<T>`, `null`,
read as serde's unit, `invalid type: integer `1`, expected unit`. Probes
`std_09`, `std_10` and `std_27` give native's answer in all three modes.
`Saturating<T>` is the numbers family's: serde reads one by its own
visitor, which clamps what's out of range. The cells and locks, whose
value and borrow are what the cell analysis decides for each, come after
a design of their own (#1623).

## Amendment: cells and locks (2026-11-13, #1623)

A cell form: what a `Cell`, a `RefCell`, a `Mutex` or an `RwLock` holds,
read through it as serde's impl does, and read as `::new` of what's read
(serde's `forwarded_impl`). Each asks the cell's own rules, which
`std_types/cell.rs` now has as one function each, `::new`'s and `get`'s
arms calling them too: `cell_made(ty, value)`, `{ value }` or, of a
`RefCell` of an object, the object (ADR 0863); and `cell_held(ty, cell)`,
what `get()` reads. Written:

- a `Cell`, what it holds, `get()`'s;
- a `RefCell`, `try_borrow()`'s, `$jsonBorrowed`: `already mutably
  borrowed` where it's mutably borrowed; a handle on a field no borrow of
  which is counted (ADR 0863, amended), as it's held;
- a `Mutex`, `lock()`'s, and an `RwLock`, `read()`'s, `$jsonLocked`: `lock
  poison error while serializing` where a panic poisoned it; locked again
  while the thread holds it, rust-js's lock's own refusal of Rust's
  deadlock (ADR 0328's decision, unchanged); a panic as its value is
  written poisons a `Mutex`, as its guard dropped while the panic unwinds
  does, and a write's error, a `Result`, doesn't.

The two helpers are emitted only where a `RefCell`, a `Mutex` or an
`RwLock` is serialized. A derived reader now makes each field as any struct
literal does (`holding`, ADR 0288): a struct's `Cell` field is what the
cell holds, which a `{ value }` read from JSON wasn't, `get()` then reading
`[object Object]`. Probes `std_07`, `std_08` and `std_20` give native's
answer in all three modes.

## Amendment: numbers (2026-11-13, #1623)

A 128-bit integer, a primitive, is the numbers' own dispatch's, not a row:
written as its digits, by its type's name, `int(n, "i128")`, which a
`Value`'s writer holds only where it fits in 64 bits, serde_json's `number
out of range` (#1547's writer, a hand-written impl's too); read by
`int128`, which of a value already read is any integer's `int` (a `Value`'s
number fits in 64 bits), and of the text serde_json's own scan of its
digits (`do_deserialize_i128`, `scan_integer128`): an optional `-` of an
`i128` alone, whole, never a float, `invalid number` and `number out of
range` where serde_json says them, a map key's too. ADR 0171 amended.

A row, `Saturating<T>` of an integer type up to 64 bits: written as a
newtype; read by serde's own visitor, `integer with support for saturating
semantics`, which clamps any integer to `T`'s range and refuses a float
(`$json.saturating(min, max)`, its bounds `Num::range`'s). A 128-bit one
has no row. Probes `big_00` to `big_03` and `num_i128` give native's
answer in all three modes.
