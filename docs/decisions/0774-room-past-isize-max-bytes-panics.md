# 0774. Room asked past `isize::MAX` bytes is a `capacity overflow` panic

Status: Accepted. Amends [0315](0315-vec-capacity-and-edits.md), whose
`with_capacity` was `[]` and `reserve` nothing, and
[0322](0322-text-and-slice-names.md), whose `String`'s were too; the
capacity [0419](0419-a-vecs-capacity-kept-where-asked.md) keeps is
checked as well (#183). Amended: a string's `repeat` is JS's own,
unchecked, as the user decided (below).

Case ([0262](0262-when-rust-and-js-disagree.md)): C, Rust's, paid for in
output, `$roomFor(n, 4)`; but of items whose size a type parameter gives,
and of a string's `repeat`, B, listed in the guide's differences, as the
user decided.

## Context

std panics with `capacity overflow` where room it's asked would be more
than `isize::MAX` bytes, as `Vec::with_capacity`, `reserve`,
`String::with_capacity`, `vec![x; n]` and `repeat` document. rust-js's
`usize` is 32-bit ([0011](0011-numbers.md)), so that's any request over
2³¹ − 1 bytes. The JS did something else: `with_capacity` was `[]` and
`reserve` nothing, so no panic; `vec![0u8; usize::MAX]` was `new
Array(4294967295).fill(0)`, which runs out of time or memory; `"ab".repeat(n)`
threw JS's `RangeError: Invalid string length`, not the panic, and only
where the engine's limit is.

## Decision

**Where a capacity is asked, the room is checked as std checks it:
`$roomFor(count, size)` panics `capacity overflow` where `count` items of
`size` bytes are more than `isize::MAX` bytes, or more than `usize::MAX`
items, of no size too.** The size is rustc's for wasm32, `size_of`'s
([0562](0562-a-pointer-is-a-place-and-an-address.md)); the lowering's
`room_for` is the one owner of the check, and `$roomFor` the runtime's.

| Rust | JS |
|---|---|
| `Vec::<u32>::with_capacity(n)`, a `VecDeque`'s, a `BinaryHeap`'s | `$roomFor(n, 4); []` |
| `v.reserve(n)`, `reserve_exact(n)` | `$roomFor(v.length + n, 4)` |
| `String::with_capacity(n)` | `$roomFor(n, 1); ""` |
| `s.reserve(n)`, `reserve_exact(n)` of a `String` | `$textRoomFor(s, n)`, its bytes counted only where they may not fit |
| `vec![x; n]` | `$roomFor(n, 4)` first, after `x` is made; `$repeat(x, n, 4, clone, drop)` of one with a destructor, which drops `x` where there's no room, as std's unwinding does |
| `"ab".repeat(n)` | `"ab".repeat(n)`, JS's own: the engine refuses a string past its length first (amended, below) |
| `[a, b].repeat(n)` | `$repeatItems(v, n, 4)`, room for `v.length * n` |

A check that can't fail is left out: room for none more, `with_capacity(0)`
or `reserve(0)`; a count known to fit, `Vec::<u64>::with_capacity(10)`;
a count alone of items of no size, `Vec::<()>::with_capacity(n)`. Room
for more of items of no size is checked by their count, `len + n` past
`usize::MAX`, as std's `checked_add` panics; so is a slice's `repeat`,
whose `checked_mul` does.

What's given to the call runs first, as Rust runs it: `vec![made(); n]`
is `const item = made(); const n = pastHalf(); $roomFor(n, 1);`.

**Of items whose size a type parameter gives**, `fn f<T>(n: usize) ->
Vec<T> { Vec::with_capacity(n) }`, the size isn't known where the call is,
and giving it would add a `TSize` to each generic function that asks room
and refuse a trait's method or a library's function, which can't be given
one ([0145](0145-type-facts.md)). So their size is taken as none:
room for them is checked by their count alone. That's case B, decided by
the user: with_capacity makes no array in JS, so nothing runs out, and a
`vec![x; n]` that std would refuse runs out of memory instead.

## Consequences

- `Vec::with_capacity(n)` of a count not known here is a statement more,
  `$roomFor(n, 4);`, and `reserve(n)` a statement where it was none.
- The capacity std's growth picks isn't kept (0419): a `push` past the
  room std's doubling asks, which may be more than its length, isn't
  checked, nor where a `Vec` grows by itself, `extend`, `resize`,
  `collect()`, nor a `HashMap`'s or a `HashSet`'s room; filed as their
  own question (#190).
- A JS string can't be as long as std's: `"-".repeat(1 << 30)`, a
  gigabyte, throws JS's `RangeError` where the engine's limit is less,
  Node's about 2²⁹ UTF-16 units; that's the engine running out, as memory
  does. Of a string's `repeat`, that's all there is (amended, below).
- Tested by `capacity_overflow.rs`, against native Rust, whose `usize`
  there is 64-bit, by room past half of `usize::MAX`, which is past both
  `isize::MAX`s; and by `runtime-package.test.ts`, where rust-js's 32-bit
  `isize::MAX` ends, one item either side.

## Amendment: a string's `repeat` is JS's own

`"ab".repeat(n)` printed `$roomFor(n, 2); "ab".repeat(n)`, and React's
ReactPerformanceTrackProperties, whose `prefix + '\xa0\xa0'.repeat(indent)`
and `' '.repeat(start)` are JS's own, read `$roomFor(indent, 4);` and
`$roomFor(start, 1);` before each. **A `str`'s or a `String`'s
`repeat(n)` is JS's `repeat`, unchecked**, as the user decided: a JS
string is at most 2³¹ − 1 UTF-16 units ([0682](0682-a-js-strings-length-is-at-most-int32-max.md)),
V8's (Node's, Chrome's) less than 2²⁹, each of 3 bytes at most, so less
than `isize::MAX` bytes. A repeat std refuses, V8 refuses too, before
it's made: a `RangeError: Invalid string length`, where std panics
`capacity overflow`. JavaScriptCore's (Safari's, Bun's) go to 2³¹ − 1
units, so there a string of text not all ASCII, past a billion units, may
be made where std panics, as memory lets it. That's case B, rare and
harmless: a program that asks a string that long runs out either way but
in that corner, and the error differs, which the guide lists among its
differences.

A `String::with_capacity(n)` and a `String`'s `reserve(n)` make no
string, so nothing else would refuse them: they're checked still, as a
`Vec`'s room and a slice's `repeat` are. `room_for` is still the one check of room asked; a string `repeat`
makes isn't asked of it, `str::repeat` being recognized as JS's method.
Tested by `test/lowering-strings.test.ts`, the bare `repeat` and its
`RangeError`; `capacity_overflow.rs` no longer asks a string past it.
