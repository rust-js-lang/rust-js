# 1129. A fact of a library's contract says what makes it true

Status: Accepted (2026-11-12, #1462, of the architecture review of that
day; m6-react, coordinating the other agents, had no objection). Beside
[0100](0100-separate-crates.md), the contract, and
[0906](0906-a-librarys-facts-are-sets.md), its facts as sets.

Case ([0262](0262-when-rust-and-js-disagree.md)): N. A page and a test:
no program's JS changes, and no fact of the contract.

## Context

The review asked for one canonical representation at every library
boundary, so no closed-world fact crosses it. Sorting the contract's
fields (`src/library.rs`, ABI 29) by what makes each true gave three
kinds: 16 of linkage and declared interface, 24 that summarise one
function, impl or item, and 26 true only of what the library's whole
crate does or never does. Those 26 are optimisations: `unboxed`,
`nulled_params`, `keeps_capacity`, a writer's options, which `Rc`s are
counted, which `RefCell`s are unchecked.

Making them canonical was weighed and not taken. A writer that always
takes its options, a `Some` always boxed at an export, capacity always
kept, cost every library's JS what only some programs use, against
"readable JavaScript out". And each already has what makes it sound: a
consumer that differs compiles the function itself (ADR 0914), agrees,
or is refused; and the consumer net and the metamorphic net's `library`
rewrite of every corpus case run each night.

What was missing is that nothing said, of a new fact, which kind it is.
The 2026-11-09 audit's six wrong answers were closed-world facts answered
apart by a library's compile and its consumer's, each added without a
stated fallback.

## Decision

**Each fact of the contract is listed in
[docs/library-contract.md](../library-contract.md) with its kind,
`linkage`, `summary` or `closed-world`, and a `closed-world` one with
what a consumer that differs does. A fact without its row fails
`test/manifest.test.ts`.**

- The page's table is every field of `Library`, `Calling`, `TypeParam`
  and `RefCellFacts`, and every variant of `ItemFact`, `ParamFact` and
  `FieldFact`; the test reads both and holds them equal, each row's kind
  one of the four, and each `closed-world` row's last column filled.
- A `closed-world` fact is allowed where that column is true: "compiles
  its own", "agrees or is refused", never "trusts it".
- The associated types' `Size` and `Align` (`Library.associated`, #1420)
  and `RoomSize` (`TypeParam.facts`, #1260) are `summary`: evidence a
  call gives or a dictionary carries, the same whoever calls.
- A new fact is m6-react's to see before it lands (#1462).

## Consequences

- The contract is no smaller, and no consumer recompiles less. What
  changes is that a fact's closed-world nature is said where it's added,
  with its fallback, and read by whoever reviews it.
- The page is the list to read before changing how a library and its
  consumer agree, as `generic_facts.rs`' catalogue is for generic code.
- If a closed-world fact is later made canonical at exports, its row
  moves to `summary` or goes; the test follows the code either way.
