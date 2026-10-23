// Mutations of src/js/walk.rs: a walk that misses a part of the tree, the
// one that reads it and the one that changes it alike.
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
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
    tests: ["test/corpus.test.ts", "-t", "^temporaries_taken_apart.rs$"],
    snapshots: true,
  },
  {
    name: "else-slot-unsaid",
    breaks: "the walk says an `if`'s `else` is where it is",
    file: "src/js/walk.rs",
    find: "                            $slotted(b, Slot::Else, f);",
    replace: "                            $slotted(b, Slot::Block, f);",
    tests: ["test/lowering-written-branches.test.ts", "-t", "an else-if chain keeps its else"],
  },
];
