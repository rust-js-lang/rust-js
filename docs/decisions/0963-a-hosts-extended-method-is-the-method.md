# 0963. A host's extended method is the method, and a union's number is the wider of its name's

Status: Accepted. Extends [0811](0811-console-create-task-may-be-absent.md),
what webapi declares that no WebIDL does, to a method a host extends, and
[0215](0215-webapi-unions.md), a union's enum, to two unions of one name.

Case ([0262](0262-when-rust-and-js-disagree.md)): N. A binding is the JS it
calls; the wider number takes every value the narrower did.

## Context

React's performance track (`ReactFiberPerformanceTrack`) logs each phase
with Chrome's DevTools extensibility API, 61 calls of

```js
console.timeStamp(name, startTime, endTime, COMPONENTS_TRACK, undefined, color);
```

guarded once, `typeof console !== 'undefined' && typeof console.timeStamp
=== 'function'`. webapi is generated from WebIDL, whose `console` has no
`timeStamp`; TypeScript's lib declares it of a label alone. And it measures
with `performance.measure(name, { start, end })`, whose times are
`DOMHighResTimeStamp`s, doubles, but `PerformanceMeasureOptions.start` was
`StrOrNumber`'s `Number(u32)`: the generator kept one union of each name,
the last it read, and a `(DOMString or unsigned long)` was last.

## Decision

- **A method a host extends is the method, of every parameter it takes
  there**, written by hand in the generator's `EXTRA` with its docs and
  the host that has it: `console::time_stamp(label, start, end, track_name,
  track_group, color)`, `console.timeStamp`, each optional parameter an
  `Option`, a union's borrowed, `Some(&start)` (ADR 0443). Where a host may
  lack it, `js::is_function(console::time_stamp)` asks, as it asks of
  `setTimeout` (ADR 0942): `typeof console !== "undefined" && typeof
  console.timeStamp === "function"`, React's own test. Not an `Option`
  property, as `create_task` is: that one's read as a value, this one called
  as a method, whose `this` is `console`.
- **Unions of one name are one enum of the wider number** (`named` in
  `webapi/generate.ts`): `StrOrNumber`'s `Number` is `f64`, which each of
  its uses takes, a time's double and a UUID's `unsigned long`, as
  TypeScript's `number` is either.

## What holds it

- `test/webapi-bindings.test.ts`, "console.timeStamp is Chrome's, and a
  measure's times are doubles", run against a host's `timeStamp`.

## Not decided here

React also writes `console.timeStamp.bind(console, ...args)` and
`performance.measure.bind(performance, name, options)`, given to
`debugTask.run`, and reuses one options object it mutates. Rust's closure,
`task.run(move || console::time_stamp(..))`, is `task.run(() =>
console.timeStamp(..))`; `js::function::bind` binds a `this` alone (ADR
0907). And a dictionary is made where it's given; one object a module keeps
and changes needs a dictionary parameter that takes an object of its
fields. Both are left for a decision.
