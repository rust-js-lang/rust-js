# 1190. A type's boundary contract is one classification

Status: Accepted (2026-11-23, step 1 of #2403, the user's decision to check
values at the boundary). Amends [0196](0196-typescript-declarations.md):
what a declaration prints of a type is its contract's kind.

Case ([0262](0262-when-rust-and-js-disagree.md)): none. This changes who
answers, not what's answered: every corpus case's `.d.ts` is byte-identical,
and so is every declaration test's.

## Context

The user decided #2391's open case (#2403): every value JS gives Rust is
checked against its Rust type, so Rust never sees a `u32` of `-1` or a
variant no `match` arm has. A check and the `.d.ts` say the same thing,
what a value of a type is to JS, and must never disagree. A `.d.ts` that
says `Map<string, number>` while the check wants a `Set` would refuse
what TypeScript allows.

The `.d.ts` came from `Declarations::ts`, one match over `ty.kind()` that
decided each kind and printed it in the same step, with what's imported
and declared on the way. A check couldn't ask it, since what it prints,
`number`, doesn't say `u8` or `u32`.

## Decision

**What a value of a type is to JS is its `Contract`, one classification,
made by `Contract::of` (`declarations/contract.rs`).**

- Each kind is asked of its owner, in the order the declarations asked:
  - `Num::of`, a number of its kind;
  - the lang items, `Option` and `String`;
  - std's types, by `is_std_def`;
  - the `Rc`s counted and the `RefCell`s held as their value, the analyses'
    `Facts`;
  - a binding's written type, `written_types`;
  - a library's contract, `Foreign::declared_type`;
  - and the crate's own, `Local`.
- **`Declarations::ts` matches on it.** Each arm prints what its old branch
  printed, and imports or declares on the way as before. A local type's
  printing is `local_ts`.
- **#2403's checks will compile the same `Contract`**, so a check and a
  declaration can't disagree about a kind. A variant carries what a check
  needs and the declaration doesn't print: `Num`'s kind, `Char` apart from
  `Str`.

## Why

- **One answer to "what is it to JS".** A check written as a second match
  over `ty.kind()` would drift from the declarations one arm at a time, as
  a `HashSet` of tuples, a `$KeySet`, already shows (a separate issue).
- **Classifying is what both need; printing isn't.** Imports, private types
  declared on the way and aliases written in the type are the printer's,
  and stay there.

## Alternatives

- **A tree of contracts the declarations print whole:** a rewrite of every
  printer, its state threaded through a tree. The classification alone
  gives the checks what they need, with no output at stake.
- **The check reads the printed TypeScript:** it can't, as `number` is every
  width.

## Consequences

- A new kind of value is a `Contract` variant first, which both the printer
  and the checker must match: the compiler holds them to it.
- `declarations.rs` loses the classification (1258 lines, from 1321), and
  `contract.rs` is its own file.

## What holds it

- Every `test/declarations*.test.ts`, unchanged and passing, and the corpus
  and the React port's `.d.ts`, byte-identical (`check:fast --all`).
- The mutation `contract-map-key-value-swapped`
  (`lower/declarations.ts`).
