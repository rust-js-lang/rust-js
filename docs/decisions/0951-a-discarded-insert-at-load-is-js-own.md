# 0951. A map's or set's call nothing reads is JS's own in code that runs at load too

Status: Accepted. Extends [0059](0059-hashmap.md)'s discarded calls,
`$add(s, x);` as `s.add(x)`, from functions' bodies to every block of a
module: its constants' closures and the code `js::on_load!` runs.

Case ([0262](0262-when-rust-and-js-disagree.md)): N. What a statement's
call gives is read by nothing.

## Context

React's `ReactStrictModeWarnings.flushLegacyContextWarning` is a closure
set where the module loads, `uniqueNames.add(name)`. Its port's
`unique_names.insert(..)` inside `js::on_load!` stayed `$add(uniqueNames,
..)`: `discarded_calls` walked functions' bodies alone.

## Decision

**`discarded_calls` walks every block of the module's items**
(`js::each_item_block_mut`): a function's, a constant's closure's, and
the statements that run where it loads.

## What holds it

- `test/lowering-collections.test.ts`, "a discarded insert in a closure
  set at load is the set's add".
- Mutation: `discarded-calls-of-functions-alone`.
