// Mutations of src/prepare/reclaim.rs (ADR 0093).
import type { Mutation } from "../../mutations";

const tests = ["test/lowering-names.test.ts", "-t", "nothing it shadows"];

export const mutations: Mutation[] = [
  {
    name: "aliased-import-kept",
    breaks: "`import SAME ..; const same = SAME;`, not `import same ..`",
    file: "src/prepare/reclaim.rs",
    find: "    aliased_imports(module);\n",
    replace: "",
    tests: ["test/modules.test.ts", "-t", "an import read only by a const"],
  },
  {
    name: "aliased-import-read-twice",
    breaks: "an import read twice is renamed for one `const`, and the other read names nothing",
    file: "src/prepare/reclaim.rs",
    find: "        if reads != 1 {\n            continue;\n        }\n        // The `const` that reads it",
    replace: "        if reads == 0 {\n            continue;\n        }\n        // The `const` that reads it",
    tests: ["test/modules.test.ts", "-t", "an import read only by a const"],
  },
  {
    name: "reclaim-shadowing-read",
    breaks: "a local is named as the module's function it reads, which it then shadows",
    file: "src/prepare/reclaim.rs",
    find: " || mentioned.contains(base) {",
    replace: " {",
    tests,
  },
  {
    name: "reclaim-keyword",
    breaks: "a parameter named `class` in Rust is `class` in JS, a word JS keeps",
    file: "src/prepare/reclaim.rs",
    find: "    if crate::names::kept_word(base) {",
    replace: "    if false && crate::names::kept_word(base) {",
    tests,
  },
  {
    name: "reclaim-conversion-unread",
    breaks: "a local is `String` where a conversion, `String(n)`, reads the global",
    file: "src/prepare/reclaim.rs",
    find: "        ExprKind::Stringed(_) if reads => f(&mut \"String\".to_string()),",
    replace: "        ExprKind::Stringed(_) if false => f(&mut \"String\".to_string()),",
    tests: ["test/lowering-names.test.ts", "-t", "an import named as a global"],
  },
  {
    name: "reclaim-no-imports",
    breaks: "an import named apart from a global nothing reads keeps its `$`",
    file: "src/prepare/reclaim.rs",
    find: "    imports(module);\n",
    replace: "",
    tests,
  },
  {
    name: "reclaim-no-locals",
    breaks: "a local named apart from a module's name its function never reads keeps its `$1`",
    file: "src/prepare/reclaim.rs",
    find: "        locals(item);\n",
    replace: "",
    tests,
  },
  {
    name: "reclaim-no-lower-suffix",
    breaks: "`result$2` stays, after `result$1` became `result`, where it could be `result$1`",
    file: "src/prepare/reclaim.rs",
    find: "            .chain((1..suffix).map(|i| crate::names::given_apart(base, i)))",
    replace: "            .chain((1..1).map(|i| crate::names::given_apart(base, i)))",
    tests: ["test/snapshots.test.ts", "-t", "calc"],
    snapshots: true,
  },
  {
    "name": "own-item-kept-apart",
    "breaks": "a module's own function named as a global it never reads stays `Math$`",
    "file": "src/prepare/reclaim.rs",
    "find": "    imports(module);\n    own_items(module);",
    "replace": "    imports(module);",
    "tests": ["test/lowering-names.test.ts", "-t", "own function named as a global"]
  },
  {
    "name": "own-item-exported-reclaimed",
    "breaks": "an exported function named as a global is renamed, and a module that imports it by its name finds nothing",
    "file": "src/prepare/reclaim.rs",
    "find": "            js::Item::Function(function) if !function.export => Some(function.name.clone()),",
    "replace": "            js::Item::Function(function) => Some(function.name.clone()),",
    "tests": ["test/lowering-names.test.ts", "-t", "own function named as a global"]
  },
  {
    name: "reclaim-assigned-unrenamed",
    breaks: "a loop's `let step$1` given its name back, `let step`, where the test's assignment still sets `step$1` (ADR 0856)",
    file: "src/prepare/reclaim.rs",
    find: "        ExprKind::Var(name) | ExprKind::Assigned(name, _) if reads => f(name),\n",
    replace: "        ExprKind::Var(name) if reads => f(name),\n",
    tests: ["test/snapshots.test.ts", "-t", "calc"],
  },
];
