// Mutations of crates/rust-js-mir/src/nulled/defined.rs (ADR 0093): what a test
// proved defined where lowering reads it, never `undefined` though maybe
// `null` (`Known::ProvedDefined`, #1933 step 4 (D)).
import type { Mutation } from "../../../../mutations";

const defined = ["test/lowering-nullable-tests.test.ts", "-t", "proved defined"];
const classified = ["test/lowering-nullable-classified.test.ts", "-t", "classify arm neither undefined nor null"];

export const mutations: Mutation[] = [
  {
    name: "defined-strict-equals-true-edge",
    breaks: "`js::strict_equals(x, None)`'s true edge, where `x` is `undefined`, is taken for its false one, so `!strict_equals` proves nothing and `return entry ?? null` stays; the other way round, `undefined` given where Rust's `None` is `null` (#1933)",
    file: "crates/rust-js-mir/src/nulled/defined.rs",
    find: "                ((tested(&a.node) && none(&b.node)) || (none(&a.node) && tested(&b.node))).then_some(*no)\n",
    replace: "                ((tested(&a.node) && none(&b.node)) || (none(&a.node) && tested(&b.node))).then_some(targets.otherwise())\n",
    tests: defined,
  },
  {
    name: "defined-strict-equals-any-side",
    breaks: "`js::strict_equals(x, y)` of any `y` is taken for `x === undefined`, so its false edge proves `x` defined where `x` may be `undefined` (#1933)",
    file: "crates/rust-js-mir/src/nulled/defined.rs",
    find: "                let none = |side: &Operand<'tcx>| self.made_none(state, side);\n",
    replace: "                let none = |_: &Operand<'tcx>| true;\n",
    tests: defined,
  },
  {
    name: "defined-classify-undefined-edge",
    breaks: "`Kind::Undefined`'s own edge of a `match` of `js::classify(x)` is taken to prove `x` defined, so `return entry` gives `undefined` where Rust's `None` is `null` (#1933)",
    file: "crates/rust-js-mir/src/nulled/defined.rs",
    find: "                (values.iter().any(|&(v, _)| v == value) && values.iter().all(|&(_, to)| to != otherwise))\n                    .then_some(otherwise)\n",
    replace: "                values.iter().find(|&&(v, _)| v == value).map(|&(_, to)| to)\n",
    tests: defined,
  },
  {
    name: "defined-classify-any-values",
    breaks: "the `otherwise` edge of a switch on `js::classify(x)`'s variant that names no `Kind::Undefined` is taken to prove `x` defined, `matches!(js::classify(x), Kind::Null)`'s false edge, where `x` is `undefined` (#1933)",
    file: "crates/rust-js-mir/src/nulled/defined.rs",
    find: "                (values.iter().any(|&(v, _)| v == value) && values.iter().all(|&(_, to)| to != otherwise))\n",
    replace: "                (values.iter().all(|&(_, to)| to != otherwise))\n",
    tests: defined,
  },
  {
    name: "defined-classify-of-any-local",
    breaks: "a `js::classify` of another value is taken for a test of the local read, so its edge proves a local it never tested (#1933)",
    file: "crates/rust-js-mir/src/nulled/defined.rs",
    find: "            || self.tested_local(state, &given.node, Some(local))? != local\n",
    replace: "            || self.tested_local(state, &given.node, Some(local)).is_none()\n",
    tests: defined,
  },
  {
    name: "classified-present-otherwise-of-any-arms",
    breaks: "the `otherwise` edge of a switch on `js::classify(x)` is taken to prove `x` present where only `Kind::Null` is named, `match js::classify(x) { Kind::Null => None, _ => x }`, so an `undefined` `x` is given as `undefined` where Rust's `None` is `null` (#1933)",
    file: "crates/rust-js-mir/src/nulled/defined.rs",
    find: "        if nullish.len() == 2 && nullish.iter().all(|&v| named(v)) && values.iter().all(|&(_, to)| to != otherwise) {\n",
    replace: "        if nullish.len() == 2 && values.iter().all(|&(_, to)| to != otherwise) {\n",
    tests: classified,
  },
];
