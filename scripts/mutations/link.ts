// Mutations of src/link.rs (ADR 0093).
import type { Mutation } from "../mutations";

export const mutations: Mutation[] = [
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
    tests: ["test/lowering.test.ts","-t","local function there"],
  },
];
