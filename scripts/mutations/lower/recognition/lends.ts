// Mutations of src/lower/recognition/lends.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "once-cell-lent-at-its-option",
    breaks: "a `OnceCell`'s lent value's offset is its `Option`'s, not past its tag, so its pointer is short of native's (#1474)",
    file: "src/lower/recognition/lends.rs",
    find: "                Some(at + some(option)?)\n",
    replace: "                Some(at)\n",
    tests: ["test/corpus.test.ts","-t","^address_of_a_mut_std_lends_from_its_storage.rs$"],
  },
  {
    name: "lazy-cell-lent-at-its-state",
    breaks: "a `LazyCell`'s lent value's offset is its state's, not its `Init`'s payload's (#1474)",
    file: "src/lower/recognition/lends.rs",
    find: "                Some(at + self.variant_field_offset(state, init, FieldIdx::ZERO)?)\n",
    replace: "                Some(at)\n",
    tests: ["test/corpus.test.ts","-t","^address_of_a_mut_std_lends_from_its_storage.rs$"],
  },
  {
    name: "peekable-lent-past-one-option",
    breaks: "a `Peekable`'s lent item's offset is past its outer `Option`'s tag alone, not its inner one's too (#1474)",
    file: "src/lower/recognition/lends.rs",
    find: "                Some(at + some(outer)? + some(inner)?)\n",
    replace: "                Some(at + some(outer)?)\n",
    tests: ["test/corpus.test.ts","-t","^address_of_a_mut_std_lends_from_its_storage.rs$"],
  },
  {
    name: "result-err-lent-at-ok",
    breaks: "a `Result`'s lent `Err` payload is taken at its `Ok`'s offset (#1474)",
    file: "src/lower/recognition/lends.rs",
    find: "            Lend::Err => self.variant_field_offset(receiver, VariantIdx::from_u32(1), FieldIdx::ZERO),\n",
    replace: "            Lend::Err => self.variant_field_offset(receiver, VariantIdx::from_u32(0), FieldIdx::ZERO),\n",
    tests: ["test/corpus.test.ts","-t","^address_of_a_mut_std_lends_from_its_storage.rs$"],
  },
  {
    name: "layout-in-function-environment",
    breaks:
      "`size_of_val` of a generic `async fn`'s future is laid out in an environment of its own, not as codegen asks,, where 1.98 finds it too generic, and is refused",
    file: "src/lower/recognition/lends.rs",
    find: "            .layout_of(ty::TypingEnv::fully_monomorphized().as_query_input(ty))\n",
    replace: "            .layout_of(ty::TypingEnv::non_body_analysis(self.tcx, rustc_hir::def_id::CRATE_DEF_ID).as_query_input(ty))\n",
    tests: ["test/corpus.test.ts", "-t", "future_sizes"],
  },
];
