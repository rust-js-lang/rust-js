// Mutations of src/lower/std_coverage.rs (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "std-coverage-nothing-known",
    breaks: "every std method counts as refused, though rust-js knows it",
    file: "src/lower/std_coverage.rs",
    find: "                    .any(|pattern| recognition.classify(def_id, given(pattern)).is_some());\n",
    replace: "                    .any(|pattern| recognition.classify(def_id, given(pattern)).is_some() && false);\n",
    tests: ["test/std-coverage.test.ts"],
  },
  {
    name: "std-coverage-patterns-untried",
    breaks: "a `str`'s `split` counts as refused, though it's known of a `&str` or a `char`",
    file: "src/lower/std_coverage.rs",
    find: "                let known = [ty::Ty::new_static_str(tcx), tcx.types.char, tcx.types.usize]\n",
    replace: "                let known = [tcx.types.unit]\n",
    tests: ["test/std-coverage.test.ts"],
  },
];
