// Mutations of src/lower/traits/dictionaries/conversions.rs (ADR 0093).
import type { Mutation } from "../../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "into-without-from",
    breaks:
      "`Into<Name>` of a `&str` isn't the crate's `From<&str> for Name`, and is refused",
    file: "src/lower/traits/dictionaries/conversions.rs",
    find: "            } else if let Some(instance) = self.resolve_instance(from, from_args)?\n                && self.is_rust_fn(instance.def_id())\n",
    replace:
      "            } else if let Some(instance) = self.resolve_instance(from, from_args)?\n                && false\n",
    tests: ["test/corpus.test.ts", "-t", "generic_into"],
  },
  {
    name: "from-dictionary-named-into",
    breaks: "a `From` dictionary's conversion is named `into`",
    file: "src/lower/traits/dictionaries/conversions.rs",
    find: '            let name = if into { "into" } else { "from" };\n',
    replace: '            let name = "into";\n',
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
];
