# 0327. Cells', locks', a `VecDeque`'s, `char`'s and arrays' other methods

Status: Accepted. Extends [0025](0025-vec-loops-refcell-mut.md),
[0144](0144-locks.md) and [0152](0152-std-item-handles.md); counted by
[0314](0314-std-data-structures.md).

Case: C ([0262](0262-when-rust-and-js-disagree.md)), but for what a ring
buffer's layout or a borrow's state says, D: refused.

## Context

ADR 0314's count had `Cell` at 6 of its 13 methods, `RefCell` 7 of 13,
`VecDeque` 34 of 55, `char` 30 of 38 and arrays 3 of 5: a cell's
`get_mut` and `swap`, a deque's `get` and `front_mut`, `char`'s escapes,
`encode_utf8` and `decode_utf16`, and an array's `each_mut` were refused.

## Decision

**Each is std's, on the cell's `{ value }`, the deque's JS array, and a
`char`'s or a `str`'s text.**

| Rust | JS |
|---|---|
| a cell's `get_mut()` | the cell, the `{ value }` box a `&mut` to a number or text is (ADR 0074), or its object |
| `a.swap(&b)`, `c.update(f)` | `$cellSwap(a, b)`, `c.value = f(c.value)` |
| a lock's `is_poisoned()`, `clear_poison()` | `false`, nothing |
| a deque's `get(i)`, `get_mut`, `front_mut`, `back_mut`, `range(r)` | a slice's (ADR 0152 for the `&mut`s) |
| `swap_remove_back(i)`, `swap_remove_front(i)`, `pop_front_if(f)`, `pop_back_if(f)` | `$dequeSwapRemove`, `$popFrontIf`, `pop_if`'s |
| `retain_mut(f)`, a `Vec`'s and a deque's | `$retainMut(v, f, handles)`, `f` given a handle on a number or text |
| `escape_default()`, `escape_debug()`, `escape_unicode()` | `$escapeDefault`, `$escapeDebug`, `$escapeUnicode`: text, which `{}` shows and a loop goes over |
| `c.encode_utf8(&mut buf)`, `char::decode_utf16(units)` | its bytes in `buf`, and `{ value: c }`; each `Ok` or `Err` of the code |
| `c.make_ascii_uppercase()`, an array's `each_ref()`, `each_mut()` | `c = $asciiCase(c, true)`; the array, `$mutItems` of its numbers |

- **A lock is never poisoned**: a panic that holds one ends the program,
  as nothing catches it (ADR 0035).
- **`escape_debug` is `{:?}`'s, `$debugChar`, both quotes escaped**: a
  `str`'s escapes a combining mark only first, a `char`'s always.
  `$debugStr` shows each `char` by it too.
- **Refused, loud**: a `VecDeque`'s `as_slices`, whose split is its ring
  buffer's layout, which a JS array hasn't; `char::encode_utf16`, whose
  `&mut [u16]` is part of its buffer, a sub-slice a JS array can't be; and
  `try_borrow`, `try_borrow_mut`, `try_lock`, `try_read` and `try_write`,
  which ask of a borrow's or a lock's state, which rust-js doesn't keep
  (ADR 0025). (Amended by ADR 0328: a `RefCell` counts its borrows, and
  `try_borrow` and `try_borrow_mut` answer as std's do.)

## Why

- **It's the same program**: each reads and writes as std's does.
- **It's tested**: a corpus case runs each against native Rust, numbers
  written through `get_mut`, `retain_mut` and `front_mut`, a swapped
  `RefCell`, escapes of control, quote, accented, emoji and combining
  `char`s, a UTF-8 buffer and an unpaired surrogate. Mutations escape a
  later combining mark, leave non-ASCII raw, write no bytes, split a
  surrogate pair, swap from the back, keep what `retain_mut` rejects, give
  numbers without handles, and copy `get_mut`'s number.
- `docs/std-coverage.txt`: `Cell` 9 of 13, `RefCell` 9 of 13, `VecDeque`
  45 of 55, `char` 37 of 38, arrays 5 of 5.

## Since

- **A deque's `range_mut(r)` is a slice's `iter_mut()` of `&mut d[r]`**
  (2026-10-10): `$mutItems($view(d, a, b))` of numbers or text, the view
  itself of objects, checked as `&mut d[a..b]` is, whose panics are
  `slice::range`'s. It was refused. The `deque_range_mut` corpus case
  runs, against native Rust, numbers after a `push_front`, strings,
  tuples, a kept one stepped, and a range of one; `deque_range_mut_panic`
  one past the end. Mutations give numbers for handles, and refuse it.
  `docs/std-coverage.txt`: `VecDeque` 49 of 55.
