// Mutations of src/lower/std_types/unwind.rs (ADR 0660).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "unwinding-where-panics-abort",
    breaks: "`catch_unwind` compiles where the crate asked its panics to abort, and catches what ends Rust's process",
    file: "src/lower/std_types/unwind.rs",
    find: "        if self.krate.panics_abort {\n",
    replace: "        if false {\n",
    tests: ["test/lowering-unwinds.test.ts", "-t", "unwinding is refused where panics abort"],
  },
  {
    name: "resume-unwind-not-thrown",
    breaks: "`resume_unwind(payload)` makes what it'd throw and goes on",
    file: "src/lower/std_types/unwind.rs",
    find: "                out.push(StmtKind::Throw(thrown).at(self.js_span(call.span)));\n",
    replace: "                out.push(StmtKind::Expr(thrown).at(self.js_span(call.span)));\n",
    tests: ["test/lowering-unwinds.test.ts", "-t", "resume_unwind throws what unwound"],
  },
];
