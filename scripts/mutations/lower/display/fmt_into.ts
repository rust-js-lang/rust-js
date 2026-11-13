// Mutations of src/lower/display/fmt_into.rs (ADR 0093): a value only a
// dictionary knows written into a writer's sink by its `fmtInto` (#1652,
// step 5 of ADR 1135).
import type { Mutation } from "../../../mutations";

const sink = ["test/corpus.test.ts", "-t", "^writer_sink_generic_and_dyn.rs$"];

export const mutations: Mutation[] = [
  {
    name: "fmt-into-dyn-unwritten",
    breaks: "a `dyn Display` written into a writer is refused, as its pair's dictionary isn't asked",
    file: "src/lower/display/fmt_into.rs",
    find: "            if !debug && (self.dynamic_trait(ty).is_none() || kind != Std::FmtDisplay) {\n",
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
    find: "        if !(listed || self.krate.written.of_adt(adt)) || !self.is_fmt_trait(tr.def_id) {\n",
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
  {
    name: "dyn-debug-unwritten",
    breaks: "a `dyn Debug`'s `Debug` written into a writer is refused, where its pair's dictionary writes it",
    file: "src/lower/display/fmt_into.rs",
    find: "            let debug = kind == Std::FmtDebug && traits.principal_def_id() == Some(self.debug_trait());\n",
    replace: "            let debug = false;\n",
    tests: ["test/corpus.test.ts", "-t", "^writer_sink_generic_and_dyn.rs$"],
  },
  {
    name: "published-library-type-refused",
    breaks: "a library's published type written into a consumer's own writer is refused, though its dictionary's `fmtInto` writes it",
    file: "src/lower/display/fmt_into.rs",
    find: "        } else if (ty.ty_adt_def()).is_some_and(|adt| self.krate.foreign.lists_fmt_into(adt.did())) {\n",
    replace: "        } else if false {\n",
    tests: ["test/writer-sink.test.ts", "-t", "published type"],
  },
  {
    name: "listed-library-copy-without-fmt-into",
    breaks: "a consumer's own copy of a library's derived `Debug` its contract lists has no `fmtInto`, which the consumer calls",
    file: "src/lower/display/fmt_into.rs",
    find: "            ty::Adt(adt, _) if self.krate.foreign.lists_fmt_into(adt.did()) => adt.did(),\n",
    replace: "            ty::Adt(adt, _) if false && self.krate.foreign.lists_fmt_into(adt.did()) => adt.did(),\n",
    tests: ["test/writer-sink.test.ts", "-t", "a library's published type is written into a consumer's writer a piece at a time"],
  },
  {
    name: "field-only-type-refused",
    breaks: "a private type only a published one's field holds, whose `Debug` nothing shows, rustls-pki-types' `BytesInner`, is refused for a `fmtInto` nothing calls",
    file: "src/lower/display/fmt_into.rs",
    find: "        if !writes_through_sink && !listed && !self.krate.written.given_adt(adt) {\n",
    replace: "        if false {\n",
    tests: ["test/writer-sink.test.ts", "-t", "a library's published type is written into a consumer's writer a piece at a time"],
  },
  {
    name: "std-fmt-fn-value-plain",
    breaks: "a std formatting trait's `fmt` taken as a `fn` pointer, `self.format(f, fmt::Display::fmt)`, is its text where a framed writer arms it, \"didn't write through its sink\"",
    file: "src/lower/display/fmt_into.rs",
    find: "        if self.krate.prints.framed.is_empty() || !crate::lower::recognition::is_fmt_method(self.tcx, def_id) {\n",
    replace: "        if true {\n",
    tests: ["test/corpus.test.ts", "-t", "^writer_sink_fn_value_of_a_std_fmt.rs$"],
  },
];
