// Mutations of src/lower/std_types/process.rs (ADR 0391).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "exit-is-args",
    breaks: "`process::exit(code)` calls the helper of `env::args()`",
    file: "src/lower/std_types/process.rs",
    find: '            ProcessOp::Exit => "$exit",\n',
    replace: '            ProcessOp::Exit => "$args",\n',
    tests: ["test/process.test.ts", "-t", "process::exit ends"],
  },
  {
    name: "temp-dir-is-home-dir",
    breaks: "`env::temp_dir()` calls the helper of `env::home_dir()` (#1639)",
    file: "src/lower/std_types/process.rs",
    find: '            ProcessOp::TempDir => "$tempDir",\n',
    replace: '            ProcessOp::TempDir => "$homeDir",\n',
    tests: ["test/process.test.ts", "-t", "env::temp_dir and env::home_dir"],
  },
  {
    name: "report-err-unprinted",
    breaks: "a `Result`'s `report()` of an `Err` writes no `Error: {err:?}` to stderr, as std's does before it gives `1` (#2292)",
    file: "src/lower/std_types/process.rs",
    find: "                err.push(StmtKind::Expr(self.stream_printed(text, true)).at(js_span));\n",
    replace: "                let _ = text;\n",
    tests: ["test/corpus.test.ts", "-t", "process_termination_reports"],
  },
  {
    name: "report-err-succeeds",
    breaks: "a `Result`'s `report()` of an `Err` gives `0`, so a failed `main` ends as one that succeeded (#2292)",
    file: "src/lower/std_types/process.rs",
    find: "                err.push(StmtKind::Assign(Expr::var(&status), Expr::num(1.0)).at(js_span));\n",
    replace: "                err.push(StmtKind::Assign(Expr::var(&status), Expr::num(0.0)).at(js_span));\n",
    tests: ["test/corpus.test.ts", "-t", "process_termination_reports"],
  },
];
