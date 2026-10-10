// Mutations of src/prepare/scopes.rs (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "scopes-twice-allowed",
    breaks: "`const n = items.pop(); const n = ..` in one block, which JS refuses",
    file: "src/prepare/scopes.rs",
    find: "        self.twice |= scope.insert(name, id).is_some();",
    replace: "        scope.insert(name, id);",
    tests: ["test/lowering.test.ts", "-t", "reuses a name"],
  },
  {
    name: "scopes-not-hoisted",
    breaks: "`const m = n; const n = 1;` in a block, whose `n` JS reads before it's made",
    file: "src/prepare/scopes.rs",
    find: "                StmtKind::Const(name, _) | StmtKind::Let(name, _) => self.declare(name),",
    replace: "                StmtKind::Const(..) | StmtKind::Let(..) => {}",
    tests: ["test/lowering.test.ts", "-t", "reuses a name"],
  },
  {
    name: "scopes-outermost-first",
    breaks: "a read is of the outermost declaration of its name, not the innermost",
    file: "src/prepare/scopes.rs",
    find: "        let of = (self.scopes.iter().rev())",
    replace: "        let of = (self.scopes.iter())",
    tests: ["test/lowering.test.ts", "-t", "reuses a name"],
  },
  {
    name: "scopes-closure-params-undeclared",
    breaks: "a closure's parameter `json` isn't its own, so a `json$1` in it is named `json` where it reads the outer one",
    file: "src/prepare/scopes.rs",
    find: "        for param in params {\n            self.pattern(param);\n        }\n        self.block_in_scope(body);",
    replace: "        self.block_in_scope(body);",
    tests: ["test/snapshots.test.ts", "-t", "dynamic"],
    snapshots: true,
  },
  {
    name: "scopes-else-not-flattened",
    breaks: "an `else` after a `return` is read as a block of its own, where the printer writes it in the function's, beside its other `n`",
    file: "src/prepare/scopes.rs",
    find: "                StmtKind::If(_, then, Some(els)) if js::leaves(then) => self.hoist(els),",
    replace: "                StmtKind::If(_, then, Some(els)) if false && js::leaves(then) => self.hoist(els),",
    tests: ["test/lowering.test.ts", "-t", "reuses a name"],
  },
];
