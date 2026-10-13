// Mutations of src/lower/mir/writer.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "mir-writer-written-whole",
    breaks: "MIR's `write!` of what may fail in a writer is written whole, what came before it lost",
    file: "src/lower/mir/writer.rs",
    find: "let pieces = values.iter().any(|v| matches!(v, Value::Fmt(Std::FmtNew, ..)));",
    replace: "let pieces = false;",
    tests: ["test/corpus.test.ts","-t","fmt_error"],
  },
  {
    name: "mir-writer-builder-kept-stepped",
    breaks: "MIR's `Debug` builder kept in a variable a loop steps shows one step",
    file: "src/lower/mir/writer.rs",
    find: "if let Some(&local) = builders.keys().find(|&&local| named(local)) {",
    replace: "if let Some(&local) = builders.keys().find(|_| false) {",
    tests: ["test/diagnostics.test.ts","-t","a Debug builder kept in a variable"],
  },
  {
    name: "mir-writer-formatter-not-given",
    breaks: "MIR refuses a writer giving its `Formatter` to the crate's generic `fmt::Write` code",
    file: "src/lower/mir/writer.rs",
    find: "if whole && self.given_formatter(def_id, generic_args, at) {",
    replace: "if false {",
    tests: ["test/corpus.test.ts","-t","generic_writers"],
  },
  {
    name: "mir-writer-questions-refused",
    breaks: "a writer that asks its `Formatter`, as `f.alternate()` does, is refused",
    file: "src/lower/mir/writer.rs",
    find: "                    if whole && at == 0 && crate::lower::recognition::formatter_query(self.tcx, def_id).is_some() {\n",
    replace: "                    if false {\n",
    tests: ["test/lowering.test.ts","-t","a writer from its MIR answers what its Formatter is asked"],
  },
  {
    name: "mir-writer-builder-entries-stepped",
    breaks: "MIR's `entries(v.iter())` is `Array.from($iter(v))`",
    file: "src/lower/mir/writer.rs",
    find: "true => crate::lower::iterators::iterated(value),",
    replace: "true => value,",
    tests: ["test/lowering.test.ts","-t","a writer's Debug builder from its MIR is the string its chain writes"],
  },
];
