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
  {
    name: "framed-write-char-as-str",
    breaks: "a framed writer's `f.write_char(c)` is given to the writer's `write_str`, not its own `write_char`",
    file: "src/lower/mir/std_calls/written_into.rs",
    find: "                        _ if self.recognition().is_write_char(def_id) => vec![SinkWrite::Char(value)],\n",
    replace: "",
    tests,
  },
  {
    name: "framed-pad-without-options",
    breaks: "a framed writer's `f.pad(s)` pads by no options, so `{:>6}` of it isn't padded",
    file: "src/lower/mir/std_calls/written_into.rs",
    find: "                    WriteCall::Pad => {\n                        given.push(options);\n",
    replace: "                    WriteCall::Pad => {\n                        given.push(Expr::undefined());\n",
    tests,
  },
  {
    name: "framed-kept-result-thrown",
    breaks: "a framed writer's write whose `Result` it keeps throws the writer's `Err` out of it, so it doesn't carry on as Rust's does",
    file: "src/lower/mir/std_calls/written_into.rs",
    find: "        if result_use(&self.recognition(), state.body, destination) == ResultUse::Inspected {\n",
    replace: "        if result_use(&self.recognition(), state.body, destination) == ResultUse::Inspected && false {\n",
    tests,
  },
  {
    name: "framed-writer-function-refused",
    breaks: "a writer function a framed writer gives its `Formatter` is refused, not called with the sink armed (#1652)",
    file: "src/lower/mir/std_calls/written_into.rs",
    find: "                        if instance.def_id().is_local() && self.writes_through_sink(instance.def_id()) =>\n",
    replace: "                        if false =>\n",
    tests,
  },
  {
    name: "framed-own-failure-unseen",
    breaks: "a writer shown into a writer that returns its own `Err` throws past the `write!`, whose `Result` Rust gives as `Err`",
    file: "src/lower/mir/std_calls/written_into.rs",
    find: "            Value::Fmt(Std::FmtDisplay, ty, _) => self.fmt_may_fail(self.display_trait(), ty),\n",
    replace: "",
    tests,
  },
  {
    name: "framed-derived-struct-as-tuple",
    breaks: "a derived struct's `Debug` written into a writer is written as a tuple's, its field names shown as values (#1652)",
    file: "src/lower/mir/std_calls/written_into.rs",
    find: '                let helper = if named { "$writeDebugStruct" } else { "$writeDebugTuple" };\n',
    replace: '                let helper = if named { "$writeDebugTuple" } else { "$writeDebugTuple" };\n',
    tests,
  },
];
