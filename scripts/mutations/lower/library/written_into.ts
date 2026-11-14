// Mutations of src/lower/library/written_into.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "library-writer-fn-unframed-here",
    breaks: "a library's writer function a consumer's framed writer gives its `Formatter` is refused, though its library frames it",
    file: "src/lower/library/written_into.rs",
    find: "                .is_some_and(|library| !library.builtin && library.framed_writers.contains(&key(self.tcx, id)))\n",
    replace: "                .is_some_and(|_| false)\n",
    tests: ["test/crates-compiled-here.test.ts", "-t", "a library's writer given as a value is its import"],
  },
  {
    name: "trait-written-arguments-unread",
    breaks: "a library's trait's method that writes the `Arguments` it's given, called through a `dyn`, is given a template, so a writer it shows claims the sink and its own pieces are lost (#1735)",
    file: "src/lower/library/written_into.rs",
    find: "                    .and_then(|library| library.written_trait_arguments.get(&key(self.tcx, id)).cloned())\n",
    replace: "                    .and_then(|_| None::<Vec<u32>>)\n",
    tests: ["test/writer-sink.test.ts", "-t", "a consumer's Arguments given a library's trait's method through a dyn"],
  },
  {
    name: "unformatted-library-type-lacks-fmt-into",
    breaks: "a library's type no formatting impl is of, held by a consumer's type whose `Display` never shows it, is taken to lack `fmtInto`, so writing that type through generic code stops as a bug of rust-js's (#1688)",
    file: "src/lower/library/written_into.rs",
    find: "            && self.formats(adt)\n",
    replace: "\n",
    tests: ["test/writer-sink.test.ts", "-t", "a library's type nothing formats"],
  },
  {
    name: "formatted-library-type-by-another-trait",
    breaks: "a library's type is taken to be formatted only where an impl of a trait not of formatting is of it, so one with a `Display` its contract doesn't list is written through a dictionary with no `fmtInto`, unchecked (#1688)",
    file: "src/lower/library/written_into.rs",
    find: "            crate::lower::recognition::is_fmt_trait(tcx, tcx.impl_trait_id(imp))\n",
    replace: "            !crate::lower::recognition::is_fmt_trait(tcx, tcx.impl_trait_id(imp))\n",
    tests: ["test/writer-sink.test.ts", "-t", "one its contract doesn't list is a bug"],
  },
  {
    name: "writes-dyns-given-by-none",
    breaks: "no library's function is taken to write the `dyn`s it's given, so a library's `dyn` that may carry no `fmtInto` given to one isn't refused, and the write throws (#1802)",
    file: "src/lower/library/written_into.rs",
    find: "        !id.is_local() && (self.library(id.krate)).is_some_and(|library| !library.builtin && library.dyn_written_into)\n",
    replace: "        !id.is_local() && (self.library(id.krate)).is_some_and(|_| false)\n",
    tests: ["test/writer-sink.test.ts", "-t", "a library's dyn Debug written through generic code"],
  },
];
