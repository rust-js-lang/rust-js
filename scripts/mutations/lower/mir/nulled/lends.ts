// Mutations of src/lower/mir/nulled/lends.rs (ADR 0093): a `&mut` lend of
// an `Option` local that never takes its `Some` away (#1933 step 4 (B)).
import type { Mutation } from "../../../../mutations";

const keptLends = ["test/lowering-nullable-tests.test.ts", "-t", "a &mut lend that never takes a Some away"];

export const mutations: Mutation[] = [
  {
    name: "kept-lend-whole-write",
    breaks: "a write of the whole through the lend, `*lent = None`, is taken to keep its `Some`, so a nullable function returns `undefined` where Rust's `None` is `null` (#1933)",
    file: "src/lower/mir/nulled/lends.rs",
    find: "                    if !place.projection.is_empty() && !payload {\n",
    replace: "                    if false {\n",
    tests: keptLends,
  },
  {
    name: "kept-lend-any-call",
    breaks: "a lend given to any call, `found.take()`, is taken to keep its `Some` (#1933)",
    file: "src/lower/mir/nulled/lends.rs",
    find: "                    if at != 0 || !keeps || !args[0].node.place().is_some_and(|p| p.projection.is_empty()) {\n",
    replace: "                    if at != 0 || !args[0].node.place().is_some_and(|p| p.projection.is_empty()) {\n",
    tests: keptLends,
  },
  {
    name: "kept-lend-reborrowed",
    breaks: "a `&mut` reborrow of the lend, `fill(&mut slot, value)`'s, which may be given anywhere, is taken to keep its `Some`, so a `None` JS gave leaves as `null` where Rust's is `undefined` (#1933)",
    file: "src/lower/mir/nulled/lends.rs",
    find: "                        matches!(of.projection.as_slice(),\n                            [PlaceElem::Deref, PlaceElem::Downcast(_, variant), ..] if variant.as_u32() == 1)\n",
    replace: "                        reads_through(of)\n",
    tests: ["test/none-given.test.ts", "-t", "each source of JS's null"],
  },
];
