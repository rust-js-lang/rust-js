// Mutations of src/lower/std_types/cell/variables.rs (ADR 0093), split from cell.ts's list with its code (#1334).
import type { Mutation } from "../../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "mir-statics-closure-set-unseen",
    breaks:
      "a thread-local a closure sets is a `const`, which the closure can't set",
    file: "src/lower/std_types/cell/variables.rs",
    find: "        js::each_item_block_mut(items, &mut |stmts| {\n            for stmt in stmts.iter() {\n                if let StmtKind::Assign(target, _) = &stmt.kind\n",
    replace:
      "        js::each_item_block_mut(&mut [], &mut |stmts| {\n            for stmt in stmts.iter() {\n                if let StmtKind::Assign(target, _) = &stmt.kind\n",
    tests: [
      "test/lowering-cells.test.ts",
      "-t",
      "thread-local only its module",
    ],
  },
  {
    name: "applied-block-kept",
    breaks:
      "a plain thread-local's closure called as a statement stays `((timerQueue) => { .. })(timerQueue)`",
    file: "src/lower/std_types/cell/variables.rs",
    find: "            js::applied_blocks(&mut function.body, true);\n",
    replace: "",
    tests: [
      "test/lowering-cells.test.ts",
      "-t",
      "closure called as a statement is its body",
    ],
  },
  {
    name: "applied-loaded-kept",
    breaks:
      "what a module runs when it's loaded keeps a thread-local's `with` a closure called, `((internals) => { .. })(Internals)`",
    file: "src/lower/std_types/cell/variables.rs",
    find: "            js::Item::Statements(stmts) => js::applied_blocks(stmts, false),\n",
    replace: "            js::Item::Statements(_) => {}\n",
    tests: ["test/lowering-cells.test.ts", "-t", "with is its closure's body"],
  },
  {
    name: "applied-loaded-tail-return",
    breaks:
      "what a module runs when it's loaded takes a closure's `return;` as its own, a module's `return`, which JS refuses",
    file: "src/lower/std_types/cell/variables.rs",
    find: "            js::Item::Statements(stmts) => js::applied_blocks(stmts, false),\n",
    replace:
      "            js::Item::Statements(stmts) => js::applied_blocks(stmts, true),\n",
    tests: ["test/lowering-cells.test.ts", "-t", "with is its closure's body"],
  },
  {
    name: "exported-cell-boxed",
    breaks:
      "a `pub` thread-local's cell only its module sets is `export const enabled = { value: false }`, which a JS importer reads as an object (ADR 0270, amended)",
    file: "src/lower/std_types/cell/variables.rs",
    find: "            js::Item::Const(c) if (!c.export || quiet.contains(c.name.as_str())) && is_box(&c.value) => {\n",
    replace:
      "            js::Item::Const(c) if !c.export && is_box(&c.value) => {\n",
    tests: [
      "test/lowering-module-variables.test.ts",
      "-t",
      "an exported cell its module alone sets",
    ],
  },
  {
    name: "kept-box-refused",
    breaks:
      "a thread-local its module keeps, whose `&mut` it hands on, is refused as an exported one is, though no JS module imports it (ADR 0270, amended)",
    file: "src/lower/std_types/cell/variables.rs",
    find: "            js::Item::Const(c) if c.export && held_values.contains(c.name.as_str()) && is_box(&c.value) => {\n",
    replace:
      "            js::Item::Const(c) if held_values.contains(c.name.as_str()) && is_box(&c.value) => {\n",
    tests: [
      "test/lowering-module-variables.test.ts",
      "-t",
      "an exported cell its module needs the cell of is refused",
    ],
  },
  {
    name: "importer-reads-value",
    breaks:
      "another module reads an exported variable's `value`, `enabled.value`, of a boolean (ADR 0270, amended)",
    file: "src/lower/std_types/cell/variables.rs",
    find: "        js::ExprKind::Member(of, key) if key == js::HELD && variable(of) => {\n",
    replace:
      "        js::ExprKind::Member(of, key) if key == js::HELD && variable(of) && false => {\n",
    tests: [
      "test/lowering-module-variables.test.ts",
      "-t",
      "an exported cell its module alone sets",
    ],
  },
  {
    name: "importer-borrow-kept",
    breaks:
      "another module's `with_borrow` of an exported variable stays `$withBorrow(label, ..)`, which counts borrows of a string (ADR 0270, amended)",
    file: "src/lower/std_types/cell/variables.rs",
    find: '            if callee.helper_name() == Some("$withBorrow") && matches!(args.as_slice(), [key, _] if variable(key)) =>\n',
    replace:
      '            if callee.helper_name() == Some("$withBorrow") && matches!(args.as_slice(), [key, _] if variable(key) && false) =>\n',
    tests: [
      "test/lowering-module-variables.test.ts",
      "-t",
      "an exported cell its module alone sets",
    ],
  },
  {
    name: "importer-read-fixed",
    breaks:
      "an importer's read of another module's variable is taken for one of a function, which nothing sets, so `const rendering = isRendering` is read where it's used, after its module set it again (ADR 0270, amended)",
    file: "src/lower/std_types/cell/variables.rs",
    find: "                facts: js::Facts::of(js::Known::Live),\n                ..read\n",
    replace:
      "                facts: js::Facts::of(js::Known::Item),\n                ..read\n",
    tests: ["test/corpus.test.ts", "-t", "^exported_cells.rs$"],
  },
  {
    name: "nullable-made-as-given",
    breaks:
      "a nullable thread-local's cell is made of what it's given, `undefined`, not `null` (ADR 0586, amended)",
    file: "src/lower/std_types/cell/variables.rs",
    find: "                *value = NoneForm::Null\n                    .storage()\n                    .written(std::mem::replace(value, Expr::undefined()), false);\n",
    replace: "",
    tests: [
      "test/lowering-module-variables.test.ts",
      "-t",
      "nullable thread-local's None is null",
    ],
  },
  {
    name: "module-array-set-in-place",
    breaks:
      "a thread-local of an array its module sets whole is written in place, `$assign(callbacks, [])`, where React's `callbacks = []` gives the variable the new one (ADR 0270, amended)",
    file: "src/lower/std_types/cell/variables.rs",
    find: '                    && callee.helper_name() == Some("$assign")\n',
    replace:
      '                    && callee.helper_name() == Some("$assign ")\n',
    tests: [
      "test/lowering-module-variables.test.ts",
      "-t",
      "a thread-local of an array its module sets",
    ],
  },
  {
    name: "module-array-set-const",
    breaks:
      "a thread-local of an array its module gives a new one stays `const`, and JS throws assigning it (ADR 0270, amended)",
    file: "src/lower/std_types/cell/variables.rs",
    find: "                c.mutable = true;\n                if c.export {\n                    exported.push((name.clone(), true));\n",
    replace:
      "                if c.export {\n                    exported.push((name.clone(), true));\n",
    tests: [
      "test/lowering-module-variables.test.ts",
      "-t",
      "a thread-local of an array its module sets",
    ],
  },
  {
    name: "module-array-set-unexported",
    breaks:
      "a thread-local of an array its module gives a new one is declared `export const`, which its importers' TypeScript says can't change (ADR 0270, amended)",
    file: "src/lower/std_types/cell/variables.rs",
    find: "                    exported.push((name.clone(), true));\n",
    replace: "",
    tests: [
      "test/lowering-module-variables.test.ts",
      "-t",
      "a thread-local of an array its module sets",
    ],
  },
  {
    name: "constant-closure-with-applied-kept",
    breaks:
      "a thread-local's `with` in a closure of a constant's value, React's DEV dispatchers', stays `((dispatcher) => { .. })(nested)` (ADR 0581, amended)",
    file: "src/lower/std_types/cell/variables.rs",
    find: "            js::Item::Const(constant) => js::applied_closures(&mut constant.value),\n",
    replace: "            js::Item::Const(_) => {}\n",
    tests: [
      "test/lowering-module-variables.test.ts",
      "-t",
      "a thread-local's with in a closure of a constant's value is its body",
    ],
  },
  {
    name: "applied-stored-again-skipped",
    breaks:
      "a closure put in its place keeps a nullable store's `?? null` of what it's now given, never `undefined` (ADR 0841, amended)",
    file: "src/lower/std_types/cell/variables.rs",
    find: "    js::each_item_expr_mut(items, &mut js::Expr::store_again);\n",
    replace: "",
    tests: ["test/lowering-nullable-reads.test.ts", "-t", "stored as they are"],
  },
  {
    name: "taken-whole-in-place",
    breaks:
      "a thread-local array taken first in its statement is emptied in place, `for (const id of $take(pending, []))`, not React's `const old = pending; pending = [];` (ADR 0270, amended)",
    file: "src/lower/std_types/cell/variables.rs",
    find: "            sets += taken_whole(stmts, &name);\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^thread_local_vec_taken.rs$"],
    snapshots: true,
  },
  {
    name: "taken-unread-read",
    breaks:
      "a take whose value is unread reads the old array into a statement of its own, `const old = pending; pending = []; old;` (ADR 0270, amended)",
    file: "src/lower/std_types/cell/variables.rs",
    find: "        if let StmtKind::Expr(_) = &stmts[i].kind {\n            stmts[i] = set;\n",
    replace: "        if false {\n            stmts[i] = set;\n",
    tests: ["test/corpus.test.ts", "-t", "^thread_local_vec_taken.rs$"],
    snapshots: true,
  },
  {
    name: "taken-let-of-old",
    breaks:
      "a `let` of a take is a `const` of an `old` it reads, `const old = pending; pending = [7, 8]; const before = old;`, not `const before = pending;` (ADR 0270, amended)",
    file: "src/lower/std_types/cell/variables.rs",
    find: "        if let StmtKind::Const(..) | StmtKind::Let(..) = &stmts[i].kind {\n",
    replace: "        if false {\n",
    tests: ["test/corpus.test.ts", "-t", "^thread_local_vec_taken.rs$"],
    snapshots: true,
  },
  {
    name: "namespaced-variable-held",
    breaks:
      "another module's thread-local read through its module's namespace, `fiber::IS_RENDERING.get()`, is read as its cell, `Fiber.isRendering.value`, where it's its module's variable, `Fiber.isRendering`, which has no `value` (rd114a, ADR 1018)",
    file: "src/lower/std_types/cell/variables.rs",
    find: "if namespace.export == js::NAMESPACE\n",
    replace: "if namespace.export == js::NAMESPACE && false\n",
    tests: ["test/corpus.test.ts", "-t", "^exported_cells.rs$"],
  },
  {
    name: "crate-variable-read-as-cell",
    breaks:
      "another of the crate's modules reads an exported variable's `value`, `enabled.value` (ADR 0270, amended)",
    file: "src/lower/std_types/cell/variables.rs",
    find: "    if named.is_empty() {\n        return;\n",
    replace: "    if !named.is_empty() {\n        return;\n",
    tests: [
      "test/lowering-module-variables.test.ts",
      "-t",
      "an exported cell its module alone sets",
    ],
  },
  {
    name: "placed-read-catch-kept",
    breaks:
      "a library's thread-local read as its `export let`, `previous = current`, keeps the `try` its `$withBorrow(..)` had, `catch (error) { dropA0?.(arg0); throw error; }` (#408)",
    file: "src/lower/std_types/cell/variables.rs",
    find: "            crate::lower::mir::settle_unthrown_catches(&mut function.body);\n",
    replace: "",
    tests: ["test/crates.test.ts", "-t", "settles its cleanup"],
  },
  {
    name: "nullable-cell-kept-plain",
    breaks:
      "a nullable thread-local kept as its cell is a `{ value }`, so what a counted lend writes, `$withBorrowMut(counted, clearGeneric, true)`, is stored as it is (#541)",
    file: "src/lower/std_types/cell/variables.rs",
    find: "            nullable_cell(&mut c.value);\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^nullable_written_through_borrows.rs$"],
  },
  {
    name: "nullable-lazy-local-plain",
    breaks:
      "a nullable thread-local made where it's first used is `$local(() => ({ value: init }))`, so what's written through it is stored as it is, `undefined` for `None` (#541, #554)",
    file: "src/lower/std_types/cell/variables.rs",
    find: '        js::ExprKind::Call(callee, args) if callee.helper_name() == Some("$local") => {\n',
    replace: '        js::ExprKind::Call(callee, args) if false && callee.helper_name() == Some("$local") => {\n',
    tests: ["test/corpus.test.ts", "-t", "^thread_local_lazy_where_observable.rs$"],
    snapshots: true,
  },
];
