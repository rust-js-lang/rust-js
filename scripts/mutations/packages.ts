// Mutations of src/packages.rs (ADR 0093).
import type { Mutation } from "../mutations";

const test = ["test/libraries.test.ts", "-t", "entry re-exports"];

export const mutations: Mutation[] = [
  {
    name: "package-entry-conditions-one",
    breaks: "a package's root is read of its first condition's entry alone, so an item another condition's entry doesn't re-export is imported by the package's name, which a host of that condition finds another module of (ADR 1008)",
    file: "src/packages.rs",
    find: "                agreed.retain(|key, exported| exports.get(key) == Some(exported));\n",
    replace: "",
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
];
