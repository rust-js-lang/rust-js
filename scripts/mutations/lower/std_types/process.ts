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
];
