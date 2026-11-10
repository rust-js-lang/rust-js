// Mutations of src/lower/pointers/into_copies.rs (ADR 0126, amended).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "copy-pointer-compared-with-any",
    breaks: "a pointer into text's bytes compared with any other pointer is kept, where it shows the copy's place, not Rust's (#1183)",
    file: "src/lower/pointers/into_copies.rs",
    find: "                            ) && self.of(&operands.0)\n                                && self.of(&operands.1)\n",
    replace: "                            )\n",
    tests: ["test/diagnostics.test.ts", "-t", "text's bytes' pointers compared"],
  },
  {
    name: "copy-pointer-copied-into",
    breaks: "a pointer into text's bytes given to `ptr::copy` as where to copy to is kept, and the bytes every reader of the text shares are written",
    file: "src/lower/pointers/into_copies.rs",
    find: "Some(Std::Pointer(PointerOp::Copy { dst_first })) => !given(usize::from(!dst_first)),",
    replace: "Some(Std::Pointer(PointerOp::Copy { .. })) => true,",
    tests: ["test/diagnostics.test.ts", "-t", "text's bytes copied into"],
  },
  {
    name: "copy-pointer-returned",
    breaks: "a pointer into text's bytes returned is kept, where its caller may compare it",
    file: "src/lower/pointers/into_copies.rs",
    find: "        if within.contains(&mir::RETURN_PLACE) {\n            return false;\n        }\n",
    replace: "",
    tests: ["test/diagnostics.test.ts", "-t", "text's bytes' pointer returned"],
  },
  {
    name: "copy-pointer-written-through",
    breaks: "a pointer into text's bytes written through is kept, and the bytes every reader of the text shares are written",
    file: "src/lower/pointers/into_copies.rs",
    find: "                    PlaceContext::NonMutatingUse(_) if through => true,\n",
    replace: "                    _ if through => true,\n",
    tests: ["test/diagnostics.test.ts", "-t", "text's bytes written through"],
  },
  {
    name: "copy-pointer-stored-anywhere",
    breaks: "a pointer into text's bytes stored in a field is kept, where it may be compared later",
    file: "src/lower/pointers/into_copies.rs",
    find: "                            assign.0.projection.is_empty() && self.within.contains(&assign.0.local)\n",
    replace: "                            true\n",
    tests: ["test/diagnostics.test.ts", "-t", "text's bytes' pointer stored in a field"],
  },
  {
    name: "copy-pointer-of-any-call",
    breaks: "any call given text's bytes, `to_vec()`, is refused as though it made a pointer into them",
    file: "src/lower/pointers/into_copies.rs",
    find: "        if self.recognition().classify(def_id, generic_args) != Some(Std::Pointer(PointerOp::FirstItem)) {\n            return Ok(());\n        }\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^text_bytes_lent_and_owned.rs$"],
  },
  {
    name: "slice-part-js-slice-where-addresses-are-taken",
    breaks: "a crate that takes a slice's address makes its in-bounds parts with JS's `slice`, which says nothing of where each is",
    file: "src/lower/pointers/into_copies.rs",
    find: "        if self.krate.stored_places.takes_slice_addresses() {\n",
    replace: "        if false {\n",
    tests: ["test/corpus.test.ts", "-t", "^slice_parts_by_address.rs$"],
  },
];
