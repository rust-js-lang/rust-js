# Bytes read through a pointer, and a pointer's offset

Status: Draft; awaiting the user's decision. Would amend
[0407](../0407-a-raw-pointer-is-a-handle-on-its-place.md), whose case D
refuses "a pointer read as another type's" and "an address".

Case: C ([0262](../0262-when-rust-and-js-disagree.md)) for what's read
and for offsets within one allocation, which wasm32 defines exactly; D
kept for an absolute address, which no JS value is.

## Context

The crates still blocked, by their first refusals, are blocked on
addressable memory (`docs/crate-corpus.md`):

| Crate | What it does | Exactly definable on wasm32? |
|---|---|---|
| aho-corasick (regex) | reads a `u16` or `u32` from a `*const u8` into a byte slice, `read_unaligned`; compares pointers into one slice; `ptr as usize` for offsets | yes: little-endian bytes, positions within one array |
| memchr's `memmem` | the same kind of reads | yes |
| semver | a pointer's address, packed with a tag in its low bits | no: the address itself |
| anyhow | a `*const dyn Error` erasing the error's type, then cast back | not by bytes: a vtable |
| once_cell | a queue of waiters linked by address, tagged in the low bits | no |
| url (yoke) | lifetimes erased through raw pointers | not by bytes |
| time (powerfmt) | a pointer to a slice of `MaybeUninit<u8>` written then read | yes: bytes in one array |

Most of the first group reads bytes as wider integers within one
allocation, which wasm32 defines bit for bit. Rust-js's byte slices are
JS arrays of numbers, and a pointer into one is already a handle with a
position (ADR 0407).

## Proposal

**A pointer into an array of integers reads and writes it as wasm32
would: `p.cast::<u32>().read_unaligned()` is the four bytes from its
position, little-endian; `p.cast::<[u8; 3]>().read()` is the three; a
write stores the bytes back. Pointers into one allocation compare and
subtract by position, and `p as usize` is its position plus a base the
allocation is given, a multiple of 16, so offsets and alignment within
it are Rust's. Everything else of 0407's D stays refused.**

- **One owner**: `pointers.rs`'s handle gains `at`, its position in its
  array, which `add`, `offset_from`, comparison and these reads ask.
- **Only arrays of integers.** A pointer into an array of objects read
  as another type is still refused: no bytes back it.
- **An address across allocations**: two allocations' bases differ by
  more than either's length, so `<` between them is consistent, but no
  program may print one: `{:p}` stays refused (ADR 0407).

## Open gates

1. **Whether a base address is a difference a program can observe.**
   `ptr as usize % 8` is Rust's only if the allocation is aligned as
   Rust's allocator would align it. Choosing 16 makes every alignment
   check pass where Rust's might not. This is case B at best, and needs
   the user's call.
2. semver's tagged pointers, once_cell's queue, anyhow's erased type and
   yoke's lifetimes are out of this draft: each needs its own decision
   or a qualified replacement (ADR 0529's contract).
3. Performance: a `u32` read assembled from four array reads is slower
   than the byte loop it replaces. That's acceptable only where it's a
   library's internal fast path.

## What would hold it

- Corpus cases reading `u16`, `u32` and `[u8; N]` unaligned from byte
  slices at every offset, writing them back, comparing and subtracting
  pointers, against native Rust.
- rustc's tests under `ptr/` and `unsafe/` that become in scope, listed
  by `bun run test:rustc`.
- aho-corasick's `is_equal_raw` run against native Rust through a
  probe.
