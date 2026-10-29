# 0617. A parameter is written by its argument

Status: Accepted.

Case ([0262](0262-when-rust-and-js-disagree.md)): results would change,
so Rust's.

## Context

Lowering from MIR asks of a local whether it may read differently where
it's read: one that doesn't is read where it's used, its field
`a._0` in place of a binding made of it, and a `move` closure reads it
rather than its own copy. It counted MIR's writes, and a parameter's
first value, its argument, is none. A `mut` parameter assigned once had
one write, as a `let` written once has, so it was read as never
changing (#99): what a `match` or an `if let` moved out of it read the
new value, `old` of `Some(old) => { name = ..; old }` read `name`; the
textbook list reversal, `while let Some(mut node) = head { head = ..`,
threw; and `move || x * factor` read `factor` after `factor = 5`.
`written_again`, which counts the argument, answered the same question
for references already.

## Decision

**A parameter's argument is its first write.** Whether a local is
written again is `written_again`'s alone, and whether it may read
differently where it's read, `written_apart`'s, `written_again` but for
a variable the program names assigned once each way, which a parameter
never is, its argument written before. What's read as never changing,
the `let`s declared first, and a `move` closure's copy all ask them.

## What holds it

- `test/corpus/mut_param_moved_out.rs`: a parameter moved out of by a
  `match` and by `while let`, then assigned; a `move` closure of a
  parameter, then assigned, a closure's parameter too.
- `test/architecture.test.ts`, "a local written again is written_again's
  alone": no other place counts MIR's writes past one.
- Mutations: `reassigned-param-alike`, `move-capture-of-reassigned-param-shared`.

## Amendment: the same until it's written (#121)

Whether a local reads the same was decided for the whole function, so a
`mut` parameter assigned only after its `match`, `describe(mut shape)`'s
`shape = Shape::Dot;`, had a `const n = shape._0;` in each arm, where
reading `shape._0` in place was right. **A variant's binding of a field
of a variable written again, of plain data, is read in place where the
variable is written on no way from the binding to a read of it**
(`queries::written_before_read`, from `assign::settled_until_read`): no
assignment of it, whole or in part, no call's result put in it and no
`&mut` of it in a block both reached from the binding and reaching a
read. One written between, in its arm or a loop's next turn, is held as
before, and a closure's capture is held where it's made. Case A: the
same reads in the same order. Held by `test/corpus/settled_until_written.rs`
and `guarded_bindings.rs` against native Rust, and the mutations
`settled-until-read-never` (`mir/assign.ts`) and `written-before-read-*`
(`mir/queries.ts`).
