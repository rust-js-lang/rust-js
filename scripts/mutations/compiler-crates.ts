// The members' features a build turns on, of their manifests alone
// (scripts/compiler-crates.ts, #2348): a walk that takes a dependency's own
// dev-dependencies as built, or skips what a feature turns on, says a crate's
// `testing` reaches a normal build where it doesn't, or misses where it does.
import type { Mutation } from "../mutations";

const tests = ["test/architecture-crates.test.ts", "-t", "a crate's `testing` feature never reaches a normal build"];

export const mutations: Mutation[] = [
  {
    name: "member-dev-dependencies-built",
    breaks: "every member's dev-dependencies are taken as built, a dependency's own too, so a normal build of the binary is said to turn on `rust-js-program`'s `testing`, which only the tests' build does",
    file: "scripts/compiler-crates.ts",
    find: '      const kinds = name === root ? rootKinds : [null, "build"];\n',
    replace: '      const kinds = [null, "build", "dev"];\n',
    tests,
  },
  {
    name: "member-feature-through-feature-unread",
    breaks: "what a feature turns on of a dependency, `dep/feature`, is never turned on, so a `testing` reached through another member's feature goes unseen",
    file: "scripts/compiler-crates.ts",
    find: "          if (dep) more = turnOn(dep.name, [of![2]]) || more;\n",
    replace: "          if (dep) continue;\n",
    tests,
  },
  {
    name: "member-root-defaults-unread",
    breaks: "a build's root turns on none of its default features, so what they reach is never counted",
    file: "scripts/compiler-crates.ts",
    find: '  turnOn(root, ["default"]);\n',
    replace: "  turnOn(root, []);\n",
    tests,
  },
];
