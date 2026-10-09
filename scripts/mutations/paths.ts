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
  {
    name: "output-symlink-unresolved",
    breaks: "an output path is compared as written, so a map that's a symlink to the input isn't seen as the input, and is written over it",
    file: "src/paths.rs",
    find: "        std::fs::canonicalize(path).map_err(|e| e.to_string())",
    replace: "        std::path::absolute(path).map_err(|e| e.to_string())",
    tests: ["test/emission.test.ts", "-t", "an output map symlink cannot overwrite an input source"],
  },
];
