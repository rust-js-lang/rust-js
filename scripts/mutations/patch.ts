// Mutations of tooling/patch.js: how rust-js's own crates' files are named,
// `/rust-js/<version>/tooling/compat/..` on every machine (ADR 0262's
// difference 63, #2438).
import type { Mutation } from "../mutations";

export const mutations: Mutation[] = [
  {
    name: "compat-named-without-its-version",
    breaks: "rust-js's own crates' files are named without the tooling's version, so two releases' js-sys are one name in an app's JS (#2438)",
    file: "tooling/patch.js",
    find: "  return `${resolve(compatDir)}=/rust-js/${version}/tooling/compat`;\n",
    replace: "  return `${resolve(compatDir)}=/rust-js/tooling/compat`;\n",
    tests: ["test/cargo.test.ts", "-t", "rust-js's own crates' panics name their files the same on every machine"],
  },
];
