// Mutations of crates/rust-js-lower/src/analysis/accessors.rs, what reads a `js::Accessor`
// (ADR 1164).
import type { Mutation } from "../../../mutations";

const file = "crates/rust-js-lower/src/analysis/accessors.rs";
const tests = ["test/js-accessor.test.ts", "-t", "an accessor read but by its get"];

export const mutations: Mutation[] = [
  {
    name: "accessor-get-of-a-local-allowed",
    breaks: "`get` of a local's accessor compiles, its JS the function itself where Rust's `get` calls it",
    file,
    find: "            if !of_a_field {\n",
    replace: "            if false {\n",
    tests,
  },
  {
    name: "accessor-field-moved-out-allowed",
    breaks: "an accessor field moved out of its struct compiles, which JS reads the value of there, not where Rust's `get` calls it",
    file,
    find: "                .is_some_and(|place| is_field(place) && is_accessor(tcx, place.ty(decls, tcx).ty))\n",
    replace: "                .is_some_and(|place| false && is_field(place) && is_accessor(tcx, place.ty(decls, tcx).ty))\n",
    tests,
  },
];
