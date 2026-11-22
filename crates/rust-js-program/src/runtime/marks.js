// The marks the runtime sets on what it makes, which every copy of the
// runtime reads: a bundled library and the app that uses it may each hold
// one, and a panic one raises is caught, hooked or downcast by the other
// (ADR 1184). Each is a registry symbol, `Symbol.for`, named `rust-js: ..`:
// a mark made by `Symbol(..)` is one copy's alone, which
// `test/runtime-marks.test.ts` refuses except where it names why. Neither
// is enumerable on what it marks, so a host shows it as it is.

// What marks a panic's `Error` as one whose payload is a `String`.
const $stringPayload = Symbol.for("rust-js: a panic whose payload is a String");

// Where a panic was raised, `file:line:column` (#2292), which a binary's
// hook names.
const $raisedAt = Symbol.for("rust-js: where a panic was raised");
