// Mutations of crates/rust-js-lower/src/traits/dictionaries/conversions.rs (ADR 0093).
import type { Mutation } from "../../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "into-of-a-binding-untagged-unanswered",
    breaks:
      "a `T: Into<BlobPart>` bound given a `&Uint8Array`, a binding crate's `From` into an untagged enum that's the value, has no dictionary, and is refused (#1775)",
    file: "crates/rust-js-lower/src/traits/dictionaries/conversions.rs",
    find: "            } else if !tried && self.recognition().calls_from_giving_its_argument(from, from_args) {\n",
    replace: "            } else if false {\n",
    tests: ["test/webapi-objects.test.ts", "-t", "From into an untagged"],
  },
  {
    name: "into-without-from",
    breaks:
      "`Into<Name>` of a `&str` isn't the crate's `From<&str> for Name`, and is refused",
    file: "crates/rust-js-lower/src/traits/dictionaries/conversions.rs",
    find: "            } else if let Some(value) = self.compiled_conversion(from, from_args, span)? {\n",
    replace: "            } else if let Some(value) = None::<Expr> {\n",
    tests: ["test/corpus.test.ts", "-t", "generic_into"],
  },
  {
    name: "from-dictionary-named-into",
    breaks: "a `From` dictionary's conversion is named `into`",
    file: "crates/rust-js-lower/src/traits/dictionaries/conversions.rs",
    find: "            let name = bindings::fn_name(self.tcx, method);\n",
    replace: "            let name = String::from(\"into\");\n",
    tests: ["test/corpus.test.ts", "-t", "^generic_from.rs$"],
  },
  {
    name: "std-try-conversion-unanswered",
    breaks: "a `TryFrom<&[u8]>` bound given an array of std's, `[u8; 2]`, is refused, its conversion no dictionary's (#1284)",
    file: "crates/rust-js-lower/src/traits/dictionaries/conversions.rs",
    find: "                && tried\n",
    replace: "                && false\n",
    tests: ["test/corpus.test.ts", "-t", "^conversions_through_generic_code.rs$"],
  },
  {
    name: "dyn-error-into-unanswered",
    breaks: "an `Into<Box<dyn Error + Send + Sync>>` bound given a message is refused (#1284)",
    file: "crates/rust-js-lower/src/traits/dictionaries/conversions.rs",
    find: "            let value = if !tried && let Some(dyn_error)",
    replace: "            let value = if false && let Some(dyn_error)",
    tests: ["test/corpus.test.ts", "-t", "^conversions_through_generic_code.rs$"],
  },
  {
    name: "library-conversion-uncompiled",
    breaks: "a library's `From` its JS exports, which a consumer's copy of its code converts `?`'s error with, is no conversion rust-js compiled, and is refused (#1420)",
    file: "crates/rust-js-lower/src/traits/dictionaries/conversions.rs",
    find: "        if !self.is_rust_fn(instance.def_id()) || self.tcx.trait_of_assoc(instance.def_id()).is_some() {\n",
    replace: "        if !self.krate.fns.contains_key(&instance.def_id()) || self.tcx.trait_of_assoc(instance.def_id()).is_some() {\n",
    tests: ["test/crates-pairs-3.test.ts", "-t", "instance_converts_and_transmutes"],
  },
  {
    name: "placed-conversion-unformed",
    breaks:
      "a conversion's dictionary gives its `From<(A, B)>` impl, which holds each part in a place, the tuple as its caller holds it, so itertools' `multizip` shape reads the bare string's `value` (#2255)",
    file: "crates/rust-js-lower/src/traits/dictionaries/conversions.rs",
    find: "            &mut values,\n            Arguments::Raw,\n            span,\n            &mut made,\n",
    replace: "            &mut values,\n            Arguments::Formed,\n            span,\n            &mut made,\n",
    tests: ["test/crates-pairs-3.test.ts", "test/corpus.test.ts", "-t", "two crates: library_holder_of_a_placed_iterator_built_by_its_generic_code|^conversion_dictionary_given_the_crates_iterator.rs$"],
  },
];
