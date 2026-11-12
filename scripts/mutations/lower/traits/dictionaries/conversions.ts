// Mutations of src/lower/traits/dictionaries/conversions.rs (ADR 0093).
import type { Mutation } from "../../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "into-without-from",
    breaks:
      "`Into<Name>` of a `&str` isn't the crate's `From<&str> for Name`, and is refused",
    file: "src/lower/traits/dictionaries/conversions.rs",
    find: "            } else if let Some(value) = self.compiled_conversion(from, from_args, span)? {\n",
    replace: "            } else if let Some(value) = None::<Expr> {\n",
    tests: ["test/corpus.test.ts", "-t", "generic_into"],
  },
  {
    name: "from-dictionary-named-into",
    breaks: "a `From` dictionary's conversion is named `into`",
    file: "src/lower/traits/dictionaries/conversions.rs",
    find: "            let name = bindings::fn_name(self.tcx, method);\n",
    replace: "            let name = String::from(\"into\");\n",
    tests: ["test/corpus.test.ts", "-t", "^generic_from.rs$"],
  },
  {
    name: "from-dictionary-backwards",
    breaks: "a `From<U>` dictionary of a `W` converts a `W` into a `U`",
    file: "src/lower/traits/dictionaries/conversions.rs",
    find: "                false => (ty, tr.args.type_at(1)),\n",
    replace: "                false => (tr.args.type_at(1), ty),\n",
    tests: ["test/corpus.test.ts", "-t", "^generic_from.rs$"],
  },
  {
    name: "std-try-conversion-unanswered",
    breaks: "a `TryFrom<&[u8]>` bound given an array of std's, `[u8; 2]`, is refused, its conversion no dictionary's (#1284)",
    file: "src/lower/traits/dictionaries/conversions.rs",
    find: "                && tried\n",
    replace: "                && false\n",
    tests: ["test/corpus.test.ts", "-t", "^conversions_through_generic_code.rs$"],
  },
  {
    name: "dyn-error-into-unanswered",
    breaks: "an `Into<Box<dyn Error + Send + Sync>>` bound given a message is refused (#1284)",
    file: "src/lower/traits/dictionaries/conversions.rs",
    find: "            let value = if !tried && let Some(dyn_error)",
    replace: "            let value = if false && let Some(dyn_error)",
    tests: ["test/corpus.test.ts", "-t", "^conversions_through_generic_code.rs$"],
  },
  {
    name: "library-conversion-uncompiled",
    breaks: "a library's `From` its JS exports, which a consumer's copy of its code converts `?`'s error with, is no conversion rust-js compiled, and is refused (#1420)",
    file: "src/lower/traits/dictionaries/conversions.rs",
    find: "        if !self.is_rust_fn(instance.def_id()) || self.tcx.trait_of_assoc(instance.def_id()).is_some() {\n",
    replace: "        if !self.krate.fns.contains_key(&instance.def_id()) || self.tcx.trait_of_assoc(instance.def_id()).is_some() {\n",
    tests: ["test/crates.test.ts", "-t", "instance_converts_and_transmutes"],
  },
];
