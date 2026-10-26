// Mutations of src/lower/mir/tidy/copied.rs (ADR 0093).
import type { Mutation } from "../../../../mutations";

export const mutations: Mutation[] = [
  {
    "name": "let-else-copy-kept",
    "breaks": "a `let`-`else`'s binding of a variable stays a copy, `const current$1 = current`",
    "file": "src/lower/mir/tidy/copied.rs",
    "find": "            && then.last().is_some_and(ends)\n            && let StmtKind::Const(b, value) = &stmts[i + 1].kind",
    "replace": "            && then.last().is_some_and(ends)\n            && false\n            && let StmtKind::Const(b, value) = &stmts[i + 1].kind",
    "tests": ["test/lowering-names.test.ts", "-t", "a let-else's binding of a variable is the variable"]
  },
  {
    name: "taken-binding-named-old",
    breaks: "what `take()` gives keeps rust-js's name, `old`, where the program's binding of it, `hidden_updates_for_lane`, is copied of it (ADR 0598, amended)",
    file: "src/lower/mir/tidy/copied.rs",
    find: "    named_for_bindings(stmts, user);\n",
    replace: "    let _ = user;\n",
    tests: ["test/lowering-shadows.test.ts", "-t", "a binding of what take.. gives names it"],
  },
  {
    name: "else-binding-copy-kept",
    breaks: "`match &new_last { None => .., Some(last) => .. }`'s `else` keeps `const last = newLast;`, where the test says `newLast` is there",
    file: "src/lower/mir/tidy/copied.rs",
    find: "        let branches = [(Some(then), true), (otherwise.as_mut(), false)];\n",
    replace: "        let branches = [(Some(then), true), (None, false)];\n        let _ = otherwise;\n",
    tests: ["test/lowering-shadows.test.ts", "-t", "a binding of a variable set again past its reads is the variable"],
  },
  {
    name: "binding-copy-kept-where-set-after",
    breaks: "a binding whose variable the block sets again past each of its reads stays `const last = newLast;`, as one set anywhere",
    file: "src/lower/mir/tidy/copied.rs",
    find: "            let set = rebinds(rest, &a) && (declares(rest, &a) || !read_before_set(rest, &b, &a, &mut false));\n",
    replace: "            let set = rebinds(rest, &a);\n",
    tests: ["test/lowering-shadows.test.ts", "-t", "a binding of a variable set again past its reads is the variable"],
  },
];
