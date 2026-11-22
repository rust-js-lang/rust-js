// Mutations of crates/rust-js-program/src/runtime/marks.js (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "string-payload-one-copys",
    breaks: "a panic's `String` payload is marked by a symbol one copy of the runtime made, so the app's copy reads a library's `panic!(\"n = {n}\")` as a `&str` (ADR 1184)",
    file: "crates/rust-js-program/src/runtime/marks.js",
    find: 'const $stringPayload = Symbol.for("rust-js: a panic whose payload is a String");\n',
    replace: 'const $stringPayload = Symbol("rust-js: a panic whose payload is a String");\n',
    tests: ["test/runtime-marks.test.ts", "-t", "a panic raised by a library's copy"],
  },
  {
    name: "raised-at-one-copys",
    breaks: "where a panic was raised is marked by a symbol one copy of the runtime made, so another copy, a minified app's, reads none (ADR 1184)",
    file: "crates/rust-js-program/src/runtime/marks.js",
    find: 'const $raisedAt = Symbol.for("rust-js: where a panic was raised");\n',
    replace: 'const $raisedAt = Symbol("rust-js: where a panic was raised");\n',
    tests: ["test/runtime-marks.test.ts", "-t", "a panic raised by a library's copy"],
  },
];
