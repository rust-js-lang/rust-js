// Mutations of src/lower/mir/std_calls/written_into.rs (ADR 1135, #1652).
import type { Mutation } from "../../../../mutations";

const tests = ["test/writer-sink.test.ts"];

export const mutations: Mutation[] = [
  {
    name: "written-arguments-not-held",
    breaks: "an argument of a `write!` into a writer is evaluated as it's written, after what comes before it, where Rust evaluates each first",
    file: "src/lower/mir/std_calls/written_into.rs",
    find: "            if let Value::Fmt(_, _, value) = item\n                && value.has_effects()\n",
    replace: "            if let Value::Fmt(_, _, value) = item\n                && value.has_effects()\n                && false\n",
    tests,
  },
  {
    name: "written-own-write-char-passed-over",
    breaks: "a writer's own `write_char` is passed over for its `write_str`",
    file: "src/lower/mir/std_calls/written_into.rs",
    find: "        let write_char_entry = match own_char {\n",
    replace: "        let write_char_entry = match own_char && false {\n",
    tests,
  },
  {
    name: "written-never-fails",
    breaks: "a `write!` into a writer that may fail gives `Ok` where a write gave `Err`",
    file: "src/lower/mir/std_calls/written_into.rs",
    find: "        Ok(Value::Expr(match fails {\n",
    replace: "        Ok(Value::Expr(match fails && false {\n",
    tests,
  },
];
