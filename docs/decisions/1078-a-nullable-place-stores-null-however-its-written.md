# 1078. A nullable place stores `null` for `None` however it's written

Status: Accepted (2026-11-04, #541, P0). Amends
[0791](0791-a-cells-storage-says-what-it-stores.md), whose handle stored
as its place did only on a nullable field's cell; and
[0949](0949-a-nullable-places-read-is-returned-as-it-is.md),
[0586](0586-a-nullable-field-is-an-option.md) and
[0861](0861-a-functions-own-nullable-parameter-is-set-null.md), whose
places' reads are taken as `null` or a value, which only their direct
writes kept true.

Case ([0262](0262-when-rust-and-js-disagree.md)): R, results change.

## Context

A `#[rust_js::nullable]` place's read is taken as `null` or a value: a
nullable thread-local's `get()` and `with_borrow(Clone::clone)` (ADR 0949),
a nullable field's (ADRs 0586, 0841, 1072), a function's own nullable
parameter (ADR 0861). So a nullable parameter every call gives one of them
is tested `=== null` (rd624), and a nullable function returns one as it
is. Only direct writes stored `null`, `KEY.set(v)` and `fiber.owner = v`.
A write through a borrow stored what it was given, `undefined` too:

```rust
CURRENT.with(|c| c.set(none_of(n)));            // current = noneOf(n);
TARGET.with_borrow_mut(|t| *t = none_of(n));    // target = noneOf(n);
clear_slot(&mut fiber.owner, n);                // fiber.owner = slot.value;
```

Then `absent(CURRENT.get())`, `x === null`, said `false` where Rust says
`true`, and `#[rust_js::nullable] fn current() -> Option<u32> {
CURRENT.get() }` gave JS `undefined` where its `.d.ts` says `number |
null`. Each write through a `&mut` did the same: `mem::swap`, `take`,
`replace`, and generic code given the `&mut`, `fn clear<T>(slot: &mut
Option<T>) { *slot = None }`, which writes `slot.value = undefined`.

The `undefined` writes are as old as the places. The answer turned wrong
where their reads were trusted: 1ab099b95 (ADR 0949) returned a nullable
thread-local's read as it is from a nullable function, and 93a3bfc39 (ADR
0861, amended) tested a nullable parameter `=== null` where each call
gives `null` or a value, which the issue's `false false false` needs.

## Decision

**What's written through a borrow of a nullable place is stored as the
place stores it.** There's one answer to which places store so,
`nulled::stores_null_for_none`: a nullable field, one a `RefCell`'s
`get_mut` lent, an item of a nullable field's items, a function's own
nullable parameter. A direct write asks it (`assign.rs`). So does a `&mut`
to it. One answer to what's stored, `Storage::written` (ADR 0791), serves
every path:

- **A `&mut` to such a place** is a handle on it that stores so,
  `Storage::NullForNone`, held by a variable whose `value` is the `&mut`'s
  place (`nulled::nullable_ref`, `nulled::through_handle`). Tidy reads and
  writes it as the place, each write as its storage stores (ADR 0791). This
  is how a nullable `RefCell` field's guard already worked. A callee's box is
  copied back `?? null`. So are `*r = v`, `mem::swap`, `take` and
  `replace`. A call that gives one, `get_mut()` or `&mut v[i]`, gives it
  so too. A reborrow of a nullable field's cell's `get_mut()` keeps the
  handle (`Expr::held` of the handle would be the place alone).
  `mem::take`, `mem::replace`, `Option::take` and `Option::replace` of
  one write its place as a path, `null` for `None`. They know it by the
  handle (`unhandled`). That retires the second owner, `entries.rs`'s
  `lends_nullable_place`, which found a nullable field and an item by
  walking MIR's reborrows (ADR 0996), and its two mutations. The JS is as
  it was: `const updates = hidden[index]; hidden[index] = null;`.

  | Rust | JS |
  |---|---|
  | `clear_slot(&mut fiber.owner, 1)` | `const slot = { value: fiber.owner }; clearSlot(slot, 1); fiber.owner = slot.value ?? null;` |
  | `std::mem::swap(&mut fiber.owner, &mut other)` | `const t = fiber.owner; fiber.owner = other ?? null; other = t;` |
  | `clear_generic(&mut x)`, `x` a nullable parameter | `const slot = { value: x }; clearGeneric(slot); x = slot.value ?? null;` |

- **A nullable thread-local that's its module's variable** (ADR 0270) is
  lent by `with` and `with_borrow_mut` a handle on what it holds that
  stores so (`cell::nullable_lent`, `QuietCells::nullable_lent`). Where the
  closure is applied in place (ADR 0840), its writes through the handle are
  the variable's, stored so (`js::applied`'s `through_handles`):
  `current = noneOf(n) ?? null;`. A function given it, `CURRENT.with(clear_cell)`,
  is given the handle, `{ get value() { return current; }, set
  value(value) { current = value ?? null; } }`.
- **One kept as its cell**, lent where something counts its borrows, or
  handed on whole, is a cell that stores `null` for `None` however it's
  written: `const target = $nullableCell(null);` (`plain_statics`). It
  covers its lends, `$withBorrowMut(target, f, true)`, a `with`'s `&RefCell`
  borrowed inside, and the runtime's own writes, `$cellReplace`. Its
  borrows are counted on it as on any cell, and only its `value` is seen.

What `insert`, `get_or_insert` and `get_or_insert_with` write there is a
`Some`'s value. Of one never boxed, that's there (`Known::Present`), so it's
stored as it is, `thenableState ??= createThenableState();`, React's
`unwrapThenable`, not `?? null`.

Reads are as before. `reads_nullable_local`, `null_where_made` and
`NulledParams` stay the one answer of what's `null` or a value. That
answer is now true.

## Alternatives

- **Stop taking such reads as `null` or a value** where a write through a
  borrow can't be shown. A library's thread-local may be lent by its
  consumer, whose writes its own reads can't see, so the answer would need
  every crate. React's reads would gain `?? null`.
- **Normalize after the call**, `TARGET.value ??= null` after a
  `with_borrow_mut`. A `Cell`'s `with` lets its closure read the key while
  it's lent, `CURRENT.with(|c| { c.set(None); CURRENT.get() })`. So the
  read would come before the fix.
- **Refuse `with` of a nullable `RefCell` thread-local** where its borrows
  are counted. That's valid, idiomatic Rust, and `$nullableCell` keeps it.

## What holds it

- Corpus `nullable_written_through_borrows` against native Rust: each
  write path above, of a module's variable, of a counted cell, of a field,
  of an item, of a `get_mut()` and of a parameter, each followed by
  `=== null`-tested reads, `is_none`, `if let`, and a nullable function's
  return.
- `test/lowering-nullable-borrows.test.ts`: what a nullable function returns
  to JS after each write is `null`, and the JS each path writes.
- `handle-in-place-write-unstored` (tidy/values.ts) is gone. It survived 170
  nullable, guard and cell corpus cases and `lowering-nullable.test.ts`: a
  nullable field's guard written through, `*f.borrow_mut() = None`, now
  reaches its handle through a variable (`through_handle`), whose writes
  `read_handles` stores. `handles_in_place` still stores a handle put in
  place as its storage says, though no lowering writes one now.
  `settled-target-value-spilled` (mir/assign.ts) survived the same way, as
  `nullable_items`' write goes through its handle. It's held again by corpus
  `item_written_of_its_own_read`, a plain `Vec<Option<u32>>`'s.
- Mutations in `scripts/mutations/lower/mir/nulled.ts`,
  `lower/std_types/cell.ts`, `lower/mir/local_facts.ts`, `js/applied.ts`
  and `lower/mir/tidy/values.ts`.
