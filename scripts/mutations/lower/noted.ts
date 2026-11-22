// Mutations of src/lower/noted.rs (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "mir-plain-of-bound-vars",
    breaks:
      "a local's type inside a binder, a function pointer's parameter, is asked whether it's `Freeze`, which crashes rustc",
    file: "src/lower/noted.rs",
    find: "                    && !t.has_escaping_bound_vars()\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "fn_pointer_of_borrowed_struct"],
  },
  {
    name: "mir-counted-rc-plain",
    breaks:
      "MIR's `Arc::strong_count(&owner)` is read where it's shown, after a later `upgrade()` counted another",
    file: "src/lower/noted.rs",
    find: "                    && self.counted_rc(t).is_none()\n                    && self.counted_weak(t).is_none()\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "rc_counted_uses"],
  },
  {
    name: "in-place-sources-of-the-last-body",
    breaks: "each body's in-place collect sources replace its enclosing body's, so a closure lowered inside `main` leaves `main`'s collects with none, their capacity unknown (#1259)",
    file: "src/lower/noted.rs",
    find: "        self.locals.mir.in_place_sources.extend(names);\n",
    replace: "        self.locals.mir.in_place_sources = names;\n",
    tests: ["test/corpus.test.ts", "-t", "^vec_collected_in_place_capacity.rs$"],
  },
];
