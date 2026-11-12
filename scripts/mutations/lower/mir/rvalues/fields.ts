// Mutations of src/lower/mir/rvalues/fields.rs (ADR 0093), split from
// rvalues.rs with it (#1334).
import type { Mutation } from "../../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "field-offset-unsaid",
    breaks: "a field's place says no offset, so a raw pointer to it is no address of where it is on its object (#1114)",
    file: "src/lower/mir/rvalues/fields.rs",
    find: "            Some(offset) => Offset::At(offset),\n",
    replace: "            Some(_) => Offset::Unsaid,\n",
    tests: ["test/corpus.test.ts", "-t", "^address_of_a_field_through_a_reference.rs$"]
  },
  {
    name: "whole-property-cell-never",
    breaks: "a whole field that's its cell's property, a `Cell`'s, isn't read as a handle on it nor written as what it holds",
    file: "src/lower/mir/rvalues/fields.rs",
    find: "        (self.is_std_type(ty, StdItem::Cell) || self.is_std_type(ty, StdItem::RefCell))\n",
    replace: "        false\n",
    tests: ["test/corpus.test.ts", "-t", "^refcell_(field_cells|field_places|variant_fields).rs$"]
  },
];
