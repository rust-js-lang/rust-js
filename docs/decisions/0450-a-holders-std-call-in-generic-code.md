# 0450. A std holder's `Deref`, `extend` and `collect` are written in place whatever it holds

Status: Accepted. Extends [0394](0394-a-deref-bound.md) and
[0445](0445-extend-and-collect-in-generic-code.md).

Case: N ([0262](0262-when-rust-and-js-disagree.md)): the same call, of
the same value.

## Context

Where its type is known, a std trait's call is written in place, as
recognition lowers it, and a dictionary is for generic code (ADR 0180).
What "known" meant was asked two ways. A writer's `write!` and a `Deref`
were written in place where the call had no type parameter at all; a
collection's `extend` and `from_iter` where its own type wasn't one, a
`Vec<T>`'s too (ADR 0445). So `Deref` of a `MutexGuard<C>`, in uuid's
`impl<C: ClockSequence> ClockSequence for Mutex<C>`, asked for a
dictionary of std's `Deref` for the guard, and was refused, though a
guard's `Deref` is its value whatever `C` is.

## Decision

**One function answers whether std's trait call is written in place,
`std_written_in_place`. A holder's, a `Deref`'s, an `Extend`'s and a
`FromIterator`'s, where its own type is known, whatever it holds: not a
type parameter's, nor a `&` to one's. A writer's where its whole type is
known, as before: a writer may write to one it holds.**

```rust
impl<C: Sequence> Sequence for Mutex<C> {
    fn next_value(&self) -> u32 {
        self.lock().unwrap_or_else(|e| e.into_inner()).next_value()
    }
}
```

```js
function mutexSequence_next_value(mutex, CSequence) {
  const value = $lock(mutex, true);
  const value$1 = value.value;
  ...
  result = CSequence.next_value(value$1);
```

## Why

- **One place answers each question** (AGENTS.md): the two conditions
  were one question, written twice, and drifted apart, so a guard of a
  generic type was refused where a `Vec<T>`'s `extend` wasn't.
- **It's what shared code does**: a lock, a `Cow` or a `Box` of a type
  parameter's, in a generic impl, uuid's.
- **It's tested**: a corpus case derefs a `MutexGuard<C>` and a `Cow<'_,
  [T]>` in generic code, against native Rust; uuid compiles; a mutation of
  each side of the function is caught.

## Alternatives

- **A dictionary of std's `Deref` for each guard**: a second way to write
  the same deref, through an object, where recognition writes it in place.
