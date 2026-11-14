// Mutations of src/lower/untagged/froms.rs (ADR 0093): a `From` into an
// untagged enum, which calls of one are the value it's given (ADR 0214,
// amended, #1775).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "mir-untagged-into-through-trait-call",
    breaks:
      "MIR's `into()` to an untagged enum isn't asked of as the `from` it runs, so a binding crate's, `chunk.into()` of a `BlobPart`, isn't the value",
    file: "src/lower/untagged/froms.rs",
    find: "            Some((_, _, implementation)) => Some(implementation),\n",
    replace: "            Some(_) => None,\n",
    tests: ["test/webapi-objects.test.ts", "-t", "From into an untagged enum that does more"],
  },
  {
    name: "untagged-from-body-skipped",
    breaks:
      "the crate's `From` into an untagged enum that does more than make the variant of its argument is taken for its argument where it's called, `Detail::from(5u32)` `5`, its body never run (#1775)",
    file: "src/lower/untagged/froms.rs",
    find: "            Some(local) => gives_its_variant(tcx, local),\n",
    replace: "            Some(_) => true,\n",
    tests: ["test/corpus.test.ts", "-t", "^untagged_from_that_does_more.rs$"],
  },
  {
    name: "untagged-from-of-a-library-skipped",
    breaks:
      "a library's `From` into its untagged enum that does more is taken for its argument by its consumer, whatever its contract says, `3u32.into()` `3` (#1775)",
    file: "src/lower/untagged/froms.rs",
    find: "            None if foreign.is_library(def_id.krate) => foreign.gives_its_argument(def_id),\n",
    replace: "            None if foreign.is_library(def_id.krate) => true,\n",
    tests: ["test/crates.test.ts", "-t", "untagged_from_across_crates|untagged From"],
  },
  {
    name: "untagged-from-of-a-binding-called",
    breaks:
      "a binding crate's `From` into an untagged enum, webapi's `BlobPart::from(chunk)`, is a call of a body rust-js has no JS of (#1775)",
    file: "src/lower/untagged/froms.rs",
    find: "            None => true,\n        }\n}\n",
    replace: "            None => false,\n        }\n}\n",
    tests: ["test/webapi-objects.test.ts", "-t", "From into an untagged"],
  },
  {
    name: "binding-from-that-does-more-accepted",
    breaks:
      "a binding crate's `From` into an untagged enum that does more than make the variant of its argument is accepted, and its calls taken for their argument (#1775)",
    file: "src/lower/untagged/froms.rs",
    find: "        if is_untagged_from(tcx, local.to_def_id()) && !variant_of_its_argument(tcx, local, true) {\n",
    replace: "        if false && is_untagged_from(tcx, local.to_def_id()) && !variant_of_its_argument(tcx, local, true) {\n",
    tests: ["test/diagnostics.test.ts", "-t", "binding crate.s From"],
  },
  {
    name: "binding-from-of-a-boxed-callback-refused",
    breaks:
      "react's `Ref::Callback(Box::new(callback))`, a binding crate's `From` whose `Box<dyn Fn>` is the function, is refused where it's compiled for its metadata (#1775)",
    file: "src/lower/untagged/froms.rs",
    find: "            of_adt && (given(arg) || (boxed && boxed_given()))\n",
    replace: "            of_adt && given(arg)\n",
    tests: ["test/diagnostics.test.ts", "-t", "binding crate.s From"],
  },
];
