// crates/rust-js-lower/src/analysis/mir_reads/drop_flow.rs's mutations (#2299 moved them with their code).
import type { Mutation } from "../../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "mir-quiet-drop-unwinds",
    breaks:
      "a drop of what JS drops nothing of is taken to unwind, so a set field's old value dropped in a cycle is given a live flag, `let next$live = true`, and its drop an `if` of it (ADR 0800)",
    file: "crates/rust-js-lower/src/analysis/mir_reads/drop_flow.rs",
    find: "            TerminatorKind::Drop { unwind, .. } if quiet.contains(&block) => *unwind = UnwindAction::Unreachable,\n",
    replace: "            TerminatorKind::Drop { unwind, .. } if false && quiet.contains(&block) => *unwind = UnwindAction::Unreachable,\n",
    tests: ["test/corpus.test.ts", "-t", "^set_field_drops_in_a_cycle.rs$"],
    snapshots: true,
  },
  {
    name: "moved-out-drops-there",
    breaks: "a `Drop` of what's surely moved out is taken to drop it, so what gives a temporary on may unwind (#717)",
    file: "crates/rust-js-lower/src/analysis/mir_reads/drop_flow.rs",
    find: "        if !there {\n            dead.insert(block);\n        }\n",
    replace: "",
    snapshots: true,
    tests: ["test/corpus.test.ts", "-t", "^drops_of_nothing_never_unwind.rs$"],
  },
  {
    name: "quiet-drops-there",
    breaks: "a `Drop` of what JS drops nothing of is taken to drop it, so what drops one may unwind (#717)",
    file: "crates/rust-js-lower/src/analysis/mir_reads/drop_flow.rs",
    find: "                if !drops(place.ty(&body.local_decls, tcx).ty))\n        })\n        .map(|(block, _)| block)\n        .collect();\n    // Of no other",
    replace: "                if false && !drops(place.ty(&body.local_decls, tcx).ty))\n        })\n        .map(|(block, _)| block)\n        .collect();\n    // Of no other",
    snapshots: true,
    tests: ["test/corpus.test.ts", "-t", "^drops_of_nothing_never_unwind.rs$"],
  },
];
