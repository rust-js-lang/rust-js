// Mutations of src/lower/mir/tidy/decisions/guarded.rs (#523, ADR 0364).
import type { Mutation } from "../../../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "bindings-set-in-guards-never",
    breaks: "a guarded `|`'s binding its arm reads isn't set in the guard, `odd((x = b))`, so the alternatives are two arms and `either_read` stays MIR's labeled block (#523, C1)",
    file: "src/lower/mir/tidy/decisions/guarded.rs",
    find: "    if many.is_empty() {\n        return None;\n    }\n    let mut tree = tree.to_vec();",
    replace: "    if true {\n        return None;\n    }\n    let mut tree = tree.to_vec();",
    tests: ["test/corpus.test.ts", "-t", "^(guards_that_call).rs$"],
    snapshots: true,
  },
  {
    name: "guard-of-no-call-unbound",
    breaks: "a guard's binding a pure guard reads, `r$1 = o._0; if (!(r$1 > 1))`, isn't read as its value, so `Some(Circle(r) | Sphere(r)) if r > 1.0` keeps MIR's labeled blocks (#523)",
    file: "src/lower/mir/tidy/decisions/guarded.rs",
    find: "        && (!pure(test) || guard())\n",
    replace: "        && !pure(test)\n",
    tests: ["test/corpus.test.ts", "-t", "^(or_bindings).rs$"],
    snapshots: true,
  },
  {
    name: "written-pure-test-bound",
    breaks: "a `const` a test the program wrote reads, one that calls nothing, `const _31 = heap[i]; if (_31 < heap[j])`, is read as its value, so the program's `if`s are a match's tree and their chain, `_19 < heap[index] && (rightIndex >= length || ..)` (#523)",
    file: "src/lower/mir/tidy/decisions/guarded.rs",
    find: "        && (!pure(test) || guard())\n",
    replace: "",
    tests: ["test/lowering-numbers.test.ts", "-t", "integer ranges"],
  },
  {
    name: "binding-set-first-always",
    breaks: "a binding the guard reads once, first, is set before it, `((x = b), odd(x))`, not where it's read, `odd((x = b))` (A1, #523)",
    file: "src/lower/mir/tidy/decisions/guarded.rs",
    find: "            if reads == 1 && super::super::moves::leads(&self.guard, name, None, &|_| true) == Some(true) {\n",
    replace: "            if false && reads == 1 && super::super::moves::leads(&self.guard, name, None, &|_| true) == Some(true) {\n",
    tests: ["test/corpus.test.ts", "-t", "^(guards_that_call).rs$"],
    snapshots: true,
  },
];
