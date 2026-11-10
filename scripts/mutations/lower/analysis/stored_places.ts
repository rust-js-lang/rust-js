// Mutations of src/lower/analysis/stored_places.rs (ADR 0562, #1113).
import type { Mutation } from "../../../mutations";

const cases = ["test/corpus.test.ts", "-t", "^address_of_(a_field_where_its_object_is_stored|an_item_std_lends).rs$"];

export const mutations: Mutation[] = [
  {
    name: "stored-places-unasked",
    breaks: "a crate that asks an address is taken for one that asks none, so no object says where it's stored, and a field's address through a reference is its object's own allocation's (#1113)",
    file: "src/lower/analysis/stored_places.rs",
    find: "    if !bodies.iter().any(|body| asks_addresses(tcx, body)) {\n",
    replace: "    if true {\n",
    tests: cases,
  },
  {
    name: "stored-places-cast-unseen",
    breaks: "a pointer made an integer, `p as usize`, isn't an address asked, so a crate that asks only so records nothing (#1113)",
    file: "src/lower/analysis/stored_places.rs",
    find: "        ExprKind::Cast { source } => thir[source].ty.is_raw_ptr() && expr.ty.is_integral(),\n",
    replace: "        ExprKind::Cast { source } => thir[source].ty.is_raw_ptr() && expr.ty.is_integral() && false,\n",
    tests: cases,
  },
  {
    name: "stored-places-unclosed",
    breaks: "a struct no pointer passes through, holding one a pointer does, `Deep`'s `Two`, isn't recorded where it's stored, so its field's address is its own allocation's, not `24` past `Deep`'s (#1113)",
    file: "src/lower/analysis/stored_places.rs",
    find: "        if added.is_empty() {\n",
    replace: "        if true {\n",
    tests: ["test/corpus.test.ts", "-t", "^address_of_a_field_where_its_object_is_stored.rs$"],
  },
  {
    name: "slice-addresses-never-taken",
    breaks: "no crate is taken to take a slice's address, so its in-bounds parts are JS's own and say nothing of where they are",
    file: "src/lower/analysis/stored_places.rs",
    find: "    let slices = bodies.iter().any(|body| takes_slice_addresses(tcx, body));\n",
    replace: "    let slices = false;\n",
    tests: ["test/lowering-collections.test.ts", "-t", "a part of a slice is"],
  },
  {
    name: "slice-addresses-of-a-vec-pointer",
    breaks: "a raw pointer to a `Vec`, the one a `thread_local!`'s initializer is read through, is taken for a slice's address, so its crate's in-bounds parts are `$part(..)`, not JS's own `slice` (React's port's `list.slice(0, list.length - 1)`)",
    file: "src/lower/analysis/stored_places.rs",
    find: ".is_some_and(|(pointee, _)| items(pointee)) => true,\n",
    replace: ".is_some_and(|(pointee, _)| of_items(pointee)) => true,\n",
    tests: ["test/lowering-collections.test.ts", "-t", "a part of a slice is"],
  },
  {
    name: "slice-addresses-pointer-unseen",
    breaks: "a raw pointer to a slice, `ptr::eq` of two parts, isn't a slice's address taken, so the parts it compares are JS's own `slice`s, two arrays, and `ptr::eq` of two `&v[1..3]` is `false` (#1185)",
    file: "src/lower/analysis/stored_places.rs",
    find: ".is_some_and(|(pointee, _)| items(pointee)) => true,\n",
    replace: ".is_some_and(|(pointee, _)| items(pointee) && false) => true,\n",
    tests: ["test/lowering-collections.test.ts", "-t", "a part of a slice is"],
  },
];
