// Mutations of src/prepare/scopes.rs (ADR 0093). Its reads resolved
// innermost first, and a rename refused of a read from outside or one that
// passes the new name's declaration, are its unit tests', `cargo test`: no
// Rust rust-js lowers reaches them, as it numbers names in order.
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "scopes-twice-allowed",
    breaks: "`const n = items.pop(); const n = ..` in one block, which JS refuses",
    file: "src/prepare/scopes.rs",
    find: "        if declaring.iter().any(|&s| self.declares(s, to)) {",
    replace: "        if false {",
    tests: ["test/lowering-names.test.ts", "-t", "reuses a name"],
  },
  {
    name: "scopes-not-hoisted",
    breaks: "`const m = n; const n = 1;` in a block, whose `n` JS reads before it's made",
    file: "src/prepare/scopes.rs",
    find: "                StmtKind::Const(name, _) | StmtKind::Let(name, _) => self.declare(name),",
    replace: "                StmtKind::Const(..) | StmtKind::Let(..) => {}",
    tests: ["test/lowering-names.test.ts", "-t", "reuses a name"],
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
    name: "scopes-other-read-captured",
    breaks: "`const m = n;` in a block whose `n$1` is named `n`, which the read then is",
    file: "src/prepare/scopes.rs",
    find: "        of_from.iter().all(|r| r.of.is_some() && !passes(r, to)) && of_to.iter().all(|r| !passes(r, from))",
    replace: "        of_from.iter().all(|r| r.of.is_some() && !passes(r, to))",
    tests: ["test/lowering-names.test.ts", "-t", "reuses a name"],
  },
  {
    name: "on-load-scopes-unwalked",
    breaks: "names in closures `js::on_load!` sets keep the names given apart, `fiber$1`, `sortedNames$1` (ADR 0952)",
    file: "src/prepare/scopes.rs",
    find: "                walk.tree.module_scope = true;\n                walk.block_in_scope(stmts);\n",
    replace: "                let _ = stmts;\n                return None;\n",
    tests: ["test/lowering-shadows.test.ts", "-t", "runs at load"],
  },
];
