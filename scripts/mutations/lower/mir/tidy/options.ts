// Mutations of src/lower/mir/tidy/options.rs (ADR 0093).
import type { Mutation } from "../../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "tested-option-undefined-value",
    breaks:
      "`const x = t ? v : undefined; if (x)` is `if (t)` of a `v` that may be `None` itself, so `maybe(None, true)` reads `undefined.name` (ADR 0910)",
    file: "src/lower/mir/tidy/options.rs",
    find: "    if !matches!(none.kind, ExprKind::Undefined) || !value.facts.has(js::Known::Defined) || changing.contains(v) {\n",
    replace:
      "    if !matches!(none.kind, ExprKind::Undefined) || changing.contains(v) {\n",
    tests: ["test/corpus.test.ts", "-t", "tested_options"],
  },
  {
    name: "tested-option-changed-value",
    breaks:
      "an `Option` of a variable that changes where it's read is read as the variable, so `changed(true)` says `after then after` (ADR 0910)",
    file: "src/lower/mir/tidy/options.rs",
    find: "    if !matches!(none.kind, ExprKind::Undefined) || !value.facts.has(js::Known::Defined) || changing.contains(v) {\n",
    replace:
      "    if !matches!(none.kind, ExprKind::Undefined) || !value.facts.has(js::Known::Defined) {\n",
    tests: ["test/corpus.test.ts", "-t", "tested_options"],
  },
  {
    name: "tested-option-read-elsewhere",
    breaks:
      "an `Option` read past its test is folded away, so what reads it after reads a variable no longer declared (ADR 0910)",
    file: "src/lower/mir/tidy/options.rs",
    find: "    if read_elsewhere || js::declares(reads, x) || js::read_where_other(reads, x, v) {\n",
    replace: "    if js::declares(reads, x) || js::read_where_other(reads, x, v) {\n",
    tests: ["test/corpus.test.ts", "-t", "tested_options"],
  },
  {
    name: "tested-option-before-a-declaration",
    breaks:
      '`const x = t ? v : undefined; let r; if (x) { r = x.p; } else { r = undefined; }` keeps the `Option`, as a `let` is between, so `if let Some(v) = js::classify(v).object() { js::get(v, "$$typeof") } else { None }` reads `v = t ? v : undefined; const r = v?.$$typeof;`',
    file: "src/lower/mir/tidy/options.rs",
    find: "        let at = i + 1 + stmts[i + 1..].iter().take_while(|s| declared(s)).count();\n",
    replace:
      "        let at = i + 1 + 0 * stmts[i + 1..].iter().take_while(|s| declared(s)).count();\n",
    tests: [
      "test/js-object.test.ts",
      "-t",
      "an object.s property read where its test holds is one conditional",
    ],
  },
  {
    name: "tested-option-null-unread",
    breaks:
      "`const x = t ? v : undefined; if (x != null) { ..x.. }` keeps its const, where it's `if (t) { ..v.. }` (ADR 0298, amended)",
    file: "src/lower/mir/tidy/options.rs",
    find: "        (Some(option), _) if of_x(option) => true,\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "tested_options"],
    snapshots: true,
  },
  {
    name: "taken-binding-read-after",
    breaks:
      "a take tested whose binding is read past the test is taken in its branch, so `return cancel` reads what the branch declared (ADR 0996)",
    file: "src/lower/mir/tidy/options.rs",
    find: "    if js::mentions_in(none_branch, c) > 0 || js::mentions_in(rest, c) > 0 {\n",
    replace: "    if js::mentions_in(none_branch, c) > 0 {\n",
    tests: ["test/lowering-takes.test.ts", "-t", "an Option taken then tested"],
  },
  {
    name: "tested-option-read-where-another",
    breaks: "an `Option` made where it's tested is read as its value inside a closure of its own `value`, `log(value._0 + typeof value.x)`, the closure's `Result` (rd514)",
    file: "src/lower/mir/tidy/options.rs",
    find: "|| js::read_where_other(reads, x, v) {\n",
    replace: "{\n",
    tests: ["test/js-object.test.ts", "-t", "beside a closure's own of its name"],
  },
  {
    name: "overwritten-any-write",
    breaks:
      "a write the program makes twice, `slot.count = 1; slot.count = 2;`, loses its first, which no take wrote (rd524)",
    file: "src/lower/mir/tidy/options.rs",
    find: "    if !none.facts.has(js::Known::Taken) || !js::taken_place(target) {\n",
    replace: "    if !js::taken_place(target) {\n",
    tests: ["test/corpus.test.ts", "-t", "^taken_then_overwritten.rs$"],
    snapshots: true,
  },
  {
    name: "overwritten-past-a-call",
    breaks:
      "a take's `None` is dropped past a call given its object, which shows the old value where Rust's shows `None` (rd524)",
    file: "src/lower/mir/tidy/options.rs",
    find: "        _ => false,\n    };\n    if !after.iter().all(between) {\n",
    replace: "        _ => true,\n    };\n    if !after.iter().all(between) {\n",
    tests: ["test/corpus.test.ts", "-t", "^taken_then_overwritten.rs$"],
  },
  {
    name: "overwritten-past-its-read",
    breaks:
      "a take's `None` is dropped past a write of another field of what's read of it, `slot.seen = format!(\"{:?}\", slot.value)`, which reads the old value (rd524)",
    file: "src/lower/mir/tidy/options.rs",
    find: "    let quiet = |value: &Expr| value.is_constant() || matches!(&value.kind, ExprKind::Var(name) if name != root);\n",
    replace: "    let quiet = |_: &Expr| true;\n",
    tests: ["test/corpus.test.ts", "-t", "^taken_then_overwritten.rs$"],
  },
  {
    name: "overwritten-same-field-between",
    breaks:
      "a take's `None` written over is kept where the write over it counts as a write between (rd524)",
    file: "src/lower/mir/tidy/options.rs",
    find: "                other != key && matches!(&of.kind, ExprKind::Var(name) if name == root) && js::taken_place(written)\n",
    replace: "                matches!(&of.kind, ExprKind::Var(name) if name == root) && js::taken_place(written)\n",
    tests: ["test/corpus.test.ts", "-t", "^taken_then_overwritten.rs$"],
    snapshots: true,
  },

  {
    name: "overwritten-branch-tail-unchecked",
    breaks:
      "a take's `None` in an `if`'s branch is dropped past a read of it in the branch (rd524)",
    file: "src/lower/mir/tidy/options.rs",
    find: "    if !after.iter().all(between) {\n        return false;\n    }\n",
    replace: "    if false {\n        return false;\n    }\n",
    tests: ["test/corpus.test.ts", "-t", "^taken_then_overwritten.rs$"],
  },
  {
    name: "overwritten-takes-off",
    breaks:
      "a take's `None` written over by a flattened state's whole write is written, `chunk.value = null;`, where upstream's isn't (rd524)",
    file: "src/lower/mir/tidy/options.rs",
    find: "    while i < stmts.len() {\n        let rest = &stmts[i + 1..];\n",
    replace: "    while false && i < stmts.len() {\n        let rest = &stmts[i + 1..];\n",
    tests: ["test/corpus.test.ts", "-t", "^taken_then_overwritten.rs$"],
    snapshots: true,
  },
];
