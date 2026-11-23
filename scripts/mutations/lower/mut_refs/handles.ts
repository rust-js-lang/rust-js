// Mutations of crates/rust-js-lower/src/mut_refs/handles.rs, a handle on a
// `&mut`'s place and a `dyn`'s pair on one, each stored as the place
// stores (#2427, #977).
import type { Mutation } from "../../../mutations";

const file = "crates/rust-js-lower/src/mut_refs/handles.rs";
const lent = ["test/corpus.test.ts", "-t", "^nullable_field_lent_as_a_handle.rs$"];
const paired = ["test/corpus.test.ts", "-t", "^stored_place_made_a_mut_dyn.rs$"];

export const mutations: Mutation[] = [
  {
    name: "handle-on-a-held-handle-made-apart",
    breaks: "a held handle's `value` lent on is given a handle of its own, `{ set value(value) { s.v = value; } }`, which stores `undefined` in a nullable field (#2427)",
    file,
    find: "            (Some(_), js::ExprKind::Member(handle, _)) => (**handle).clone(),\n",
    replace: "            (Some(_), js::ExprKind::Member(..)) => Expr::handle(place.clone()),\n",
    tests: lent,
  },
  {
    name: "pair-on-a-held-handle-stored-as-given",
    breaks: "a `&mut dyn`'s pair on a nullable field's held handle assigns the field as it's given, `fiber.owner = value`, so a `None` is `undefined` (#977, #2427)",
    file,
    find: "            Some(handled) => Expr::stored_pair(handled.on.clone(), dictionary, handled.storage),\n",
    replace: "            Some(_) => Expr::pair(place, dictionary),\n",
    tests: paired,
  },
  {
    name: "pair-made-after-its-handle-renamed",
    breaks: "a `&mut dyn`'s pair asks whether its place is a held handle's after `place_where_made` names the handle again, so it's never found and assigns the field as it's given (#2427)",
    file,
    find: "        match self.held_handle(&place).is_some() {\n",
    replace: "        match false {\n",
    tests: paired,
  },
  {
    name: "reborrowed-pair-place-unfixed",
    breaks: "a `&mut dyn` pair of a reborrow, `&mut *x`, reads `x.value` whenever it's read, so `x = &mut q` before it's used writes `q` (#968)",
    file,
    find: "                let place = self.place_where_made(place, out);\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^dyn_of_a_generic_impl_written_whole.rs$"],
  },
];
