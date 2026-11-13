// Mutations of src/lower/display/fmt_into.rs (ADR 0093): a value only a
// dictionary knows written into a writer's sink by its `fmtInto` (#1652,
// step 5 of ADR 1135).
import type { Mutation } from "../../../mutations";

const sink = ["test/writer-sink.test.ts", "-t", "each of generic code's and dyns' writes is native Rust's, in the plain build"];

export const mutations: Mutation[] = [
  {
    name: "fmt-into-dyn-unwritten",
    breaks: "a `dyn Display` written into a writer is refused, as its pair's dictionary isn't asked",
    file: "src/lower/display/fmt_into.rs",
    find: "            if self.dynamic_trait(ty).is_none() || kind != Std::FmtDisplay {\n",
    replace: "            if true {\n",
    tests: sink,
  },
  {
    name: "fmt-into-generic-unwritten",
    breaks: "a `T` written into a writer is refused, as its caller's dictionary isn't asked",
    file: "src/lower/display/fmt_into.rs",
    find: "        } else if self.formatted_by_dictionary(trait_id, ty) {\n",
    replace: "        } else if false {\n",
    tests: sink,
  },
  {
    name: "fmt-into-impl-entry-left-out",
    breaks: "an impl of the crate's own given a generic writer carries no `fmtInto`, which the writer calls",
    file: "src/lower/display/fmt_into.rs",
    find: "        if !self.krate.written.of_adt(adt) || !self.is_fmt_trait(tr.def_id) {\n",
    replace: "        if true {\n",
    tests: sink,
  },
  {
    name: "fmt-into-std-entry-left-out",
    breaks: "a std value's dictionary given a generic writer, an `i32`'s, carries no `fmtInto`, which the writer calls",
    file: "src/lower/display/fmt_into.rs",
    find: "        if !self.krate.written.of_type(self.tcx, ty) {\n",
    replace: "        if true {\n",
    tests: sink,
  },
  {
    name: "fmt-into-generic-impl-dictionaries-dropped",
    breaks: "a generic impl's writer, `Duo<T>`'s, armed by its `fmtInto` is given no `TDisplay`, so its `T`'s write throws",
    file: "src/lower/display/fmt_into.rs",
    find: "        args.extend(extra);\n",
    replace: "",
    tests: sink,
  },
  {
    name: "fmt-into-debug-never-pretty",
    breaks: "a std value's `Debug` written by its `fmtInto` under `{:#?}` writes it on one line",
    file: "src/lower/display/fmt_into.rs",
    find: "                let pretty = Pretty::When(given.clone()).alternate();\n",
    replace: "                let pretty = Pretty::Plain.alternate();\n",
    tests: sink,
  },
];
