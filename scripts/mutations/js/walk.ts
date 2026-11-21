// Mutations of src/js/walk.rs: a walk that misses a part of the tree, the
// one that reads it and the one that changes it alike.
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "walk-some-where-value-unwalked",
    breaks: "a walk doesn't visit a `SomeWhere`'s value",
    file: "src/js/walk.rs",
    find: "                    ExprKind::SomeWhere(a, b) => {\n                        a.$expr(f);\n                        b.$expr(f);\n                    }\n",
    replace: "                    ExprKind::SomeWhere(a, _) => {\n                        a.$expr(f);\n                    }\n",
    tests: ["test/corpus.test.ts", "-t", "^cloned_of_a_generic_source_cloned_and_stepped_from_its_back.rs$"],
    snapshots: true,
  },
  {
    name: "walks-skip-closures",
    breaks: "an expression's walk skips a closure's body, so a pass neither sees nor changes what's in one",
    file: "src/js/walk.rs",
    find: "                    ExprKind::Arrow(_, body) | ExprKind::AsyncArrow(_, body) => $exprs(body, f),\n",
    replace: "                    ExprKind::Arrow(..) | ExprKind::AsyncArrow(..) => {}\n",
    tests: ["test/corpus.test.ts", "-t", "^(closure_drops|iterator_reduce|map_eq).rs$"],
  },
  {
    name: "walks-skip-switches",
    breaks: "a list walk skips a `switch`'s cases",
    file: "src/js/walk.rs",
    find: "                        $cases(cases, default).for_each(|body| $slotted(body, Slot::Block, f));\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^shadow_set_in_cases.rs$"],
  },
  {
    name: "else-slot-unsaid",
    breaks: "the walk says an `if`'s `else` is where it is",
    file: "src/js/walk.rs",
    find: "                            $slotted(b, Slot::Else, f);",
    replace: "                            $slotted(b, Slot::Block, f);",
    snapshots: true,
    tests: ["test/corpus.test.ts", "-t", "^(default_value_exits|interpreter)\\.rs$"],
  },
  {
    name: "assigned-target-unwalked",
    breaks: "an assignment's value's target isn't walked, so a loop's `let step$1` is given its name back, `let step`, where the test's assignment still sets `step$1` (ADR 0860)",
    file: "src/js/walk.rs",
    find: "                    | ExprKind::Assigned(a, b) => {\n                        a.$expr(f);\n                        b.$expr(f);\n                    }\n",
    replace: "                    => {\n                        a.$expr(f);\n                        b.$expr(f);\n                    }\n                    ExprKind::Assigned(_, b) => b.$expr(f),\n",
    tests: ["test/snapshots.test.ts", "-t", "calc"],
  },
];
