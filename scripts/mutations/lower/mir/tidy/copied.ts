// Mutations of src/lower/mir/tidy/copied.rs (ADR 0093).
import type { Mutation } from "../../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "copied-binding-reads-keep-their-id",
    breaks:
      "a binding's reads made the variable's keep the binding's id, spelled as the variable is: a read `js::validate` refuses as another variable's (#2026)",
    file: "src/lower/mir/tidy/copied.rs",
    find: "                        && (*v == b_name || (renamed && *v == a_name))\n",
    replace: "                        && (renamed && *v == a_name)\n",
    tests: ["test/corpus.test.ts", "-t", "^(stock_orders|dyn_any_counted_rc|closure_writes_captured_formatter).rs$"],
  },
  {
    name: "let-else-binding-unnamed",
    breaks:
      "a `let`-`else`'s binding of what `take()` gives is `old`, rust-js's name, not the program's, `thenable` (ADR 0598, amended)",
    file: "src/lower/mir/tidy/copied.rs",
    find: "                let named = named && own;\n",
    replace: "                let named = false;\n",
    tests: ["test/lowering-shadows.test.ts", "-t", "let else's binding"],
  },
  {
    name: "let-else-copy-kept",
    breaks:
      "a `let`-`else`'s binding of a variable stays a copy, `const current$1 = current`",
    file: "src/lower/mir/tidy/copied.rs",
    find: "            && then.last().is_some_and(ends)\n            && let StmtKind::Const(b, value) = &stmts[i + 1].kind",
    replace:
      "            && then.last().is_some_and(ends)\n            && false\n            && let StmtKind::Const(b, value) = &stmts[i + 1].kind",
    tests: ["test/corpus.test.ts", "-t", "^non_null.rs$"],
    snapshots: true,
  },
  {
    name: "taken-binding-named-old",
    breaks:
      "what `take()` gives keeps rust-js's name, `old`, where the program's binding of it, `hidden_updates_for_lane`, is copied of it (ADR 0598, amended)",
    file: "src/lower/mir/tidy/copied.rs",
    find: "    named_for_bindings(stmts, user);\n",
    replace: "    let _ = user;\n",
    tests: [
      "test/lowering-shadows.test.ts",
      "-t",
      "a binding of what take.. gives names it",
    ],
  },
  {
    name: "else-binding-copy-kept",
    breaks:
      "`match &new_last { None => .., Some(last) => .. }`'s `else` keeps `const last = newLast;`, where the test says `newLast` is there",
    file: "src/lower/mir/tidy/copied.rs",
    find: "        let branches = [(Some(then), true), (otherwise.as_mut(), false)];\n",
    replace:
      "        let branches = [(Some(then), true), (None, false)];\n        let _ = otherwise;\n",
    tests: [
      "test/lowering-shadows.test.ts",
      "-t",
      "a binding of a variable set again past its reads is the variable",
    ],
  },
  {
    name: "binding-copy-kept-where-set-after",
    breaks:
      "a binding whose variable the block sets again past each of its reads stays `const last = newLast;`, as one set anywhere",
    file: "src/lower/mir/tidy/copied.rs",
    find: "    rebinds(stmts, a) && (declares(stmts, a) || !read_before_set(stmts, b, a, &mut false))\n",
    replace: "    rebinds(stmts, a)\n",
    tests: [
      "test/lowering-shadows.test.ts",
      "-t",
      "a binding of a variable set again past its reads is the variable",
    ],
  },
  {
    name: "while-let-binding-kept",
    breaks:
      "`while let Some(parent_fiber) = parent { node = parent_fiber; parent = .. }` keeps `const parentFiber = parent;`, as `parent` is set again past its read",
    file: "src/lower/mir/tidy/copied.rs",
    find: "            if !set_before_read(rest, &a_name, &b_name) && !rebinds(rest, &b_name) && !in_closure(rest, &b_name) {\n",
    replace: "            if !rebinds(rest, &a_name) && !rebinds(rest, &b_name) && !in_closure(rest, &b_name) {\n",
    tests: [
      "test/lowering-shadows.test.ts",
      "-t",
      "a while let binding read before its variable is set again is the variable",
    ],
  },
];
