// Mutations of src/packages.rs (ADR 0093).
import type { Mutation } from "../mutations";

const test = ["test/libraries.test.ts", "-t", "entry re-exports"];

export const mutations: Mutation[] = [
  {
    name: "package-entry-default-unread",
    breaks: "a package's root is read of its first condition's entry, `react-server`'s, where an `import` resolves its `default` (ADR 1008, amended)",
    file: "src/packages.rs",
    find: '                let default = conditions.get("default").map(target);\n',
    replace: '                let default = conditions.values().next().map(target);\n',
    tests: test,
  },
  {
    name: "package-entry-unparsed-kept",
    breaks: "a package's entry oxc can't read, Flow's, is taken as re-exporting nothing, the file kept without a word, where it's refused, naming it (ADR 1008)",
    file: "src/packages.rs",
    find: "        let parsed = parse(&code).map_err(|error| {\n",
    replace: "        let parsed = parse(&code).or_else(|_| parse(\"\")).map_err(|error| {\n",
    tests: test,
  },
  {
    name: "package-entry-star-unread",
    breaks: "an entry's `export * from` re-exports nothing, so what it re-exports is imported by its file (ADR 1008)",
    file: "src/packages.rs",
    find: "                starred.entry(export).or_default().insert(module.path);\n",
    replace: "",
    tests: test,
  },
  {
    name: "package-entry-not-plain-read",
    breaks: "an entry that runs code of its own as it loads is imported by the package's name, which runs what the Rust doesn't (ADR 1008)",
    file: "src/packages.rs",
    find: "    if !entry.plain {\n        return found;\n    }\n",
    replace: "",
    tests: test,
  },
  {
    name: "entry-root-unmatched",
    breaks: "a crate's root that re-exports what its package's entry doesn't writes no JS, its re-exports lost (ADR 1008, amended)",
    file: "src/packages.rs",
    find: "            .any(|export| &export.module == from && &export.export == name && &export.entry == alias)",
    replace: "            .all(|export| &export.module == from && &export.export == name && &export.entry == alias)",
    tests: test,
  },
];
