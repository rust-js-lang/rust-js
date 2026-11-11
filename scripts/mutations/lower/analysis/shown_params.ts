// Mutations of src/lower/analysis/shown_params.rs (#1331).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "shown-params-of-no-calls",
    breaks:
      "what a function gives another that shows it with options isn't shown so by it, so a library's `labelled` that gives `T` on to `pretty` marks nothing, and its consumer's `Labelled` shows `One(3)` on one line",
    file: "src/lower/analysis/shown_params.rs",
    find: "                    changed |= found.give(tcx, owner, ty, shown);\n",
    replace: "                    _ = (owner, ty, shown);\n",
    tests: ["test/crates.test.ts", "-t", "two crates: library_shows_a_consumers_value_with_options"],
  },
  {
    name: "shown-fields-unreached",
    breaks:
      "a type's fields aren't reached by what its `Debug` is shown with, so `Outer`'s `Plain` shows `n: 26` in its `{:#x?}`, where Rust shows `n: 1a`",
    file: "src/lower/analysis/shown_params.rs",
    find: "                    self.reach(tcx, field.ty(tcx, args).skip_normalization(), shown, seen);\n",
    replace: "                    _ = (field, args);\n",
    tests: ["test/crates.test.ts", "-t", "two crates: library_shows_a_consumers_value_with_options"],
  },
  {
    name: "dyn-debug-made-unshown",
    breaks: "a consumer's type made a `dyn Debug` for a library that shows one pretty or in hex gets a writer of no options (#1335)",
    file: "src/lower/analysis/shown_params.rs",
    find: "                        found.give(tcx, owner, made_of, found.dyn_debug_given);\n",
    replace: "",
    tests: ["test/crates.test.ts", "-t", "library_shows_a_consumers_dyn_debug_with_options"],
  },
  {
    name: "library-dyn-debug-unrecorded",
    breaks: "a library's manifest says it shows no `dyn Debug` with options, so its consumer's writers take none (#1335)",
    file: "src/lower/analysis/shown_params.rs",
    find: "            .fold(self.dyn_debug_given, |all, (_, &shown)| all.with(shown))\n",
    replace: "            .fold(Shown::default(), |all, (_, _)| all)\n",
    tests: ["test/crates.test.ts", "-t", "library_shows_a_consumers_dyn_debug_with_options"],
  },
];
