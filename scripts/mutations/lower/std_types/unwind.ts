// Mutations of src/lower/std_types/unwind.rs (ADR 0660).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "unwinding-where-panics-abort",
    breaks:
      "`catch_unwind` compiles where the crate asked its panics to abort, and catches what ends Rust's process",
    file: "src/lower/std_types/unwind.rs",
    find: "        if self.krate.panics_abort {\n",
    replace: "        if false {\n",
    tests: [
      "test/lowering-unwinds.test.ts",
      "-t",
      "unwinding is refused where panics abort",
    ],
  },
  {
    name: "resume-unwind-not-thrown",
    breaks: "`resume_unwind(payload)` makes what it'd throw and goes on",
    file: "src/lower/std_types/unwind.rs",
    find: "                out.push(StmtKind::Throw(thrown).at(self.js_span(call.span)));\n",
    replace:
      "                out.push(StmtKind::Expr(thrown).at(self.js_span(call.span)));\n",
    tests: [
      "test/lowering-unwinds.test.ts",
      "-t",
      "resume_unwind throws what unwound",
    ],
  },
  {
    name: "resumed-as-given-unmarked",
    breaks:
      "`resume_unwind(e)` of a payload only read is `$unwinding(e)`, a call where the stack may be spent, not what was thrown (rd550)",
    file: "src/lower/std_types/unwind.rs",
    find: "                    thrown.facts = thrown.facts.with(Known::AsGiven);\n",
    replace: "",
    tests: ["test/lowering-unwind-arms.test.ts", "-t", "a catch makes its payload where it's first read"],
  },
  {
    name: "resumed-as-given-by-a-shared-span",
    breaks:
      "a `resume_unwind` is asked of the first THIR call at its span, of two a macro's repetition writes there, so a fact of one is taken of the other (#1671 change 3 M5)",
    file: "src/lower/std_types/unwind.rs",
    find: "                let resumed = crate::lower::body_queries::the_call_at(self.frame.thir, call.span);\n",
    replace:
      "                let resumed = (self.frame.thir.exprs.iter_enumerated()).find_map(|(id, e)| matches!(e.kind, rustc_middle::thir::ExprKind::Call { fn_span, .. } if fn_span == call.span).then_some(id));\n",
    tests: ["test/corpus.test.ts", "-t", "^catch_unwind_resumed_in_a_macro.rs$"],
    snapshots: true,
  },
];
