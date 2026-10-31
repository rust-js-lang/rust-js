// Mutations of src/lower/body_queries/ranges/cells.rs (ADR 0093).
import type { Mutation } from "../../../../mutations";

const corpus = ["test/corpus.test.ts", "-t", "refcell_vec_bounds"];

export const mutations: Mutation[] = [
  {
    name: "cell-view-not-followed",
    breaks:
      "`queue.borrow()[i]` below its loop's `i < queue.borrow().len()` is `$index(queue, i)`, not React's `queue[i]`",
    file: "src/lower/body_queries/ranges/cells.rs",
    find: "                ExprKind::Call { fun, ref args, .. } if let Some(view) = self.cell_view(fun) => {",
    replace:
      "                ExprKind::Call { fun, ref args, .. } if false && let Some(view) = self.cell_view(fun) => {",
    tests: corpus,
    snapshots: true,
  },
  {
    name: "cell-guard-change-unseen",
    breaks:
      "`let all = &mut *items; all.clear();` of a `RefMut` is unseen, and `items[i]` after it read unchecked, `undefined` where Rust panics",
    file: "src/lower/body_queries/ranges/cells.rs",
    find: "                .filter(|&(_, celled)| celled)\n",
    replace: "                .filter(|_| false)\n",
    tests: corpus,
  },
  {
    name: "cell-index-not-learned",
    breaks:
      "`queue.borrow_mut()[i] = x; queue.borrow_mut().drain(0..i)` is `$drain(queue, 0, i)`, its range not known below the length the index was",
    file: "src/lower/body_queries/ranges/cells.rs",
    find: "        if self.volatile.contains(&var) {\n            return;\n        }\n        let range",
    replace:
      "        if true || self.volatile.contains(&var) {\n            return;\n        }\n        let range",
    tests: corpus,
    snapshots: true,
  },
  {
    name: "cell-binding-drop-unseen",
    breaks:
      "a binding whose drop empties the queue, `{ let _gone = clearer; }`, is no change, and `queue.borrow()[0]` after it read unchecked",
    file: "src/lower/body_queries/ranges/cells.rs",
    find: "            {\n                self.found(ty);\n            }\n            walk_pat(self, pat);",
    replace: "            {\n            }\n            walk_pat(self, pat);",
    tests: corpus,
  },
  {
    name: "cell-assign-old-value-own-cell-too",
    breaks:
      "`queue.borrow_mut()[at] = task`, whose old task's drop runs while the queue's guard holds it, forgets the queue's length, and its drain is `$drain`",
    file: "src/lower/body_queries/ranges/cells.rs",
    find: "            .filter(|((path, _), _)| path.contains(&CELL))\n",
    replace: "            .filter(|_| false)\n",
    tests: corpus,
    snapshots: true,
  },
  {
    name: "cell-read-through-test-held",
    breaks:
      "`let there = !queue.borrow().is_empty();` holds its test past a call that empties the queue, and `queue.borrow()[0]` after it read unchecked",
    file: "src/lower/body_queries/ranges/cells.rs",
    find: "            fn_def(thir[strip(thir, fun)].ty).is_some_and(|(def_id, args)| cell_view(tcx, def_id, args).is_some())\n",
    replace: "            false\n",
    tests: corpus,
  },
];
