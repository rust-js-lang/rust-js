// Mutations of src/js/expr_walk.rs (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "closure-reads-unseen",
    breaks: "a helper only a closure calls isn't imported, and the module throws when the closure runs",
    file: "src/js/expr_walk.rs",
    find: "            ExprKind::Arrow(_, body) | ExprKind::AsyncArrow(_, body) => visit_stmts(body, read),\n",
    replace: "            ExprKind::Arrow(..) | ExprKind::AsyncArrow(..) => {}\n",
    tests: ["test/corpus.test.ts", "-t", "char_from_code"],
  },
  {
    name: "strings-read-as-vars",
    breaks: "a string that spells a helper's name imports it, as scanning the printed text did",
    file: "src/js/expr_walk.rs",
    find: "            ExprKind::Var(name) => read(name),\n",
    replace: "            ExprKind::Var(name) | ExprKind::Str(name) => read(name),\n",
    tests: ["test/link.test.ts", "-t", "spells a helper"],
  },
  {
    name: "closure-blocks-substitution",
    breaks: "a closure called in place, with closures in it, is named and called: `const value = () => ..; ok ? value() : undefined`",
    file: "src/js/expr_walk.rs",
    find: "                    !replaced\n                } =>\n",
    replace: "                    false && !replaced\n                } =>\n",
    tests: ["test/compiler.test.ts", "-t", "then of a closure"],
  },
  {
    name: "closure-reading-replaced-kept",
    breaks: "a closure reading a parameter replaced is kept as it is, reading a variable that isn't there",
    file: "src/js/expr_walk.rs",
    find: "                    !replaced\n                } =>\n",
    replace: "                    replaced || !replaced\n                } =>\n",
    tests: ["test/compiler.test.ts", "-t", "combinators.report matches"],
  },
  {
    name: "truthy-test-kept",
    breaks: "`if js::truthy(value)` is `if (!!value)`, not `if (value)`",
    file: "src/js/expr_walk.rs",
    find: "                value.as_test(changed)\n",
    replace: "                self.clone()\n",
    tests: ["test/bindings.test.ts", "-t", "skips what's falsy is given test && value"],
  },
  {
    name: "retested-never",
    breaks: "a `const` only tests read keeps its boolean, `const hide = !!error || !ready`",
    file: "src/js/expr_walk.rs",
    find: "        changed.then_some(tested)\n",
    replace: "        None\n",
    tests: ["test/lowering.test.ts", "-t", "a const only tests read is read by its truth"],
  },
  {
    name: "test-parts-kept",
    breaks: "`if (a && !!text)`, not `if (a && text)`",
    file: "src/js/expr_walk.rs",
    find: "                        _ => out.push(e.as_test(changed)),\n",
    replace: "                        _ => out.push(e.clone()),\n",
    tests: ["test/lowering.test.ts", "-t", "if-let chains and loops from MIR read as written"],
  },
  {
    name: "reads-same-index-variable",
    breaks: "`files[name]` is spilled, read twice",
    file: "src/js/expr_walk.rs",
    find: " && (index.is_constant() || index.reads_same()),",
    replace: " && index.is_constant(),",
    tests: ["test/lowering.test.ts","-t","property read of what may be None"],
  },
  {
    "name": "tested-null-check-kept",
    "breaks": "a test of `x != null && x` keeps its null check, which `x`'s truth says",
    "file": "src/js/expr_walk.rs",
    "find": "same_path(next, x))",
    "replace": "same_path(next, x) && false)",
    "tests": [
      "test/lowering.test.ts",
      "-t",
      "a guarded binding of an Option|a test from MIR that .x. is set and true"
    ]
  },
  {
    "name": "wrap-call-an-effect",
    "breaks": "a number wrapped to its width, `BigInt.asIntN(64, n)`, is something done, held first, `const value = \u2026; m.cents = value;`",
    "file": "src/js/expr_walk.rs",
    "find": "                        \"BigInt\" => matches!(name.as_str(), \"asIntN\" | \"asUintN\"),\n",
    "replace": "                        \"BigInt\" => false,\n",
    "tests": [
      "test/lowering.test.ts",
      "-t",
      "a wide field updated from MIR"
    ]
  },
  {
    "name": "math-call-does",
    "breaks": "`Math`'s functions count as doing something: a stage of `Math.imul(x, 2)` keeps its chain lazy, and a range's end of `Math.max` is a `{ end }` made first",
    "file": "src/js/expr_walk.rs",
    "find": "                        \"Math\" => name != \"random\",\n",
    "replace": "                        \"Math\" => false,\n",
    "tests": [
      "test/lowering.test.ts",
      "-t",
      "a lazy chain from MIR spread once|a slice to a computed end"
    ]
  },
  {
    "name": "math-random-pure",
    "breaks": "`Math.random()` does nothing, so a stage of it runs where its chain is made",
    "file": "src/js/expr_walk.rs",
    "find": "                        \"Math\" => name != \"random\",\n",
    "replace": "                        \"Math\" => true,\n",
    "tests": [
      "test/lowering.test.ts",
      "-t",
      "a lazy chain from MIR spread once"
    ]
  },
  {
    name: "js-map-not-only-reading",
    breaks: "an array's `.map(f)` of a callback that only reads is taken to do something, and a value formatting reads twice after it stays a function called in place",
    file: "src/js/expr_walk.rs",
    find: "            \"map\",\n            \"join\",\n",
    replace: "            \"join\",\n",
    tests: ["test/corpus.test.ts", "-t", "^(slice_methods|text_edits).rs$"],
    snapshots: true,
  },
  {
    name: "js-substitute-shadowed",
    breaks: "a name a callback binds itself is replaced in its body too, and `{:?}` of an `Option` of an `Option` prints the outer value where the inner is",
    file: "src/js/expr_walk.rs",
    find: "with(n).filter(|_| !names.contains(&n))?",
    replace: "with(n)?",
    tests: ["test/corpus.test.ts", "-t", "^iter_next_back.rs$"],
    snapshots: true,
  },
];
