// Mutations of src/link.rs (ADR 0093).
import type { Mutation } from "../mutations";

export const mutations: Mutation[] = [
  {
    name: "unread-module-imported",
    breaks: "a module imports another's item it doesn't read, a numbered enum's variant whose test an exhaustive `match` needn't write, `import { Fast, Slow }`",
    file: "src/link.rs",
    find: "        .filter(|request| read.contains(&request.symbol))\n",
    replace: "",
    tests: ["test/lowering-structs-enums.test.ts", "-t", "a repr enum's variants are its module's consts"],
  },
  {
    name: "module-load-symbols-unresolved",
    breaks: "module-load statements reach the printer with unresolved cross-module symbols",
    file: "src/link.rs",
    find: "            Item::Statements(stmts) => block(stmts, visitor),\n",
    replace: "            Item::Statements(_) => {}\n",
    tests: ["test/link.test.ts", "-t", "module-load statements"],
  },
  {
    name: "module-load-bindings-unreserved",
    breaks: "an import alias collides with a module-load statement's local binding",
    file: "src/link.rs",
    find: "            Item::Statements(stmts) => block(stmts, visitor),\n",
    replace: "            Item::Statements(stmts) => {\n                if visitor.replacements.is_some() {\n                    block(stmts, visitor)\n                }\n            }\n",
    tests: ["test/link.test.ts", "-t", "module-load statements"],
  },
  {
    name: "arrow-params-unreserved",
    breaks: "an import alias isn't named around what a nested arrow binds, so the arrow's parameter, `Card$1`, shadows the import `<Card$1 />` there",
    file: "src/link.rs",
    find: "            for param in params {\n                pattern(param, visitor);\n            }\n            block(body, visitor);\n",
    replace: "            if visitor.replacements.is_some() {\n                for param in params {\n                    pattern(param, visitor);\n                }\n                block(body, visitor);\n            }\n",
    tests: ["test/jsx.test.ts", "-t", "named component imports avoid local functions"],
  },
  {
    name: "local-function-unlinked",
    breaks: "a function written in a body reaches the printer with another module's function unresolved",
    file: "src/link.rs",
    find: "            StmtKind::Function(function) => function_names(function, visitor),\n",
    replace: "            StmtKind::Function(_) => {}\n",
    tests: ["test/lowering-functions.test.ts", "-t", "local function there"],
  },
  {
    name: "written-helper-unasked",
    breaks: "a helper written as the runtime's, tidying's `$lazyRange(0, n)`, isn't asked for, and its module doesn't import it: a ReferenceError where it runs",
    file: "src/link.rs",
    find: "                    written.extend(crate::runtime::helper_of(name));\n",
    replace: "                    let _ = name;\n",
    tests: ["test/lowering-iterators.test.ts", "-t", "a range made where it's stepped"],
  },
  {
    name: "imported-types-module-unwritten",
    breaks: "a module of types alone another module's JS re-exports, `export * as bar from \"./foo/bar.js\"`, writes no JS, so the program throws as it loads (ADR 1005, amended; rustc's imports/use-mod.rs)",
    file: "src/link.rs",
    find: "        module.types_only = false;\n",
    replace: "        let _ = &module;\n",
    tests: ["test/corpus.test.ts", "-t", "^module_of_types_reexported.rs$"],
  },
];
