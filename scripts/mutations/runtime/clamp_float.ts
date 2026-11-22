// Mutations of crates/rust-js-program/src/runtime/clamp_float.js (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "clamp-f32-site-of-f64",
    breaks: "an `f32`'s `clamp` names `f64`'s line",
    file: "crates/rust-js-program/src/runtime/clamp_float.js",
    find: "$stdSites.clamp[type][0]",
    replace: "$stdSites.clamp.f64[0]",
    tests: ["test/binaries.test.ts", "-t", "std raises"],
  },
  {
    name: "clamp-float-nan-bound",
    breaks: "a NaN bound of a float's `clamp` isn't noticed",
    file: "crates/rust-js-program/src/runtime/clamp_float.js",
    find: "  if (!(min <= max)) {",
    replace: "  if (min > max) {",
    tests: ["test/corpus.test.ts", "-t", "float_clamp_nan"],
  },
];
