# 1186. A helper call is read by its declared parameter names

Status: Accepted (2026-11-23, #2334; the lead approved the design: one
owner of the helpers' signatures, one named view of a call, the arity's
meanings made explicit, a contract test and a ratchet).

Case ([0262](0262-when-rust-and-js-disagree.md)): none. This changes how
the compiler's passes read the calls it writes, not what the JS does.

## Context

The compiler's passes find a runtime helper's call and read its
arguments: a simplification of `$withBorrow(key, f)`, a `$lent(it)`
lent as it is, a `$fieldCell(o, "f")` read as `o.f`. Each read them by
their shape: `[cell, raised]`, `args.len() == 2`, `args[4]`. #2292 gave
85 helpers a location argument, `raisedAt`, and four passes stopped
matching, each found as a wrong or lost form. m1-lisa's audit of the 156
uses of `helper_name()` found about 80 reading a fixed shape. A helper
that gains a parameter, or a call that gives an optional one, breaks
each of them silently.

Step 1 checked the audit's eight live sites: one was a miss, and its cause
was a location wrapped as `ExprKind::RaisedAt` that `same_path` didn't look
into. Skipping `raisedAt` there would have been wrong: `v[0] = v[0] + 1`
indexes one place at two columns, and Rust reads the right one first, so
its panic names that column. The other seven read an arity that means
something: a `$lent` given its `kind` owns what it lends where it's
`"boxed"`; a `$fieldCell` given `nullable` writes `None` as `null`; an
`$orInsertWith` given its drops must drop them.

## Decision

**The runtime's helpers' declarations are read once, of their sources**
(`runtime/signatures.rs`): each `$` name a helper declares, the helper
that declares it, and of a function its parameters in order, each its name
(or its pattern's text), whether it has a default and whether it's a rest.
The parse reads a parameter list across lines, around brackets and
strings, `$padAdapter(f, state = { onNewline: true })`. `helper_of`,
`exports`, `location_at` and `location_where_asked` answer of that one
table; `package_module` and `imported_helpers` name what a source declares
with its tokenizer.

**A call is read by those names** (`js::HelperCall`):
`e.helper_call_of("$lent")?.given("kind")`, `.arg("cell")`,
`.args_from("items")`, `helper_call_of_mut(..).arg_mut(..)`, and
`helper_call_of_any(&[..])` of one of a few. The helper is named where its
parameters are, so a test can check them. A name the helper doesn't
declare is a bug, said where it's asked. `raisedAt` is a parameter like
any other. Where calls are compared, the location is compared as a value,
never skipped.

**An argument a call is padded with isn't given**: `call_located`'s
`undefined` before a `raisedAt` it asks for carries `Known::Padded`, the
fact its producer sets, so `$count(it, undefined, "lib.rs:4:9")` gives no
`drop`. An `undefined` a call gives deliberately stays given.

**An arity that means something is asked by name:** `given("kind")`,
`given("nullable")`, `given("drop")`, `given("offset")`, never a count.

**Held by two tests:**
- `js::helper_call`'s unit test: each name a call is read by, near the
  `helper_call_of` that names its helper, is one that helper declares, of
  every compiler source. A renamed JS parameter fails there.
- `test/helper-calls.test.ts`: each file's reads by `helper_name()` are at
  most `test/helper-shapes-known.txt`'s, a ratchet that migrating a file
  lowers, and stated exactly.

## Alternatives

- **Facts carried on the call for each question.** They'd be a second copy
  of what the arguments say, which drifts; a fact is kept only where the
  question isn't about an argument (`Known::Padded`, `Known::Momentary`).
- **Shape matches kept, with the four fixes.** The next parameter breaks
  the next pass, found again as a wrong form.
- **A text exception:** `rust-js-emit`'s `to_oxc/lines.rs` finds a
  branch's operator in its printed line, as the printer gives no offset;
  it stays text, the ratchet's one listed reason.

## Consequences

- Migrating the remaining sites, by file, keeps the corpus, std and the
  React port byte-identical except where a miss is fixed, and lowers the
  ratchet.
- A helper's JS is the one place its parameters are declared; the
  compiler's passes, its location argument and its exports read them
  there.
