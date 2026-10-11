// Mutations of src/lower/mir/tidy.rs (ADR 0364).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "tidy-effectful-init-dropped",
    breaks: "`let a = made(1); a = 2;` is `let a = 2;`, and `made(1)` never runs",
    file: "src/lower/mir/tidy.rs",
    find: "is_none_or(|e| !e.has_effects())",
    replace: "is_none_or(|e| true || !e.has_effects())",
    tests: ["test/mir.test.ts","-t","declarations_assigned_later"],
  },
  {
    name: "tidy-declaration-past-read",
    breaks: "a `let` moves to its next assignment past a read of it, which reads it before it's declared",
    file: "src/lower/mir/tidy.rs",
    find: "                    .find(|&j| js::mentions_in(&stmts[j..=j], name) > 0)\n",
    replace: "                    .find(|&j| matches!(stmts[j].kind, StmtKind::Assign(..)) && js::mentions_in(&stmts[j..=j], name) > 0)\n",
    tests: ["test/mir.test.ts","-t","declarations_assigned_later"],
  },
  {
    name: "tidy-closure-assign-unseen",
    breaks: "a variable a closure assigns is a `const`",
    file: "src/lower/mir/tidy.rs",
    find: "            {\n                assigned.insert(name.clone());\n            }\n        }\n",
    replace: "            {\n                let _ = name;\n            }\n        }\n",
    tests: ["test/mir.test.ts","-t","declarations_assigned_later"],
  },
  {
    name: "tidy-handle-assign-unseen",
    breaks: "a variable a handle's setter assigns is a `const`",
    file: "src/lower/mir/tidy.rs",
    find: "        if let ExprKind::Handle(place) | ExprKind::Pair(place, _) = &e.kind\n",
    replace: "        if false && let ExprKind::Handle(place) | ExprKind::Pair(place, _) = &e.kind\n",
    tests: ["test/mir.test.ts","-t","mut_ref_handle"],
  },
  {
    name: "mir-tidy-alias-kept",
    breaks: "a `const` read once, by another `const` of it, stays a name for a name",
    file: "src/lower/mir/tidy.rs",
    find: "                            && js::mentions_in(&stmts[j..], a) == 1\n",
    replace: "                            && false\n",
    tests: ["test/mir.test.ts","-t","JSX tests"],
  },
];
