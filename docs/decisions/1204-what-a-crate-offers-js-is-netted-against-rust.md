# 1204. What a crate offers JS is netted against Rust

Status: accepted (2026-11-25, the lead's decision on Tu's behalf, #2481)

## Context

83% of the JS boundary's P0s were found by hand (#2481's read of 455 P0s).
No net gave the compiled code JS's own values or asked whether what Rust
offers JS is there. #2544 was one: `pub mod sub { pub use dep as d; }` and
the root's `pub use sub as s;` compiled, and JS had no `s`, with nothing
said. The writer decided both what to write and what was written, so no
check could see the drop.

## Decision

**The build manifest says the paths a crate offers JS**, `offered_paths`,
an additive field (no version bump), public, which a host may read:

- each `pub` item of the crate's root and each of its `pub use`s, by the
  name the `use` gives it;
- through each module re-exported, the crate's own or a library's that has
  JS, by name or by a glob, what that module offers in turn;
- each with its Rust path, the path JS reads it by, and its kind: a
  function, a `const`, a `static`, a namespace or a type. A function has
  its parameters' and its result's types, a `const` or a `static` its own,
  as a tree of plain forms (`OfferedType`), any other by its Rust name.

Not a module's own `pub mod`: only Rust names it (#2403, lever (3)). Nor a
library's module of no JS, of types alone or a binding crate's.

**They're found of rustc's `module_children` alone** (`offered.rs`), apart
from what writes the JS and its `.d.ts`. Their record types are a module of
their own, `rust_js_program::offered`, which the front end writes and the
manifest carries, as neither phase's; `tooling/manifest.js` validates them. A path the writer drops is then a
path the JS lacks, not one both agree is gone.

**The export net** (`test/export-net.ts`, nightly) holds each crate's JS
and `.d.ts` to them:

- **existence:** each value path is in the JS, of its kind (a function a
  function, a namespace an object), a namespace where it offers a value;
- **declarations:** each value and type path is read through the `.d.ts`
  by `tsc`, which refuses one it lacks;
- **oracle A, native Rust:** each function of plain types, called with
  values at their types' bounds and inside (`test/generated-calls.ts`),
  and each `const` and `static`, gives what native Rust's does, printed
  alike by both sides (a float by its bits, a string by its code points);
- **oracle D:** no JS exception but a Rust panic escapes an export.

It runs over its own cases (`test/export-net/`), a crate alone and a
library with the app using it, and the crate pairs' table, each crate
compiled as a library with its declarations. A case of its own checks
something of each kind, so it never passes having checked nothing.

## Consequences

- The value generator and printers are one module, `generated-calls.ts`,
  which declarations-conformance reads its signatures through next, its
  regex parse of the source gone (#2573): one source of signatures.
- Later, each its own landing (#2481): oracle B (each result against its
  declared type), oracle C (counting proxies, volatile getters,
  `--js-reads`), binding stubs, the corpus as libraries; and a class's
  members, which v1 doesn't offer.

## What holds it

- `test/export-net-1.test.ts` to `-6` (`exportNetParts`), each the tests
  test/export-net-titles.ts puts in its part: the own cases and the pairs;
  the offered paths of #2544's shapes, exactly. A run of the values
  generated is given 15 s, as the load makes it, before it's run again of
  small values alone.
- Put back by hand, #2544's bug fails the net: each path through a
  dropped re-export is missing.
- Mutations of `offered.rs`: `offered-namespace-unwalked`,
  `offered-alias-ignored`.
- Its first catch, on the crate pairs: a `pub use` of the crate's own
  static wrote no re-export, so a library root of a module and that `pub
  use` wrote no entry at all (#2581, fixed before this landed).

## Amendment: declarations-conformance reads its signatures from the records (2026-11-25, #2573)

declarations-conformance (ADR 1091) read each exported function's signature
by a regex parse of its case's source, a second authority beside the
records, which could disagree with them silently. It now reads the records.
Its source parser is gone, about 250 lines, and its cases, call counts and
`None` forms are unchanged: 36 cases, `None` given 23, 33 and 20 times, of
16 cases.

Its coverage needed four additions to the records, all additive in the
manifest's contract and validated by tooling/manifest.js:

- **`OfferedType::Ref`'s `mutable`,** and the type forms `Box`, `Fn` (of a
  callable bound or a `dyn Fn`), `Param` and `Named`, the crate's types by
  their paths.
- **A struct's `fields`,** each by its Rust name, its JS name, its type and
  its type as written; and whether it's `sealed`. The JS name is
  `representation::variant_field`'s, the lowering's naming owner's, never
  re-derived.
- **A generic function's `bounds`,** each type parameter's traits, its
  callable form, and of a trait of the crate's, each impl rustc has of it
  (`all_impls`), by the type it's of and its dictionary's JS name,
  `traits::impl_name`'s.

A struct with a field a JS symbol names, a binding's (ADR 1051), records
no fields and says why, `unoffered_fields: "symbol_keyed"`, so no net takes
its silence for no fields. Its fields' own key form, a symbol key beside
the string `js`, which the nets would construct and check, is still to come
(#2583, P3).
test/type-parameter-reads.txt counts offered.rs's two reads of a type
parameter. They describe a signature for its callers, as declarations.rs'
reads do, and make no JS form of generic code.

Each case compiles twice: once for its records, which say its sealed
structs, then with their makers. A type's name is rustc's answer, so a
case's own `String` is its own type (#1875) by construction, and the test
of that reads the records.

Mutation `offered-type-leaf-flipped`, a `bool` recorded as a `u8`, is caught
by both nets. `offered-dyn-fn-unread`, `offered-symbol-field-named` and
`offered-symbol-keyed-unsaid` hold the `dyn Fn` reading and the symbol-named
fields.

## Amendment: oracle B, each value's JS shape (2026-11-25, the lead's design)

Oracle A compares what JS prints with what native Rust prints. Its printer
is lenient, so a value of the wrong JS shape passed wherever the printout
agreed: a `u64` as a number, a `Vec` as a typed array. And a struct's or an
enum's result, which the printer can't print, was never called at all.

**Oracle B holds each value the net reads to its record's type, by the
guide's types table.** `conforms` (test/generated-calls.ts) checks:

- **Numbers:** a number, an integer in range, of 32 bits or less; a BigInt
  of 64 and 128; an `f32` that `Math.fround` leaves alone.
- **Text and unit:** a `char` is one code point, a string is a string, and
  `()` is `undefined`.
- **Collections:** a `Vec` or slice is an array (`Array.isArray`), and a
  tuple is an array of exactly its length.
- **Pointers and functions:** a `Box` or `&T` is its value, and a `Fn` is a
  function.
- **An `Option`** is held to its place's `None` form.
- **A struct of recorded fields** is an object of its fields' JS keys, each
  conforming. A `#[repr(transparent)]` struct is its field.

Each shape names its guide row, `shapeRows`. test/oracle-b.test.ts finds
each row in docs/guide/interop/types.md as written, and checks the checker
on right and wrong values. That file runs on every push.

**What B can't decide, it counts as unchecked rather than passing it
silently:**

- enums, for which there are no variant records yet;
- a tuple struct's fields;
- a `&mut`'s box;
- an `Option` of what may itself be `undefined`, whose `Some` is `$some`'s
  box, a form the guide's table doesn't give.

Each own case of the net leaves none unchecked. Across the net's 220 cases,
B checks 521 shapes and leaves 173 unchecked.

**The facts the records add.** All are additive, and `parseManifest`
validates them:

- **`OfferedField::none`, a field's `None` form.** It comes from the `None`
  owner's one answer, `representation::none_form` of `NonePlace::Field`,
  with `#[rust_js::present]`'s key axis. The forms are `plain`, `present`,
  `null`, `null_optional`, `nested` and `held`.
- **`OfferedPath::result_none`,** a function's result's form, from
  `none_form` of `NonePlace::FnResult`.
- **`OfferedPath::def`,** where a struct, an enum or a union is defined:
  its crate's canonical name (`library::canonical_crate_name`) and its path
  there. `OfferedType::Named` names a type by the same key wherever it's
  offered, a dependency's type in its app's signatures too.
- **`OfferedPath::transparent`,** a transparent struct's field type, from
  `recognition::transparent_field`.

A plain field's `None` is a key left out where it's known there, or a key
of `undefined` where it's computed. ADR 0280 decided this (case A), and B
accepts both.

The net now calls a function whose result the printer can't print, for its
shape alone (`shapedOnly`), with no native line. Such a call throwing a JS
exception that's not a Rust panic is oracle D's problem, as before.

Own case `struct_shapes`: a field of each `None` form, a literal-`None`
maker, a transparent struct, a struct of structs.

**Mutations:**

- `offered-nullable-said-plain` and `offered-transparent-unsaid` mutate B's
  facts.
- aggregates.rs's `present-key-left-out`, a shape the printout can't see,
  is caught by the net's `struct_shapes` too.

## Amendment: enums' records (2026-11-25, the lead's next landing)

Oracle B counted an enum's value unchecked, as the records held no
variants. Each enum's record now says how JS holds its values
(`OfferedPath::variants`). Each fact is `bindings`' answer, as the `.d.ts`
writer asks it:

- **Its form:** `tagged`, `declared` (`#[rust_js::tag]`, `declared_tag`),
  `untagged` (`is_untagged`) or `keyed` (`Flattened::keyed`, which holds a
  library's keyed enums too).
- **Its tag's key** (`tag_key`).
- **Each variant's tag** (`variant_tag`: its name, its number under a
  `#[repr(u8)]`, or a discriminated union's `true`/`false`). An untagged
  variant of no fields has its value instead (`untagged::unit_value`).
- **Each variant's fields,** by their JS names (`variant_field`) and
  `None` forms, and whether it's an `otherwise` (`is_otherwise`).

`Named` now carries its generic arguments by index, and `Param` its index,
so B checks `Held<u64>`'s payload as a `u64`.

**What B checks, by the guide's rows,** which `shapeRows` holds:

- **Tagged:** a fieldless variant is its tag, and a variant with fields is
  an object of its tag and its fields.
- **Declared tag:** each variant is an object of its tag, and an
  `otherwise` is the object it holds.
- **Untagged:** one variant's payload or literal. Where no variant fits and
  one can't be decided, it's counted unchecked.
- **Keyed:** an object of one variant's fields.

**Results:** across the net, 607 shapes checked and 169 unchecked, with no
problem found. The unchecked are now std's types and closures, which the
next landing records: `Rc`, iterators, `dyn` traits, `Result`, cells,
`impl Fn`. Their per-case reasons come back as `shapes.why`.

Own case `enum_shapes`. Mutations `offered-untagged-said-tagged` and
`offered-variant-tag-unsaid`.

## Amendment: std's types and closures (2026-11-25, the lead's call)

After the enums, oracle B's unchecked shapes were std's types and closures:
`Rc` (52), iterators (56), `dyn` traits (27), `Result` (7), `impl Fn` and
`fn` pointers (7), cells (4) and an array. The records now carry each form,
every one of an owner's answers, and B checks each against its guide row:

- **`Result`:** `{ TAG: "Ok" | "Err", _0 }`.
- **`[T; N]`:** an array of exactly `N`.
- **A `fn` pointer and an `impl Fn*`:** a function. The opaque type's
  bounds give its inputs and output.
- **A `dyn` trait:** `{ value, impl }`, its value left out where it's
  `undefined`, a unit struct's (ADR 0280). `dyn Error + Send + Sync` follows
  the same row.
- **`Rc`:** the value itself, or `{ value, strong, weak }` where the crate
  counts it. That fact is the instances' answer, `Counted::counts`, never
  re-derived.
- **`Cell` and `RefCell`:** `{ value }`. A `RefCell` held as the object it
  holds is that object (`RefCells::is_value_cell`).
- **A cell field:** the property itself (`OfferedField::property`). Its rule
  moved to a `tcx`-level `representation::property_of` that
  `FnCx::property_cell` and the records both ask, with its `refcell-field`
  decision recorded there.
- **An `impl Iterator` given back, or std's own, `Chars`:** a JS iterator,
  its `back()` of an `impl DoubleEndedIterator`, its `len()` of an `impl
  ExactSizeIterator`, by the opaque type's bounds and their supertraits.
  The guide's types table had no row for it; the row is added, as the
  export guide's Iterators section states.

`recognition::exact_size_iterator_trait` asked a diagnostic item std doesn't
give `ExactSizeIterator`, so it found none, ever. That also meant a held
`Enumerate`'s back never followed a library's compiled `len` (ADR 0871). It
finds the trait by its path now. The corpus's 197 cases of `enumerate()`
run unchanged, plain and under every iterator form. `DoubleEndedIterator`
is a `StdItem`.

**Results:** across the net, 845 shapes checked and 26 unchecked, with no
problem found. The unchecked left are tuple and unit structs, `Weak`,
`NonZero`, `HashSet`, `NonNull` and `Atomic`.

**Coverage elsewhere:** declarations-conformance reads the new kinds as
named types, of which only a `None` is given. Its `None` forms are given 25,
35 and 22 times, of 17 cases, more than before.

Own case `std_shapes`.

**Mutations:**

- `offered-rc-counted-unsaid`, `offered-refcell-itself-unsaid` and
  `offered-cell-property-unsaid` mutate the records' facts.
- `exact-size-iterator-unfound` mutates the lookup.
- `once-field-boxed` follows its line to `property_of`.

## Amendment: oracle B's full coverage (2026-11-25, the lead's ask)

The last 26 unchecked shapes are recorded and checked, each against its
guide row. The types table gains four rows, its text that of
semantics.md's table and the pointers' sections:

- **`NonZero<T>`:** its number, as `T`'s, never 0.
- **`Weak<T>`:** what its `Rc` is, the value itself or its counts' object,
  by `Counted::counts` of the same pointee; `null` of `Weak::new()`.
- **A raw pointer or a `NonNull<T>`:** a handle whose `value` is its
  referent (`pointers::raw_pointee`, the one answer of what's a raw
  pointer). A raw one may be `null`; a `NonNull` never is. B tests the
  `value` key without reading it, as a read may expose an address.
- **An atomic:** `{ value }`, as a `Cell` is.

These use rows the table already had:

- **`HashSet` and `BTreeSet`:** a `Set`. A `$BTreeSet` extends `Set`.
- **`HashMap` and `BTreeMap`:** a `Map`, each key and value conforming.
- **A tuple struct:** its fields' array (`OfferedPath::positional`), not a
  class's or a transparent one's. Its `numbered_struct` is the
  representation's.
- **A unit struct:** `undefined`, or its `#[rust_js::name]`'s value
  (`OfferedPath::unit`, `bindings::unit_value`).

**Depth and sharing.** B holds each object to a type once (`count.seen`),
so a cycle ends and a value shared across a structure is checked once. Its
depth guard is 1,000 places, which now bounds only recursion. At 8 it had
counted a 30-node list's tail unchecked.

**Results:** across the net's 223 cases, 986 shapes checked and none
unchecked, with no problem found. Every case is now held to none unchecked
(`shapes.why` empty), the crate pairs too.

Own case `rest_shapes`.

**Mutations:** `offered-unit-value-unsaid`, `offered-positional-unsaid`,
`offered-weak-counted-unsaid` and `offered-raw-pointer-never-null`.
`offered-rc-counted-unsaid` names its `Rc` arm, now that a `Weak`'s shares
its line.

## Amendment: oracle C, objects and proxies (2026-11-25, the lead's step 1 of 2)

Until now the net gave only plain values: numbers, text, `Option`s,
arrays. No object JS made reached an export, so a function taking a struct
was never called. Oracle C's first landing:

**Objects from the records.** The one generator (`values`) makes each of a
crate's types from its record, a JS literal of its guide row's form beside
the Rust literal of the same value:

- a struct's object of its fields' JS keys, a nullable field's `None` its
  `null`;
- a tuple struct's array, and a unit struct's value;
- each enum variant in its form, a tag or an object of its tag, an
  untagged one's payload, a keyed one's fields.

A type's Rust literal names it by the path a crate offers it at, the
first crate's. Oracle A then calls the functions that take them and
compares them with native Rust.

**Not given, and said.** A type the generator makes no value of is counted
per case by why (`Netted.notGiven`) and printed, never dropped silently:

- sealed, which no other crate makes (6 across the net);
- a `#[rust_js::class]` (3; `OfferedPath::class`, `classes::is_class`);
- a struct of a cell field that's its property (19);
- also generic, unnamed, an `otherwise`, and a nested `None`.

Makers for these are #2594 (P3).

**The proxies variant.** The harness runs again in a fresh process, each
argument a counting proxy (`test/counting-proxies.ts`, getter-reads'
harness moved out to be the one both nets use). Its lines must equal the
plain run's. The proxies' log is kept for the second landing, `--js-reads`
and the volatile variant.

**Results:** across the net's 224 cases, 422 calls, about 30 more of them
of objects, with no problem found. Own case `object_arguments`: structs by
value and by reference, a nullable field, a tuple struct, a unit struct,
and each enum form.

test/oracle-c.test.ts, the push suite's, holds the generator's forms and
reasons and the proxies' log.

Mutation `offered-field-js-name-as-rust`: a field recorded by its Rust name
gives JS `{ max_len }` where it reads `maxLen`, and native Rust disagrees.

## Amendment: oracle C, Rust's reads and volatile getters (2026-11-25, the lead's step 2 of 2)

Oracle C's second landing holds the reads a crate makes of what JS gives
it to Rust's, on ADR 1022's own contract, so there's no second meaning of
"a read".

**The reference build.** `--js-reads` is a test hook, plumbed as
`--count-every-cell` is, and `buildFlags.reads` in test/builds.ts. With
it, `representation::fields_answered_by_getters`, the one place a field
read is decided, answers true of every struct. Every field read is then
`Known::Getter`, which ADR 1022 guarantees each pass keeps where Rust reads
it. The net builds each crate twice, and runs the reference build given
counting proxies. Its log of reads is Rust's.

**The reads oracle.** The readable build's proxy log must equal the
reference's, per call and read for read. A difference is a read
`js_may_make` or `js_may_give` missed, or one a pass moved, merged or
read again.

**The volatile variant.** Each primitive field the reference reads exactly
once is given as a getter of two answers: its plain value first, another
after. The output must equal the plain run's, as a second read would see
the second answer.

**Its blind spot.** A read the lowering itself emits twice appears in both
builds, so it equals itself and this oracle can't see it. getter-reads'
hand-written `//@ logs` stay that class's oracle (#1908's kind). A MIR
counter would see it, but it needs the printer to tell a read from a
write's target; it earns that change if a P0 of the class turns up.

**Where the reference build can't be made,** a case's reads aren't
compared, and why is said (`Netted.unread`), never passed silently:

- `library_identities_kept_apart` is refused under the hook (#1343's
  `dyn Fn` addresses in a crate that counts an `Rc`).
- `library_writes_a_dyn_errors_debug` and
  `library_writes_an_io_error_piecewise` panic in `optional_call`, where a
  dropped `Box<dyn Error>` field's getter read is a callee asserted to
  have no effects: #2596 (P1). A struct that `js_may_make` holds could
  take the same path in a normal build.

**Results:** across the net's 226 cases, 952 reads compared and 93 fields
given two answers, over 449 calls, with no problem found.

Own case `object_arguments` gains `kept`, ADR 1022's shape: a `let` of a
field Rust reads once, which a plain-data reading inlines at each use.

Mutation `js-given-fields-read-as-plain`: `js_may_give` unasked, so tidy
inlines `const x = p.x`; the reads differ from Rust's, and the volatile
getters' second answers show.
