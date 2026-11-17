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
];
