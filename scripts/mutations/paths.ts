// Mutations of src/paths.rs (ADR 0093).
import type { Mutation } from "../mutations";

export const mutations: Mutation[] = [
  {
    name: "specifier-parent-kept",
    breaks: "a link's `../` is a directory named `..`, not the parent, so a nested module's link names no file",
    file: "src/paths.rs",
    find: '            ".." => {\n                path.pop();\n            }',
    replace: '            ".." => {}',
    tests: ["test/manifest.test.ts", "-t", "links are each relative specifier"],
  },
];
