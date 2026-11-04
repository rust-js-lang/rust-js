// Mutations of src/lower/analysis/programs.rs (ADR 1076, amended, #597).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "tested-program-const-read-unrefused",
    breaks: "a consumer reads a trait's constant of a type parameter that a program's dictionary lacks, `undefined` where Rust reads 4 (#597)",
    file: "src/lower/analysis/programs.rs",
    find: "        if let Some((program, ty)) = foreign.lacks_dictionary_const(konst) {\n",
    replace: "        if let Some((program, ty)) = foreign.lacks_dictionary_const(konst).filter(|_| false) {\n",
    tests: ["test/rust-js-test-programs.test.ts", "-t", "reads a trait's constant"],
  },
  {
    name: "tested-program-dropping-dyn-unrefused",
    breaks: "a consumer makes a `dyn` of a program's trait of what drops, which the program never drops (#597)",
    file: "src/lower/analysis/programs.rs",
    find: "        let Some(program) = foreign.drops_no_dyn_of(tr) else {\n",
    replace: "        let Some(program) = foreign.drops_no_dyn_of(tr).filter(|_| false) else {\n",
    tests: ["test/rust-js-test-programs.test.ts", "-t", "makes a dyn of the program's trait"],
  },
];
