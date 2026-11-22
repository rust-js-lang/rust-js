// Mutations of crates/rust-js-lower/src/library/bound.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "binding-releases-unrefused",
    breaks: "two releases of a binding crate over one npm package build, one calling the other's JS, the one copy the app installs (#1799)",
    file: "crates/rust-js-lower/src/library/bound.rs",
    find: "        if ids.len() < 2 {\n",
    replace: "        if ids.len() < 3 {\n",
    tests: ["test/binding-releases.test.ts", "-t", "over one npm package are refused"],
  },
  {
    name: "binding-releases-of-libraries-unseen",
    breaks: "an app whose libraries import two releases of one binding of a package, each one, builds: their records unread (#1799)",
    file: "crates/rust-js-lower/src/library/bound.rs",
    find: "    let theirs = (foreign.dependencies.libraries.values()).flat_map(|library| library.bound.iter().cloned());\n",
    replace: "    let theirs = (foreign.dependencies.libraries.values()).flat_map(|library| library.bound.iter().cloned()).take(0);\n",
    tests: ["test/binding-releases.test.ts", "-t", "over one npm package are refused"],
  },
  {
    name: "aliased-binding-releases-one-package",
    breaks: "an npm alias, `dom-v0`, is taken for the package it aliases' name alone, its releases refused though they're two packages (#1799)",
    file: "crates/rust-js-lower/src/library/bound.rs",
    find: "        false => Some(first),\n",
    replace: "        false => Some(\"@floating-ui/dom\"),\n",
    tests: ["test/binding-releases.test.ts", "-t", "an alias, are two packages"],
  },
];
