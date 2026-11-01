// Mutations of src/packages.rs (ADR 0093).
import type { Mutation } from "../mutations";

const test = ["test/libraries.test.ts", "-t", "entry re-exports"];
const subpaths = ["test/libraries.test.ts", "-t", "package's subpath"];

export const mutations: Mutation[] = [
  {
    name: "package-entry-default-unread",
    breaks: "a package's root is read of its first condition's entry, `react-server`'s, where an `import` resolves its `default` (ADR 1008, amended)",
    file: "src/packages.rs",
    find: '            let default = conditions.get("default").map(default_target);\n',
    replace: '            let default = conditions.values().next().map(default_target);\n',
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
  {
    name: "package-subpath-any-target",
    breaks: "a module is imported by its package's subpath where `exports` resolves it to another file than its JS, `./src/*`'s `./src/flags`, which Node then doesn't find (ADR 0770, amended)",
    file: "src/packages.rs",
    find: "    (target == file).then_some(bare)\n",
    replace: "    Some(bare)\n",
    tests: subpaths,
  },
  {
    name: "package-subpath-pattern-shortest",
    breaks: "of two `exports` patterns a subpath fits, the one whose part before its `*` is shorter is taken, where Node takes the longer (ADR 0770, amended)",
    file: "src/packages.rs",
    find: "        .max_by_key(|(order, _, _)| *order)?;\n",
    replace: "        .min_by_key(|(order, _, _)| *order)?;\n",
    tests: subpaths,
  },
  {
    name: "package-subpath-exact-key-unread",
    breaks: "a subpath `exports` names by its own key, `./src/flags`, is resolved by a pattern alone, another condition's, so the module is imported by its file (ADR 0770, amended)",
    file: "src/packages.rs",
    find: "    if let Some(value) = exports.get(subpath).filter(|_| !subpath.contains('*')) {\n",
    replace: "    if let Some(value) = exports.get(subpath).filter(|_| false) {\n",
    tests: subpaths,
  },
  {
    name: "package-subpath-segments-unchecked",
    breaks: "a subpath whose pattern's part has a `node_modules` segment, which Node refuses, ERR_INVALID_MODULE_SPECIFIER, is imported by it (ADR 0770, amended)",
    file: "src/packages.rs",
    find: "    if part.split(['/', '\\\\']).any(refused) {\n",
    replace: "    if false && part.split(['/', '\\\\']).any(refused) {\n",
    tests: subpaths,
  },
  {
    name: "scope-past-nearest",
    breaks:
      "a file's package is the farthest `package.json` above it, a workspace's root, not the nearest, as Node scopes one, so a crate's module in another package is imported by its file (ADR 0770, amended)",
    file: "src/packages.rs",
    find: "        .find(|dir| dir.join(\"package.json\").is_file())\n",
    replace: "        .filter(|dir| dir.join(\"package.json\").is_file())\n        .last()\n",
    tests: ["test/libraries.test.ts", "-t", "another package"],
  },
];
