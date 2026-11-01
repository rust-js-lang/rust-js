// Mutations of src/lower/mir/alternatives.rs (ADR 0093): an or-pattern's
// alternatives JS holds as one (rd430).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "alike-alternatives-apart",
    breaks: "an or-pattern's alternatives of one JS place each set a variable, a `&mut`'s a getter and setter box, a `match`'s a labeled block (rd430)",
    file: "src/lower/mir/alternatives.rs",
    find: "                    alike.held_as.insert(later, earlier);\n",
    replace: "",
    tests: ["test/classes-flattened-state.test.ts", "-t", "an or-pattern of a field each variant has"],
  },
  {
    name: "alike-alternatives-of-any-field",
    breaks: "an or-pattern's alternatives of fields at other places, `Circle { r } | Ring(r)`, are one, a `Ring`'s `r` read as `shape.r`, undefined (rd430)",
    file: "src/lower/mir/alternatives.rs",
    find: "                    if !self.one_variant_place(of, (vx, fx), (vy, fy)) {\n",
    replace: "                    if false {\n",
    tests: ["test/corpus.test.ts", "-t", "or_pattern_one_place"],
  },
  {
    name: "alike-alternatives-moved",
    breaks: "an or-pattern's alternatives that move a part out are one, so what's left of the variant drops as the earlier alternative's moved, `drop a1` before it's used (rd430)",
    file: "src/lower/mir/alternatives.rs",
    find: "            (Operand::Copy(x), Operand::Copy(y)) => place(x, y),\n",
    replace: "            (Operand::Copy(x), Operand::Copy(y)) | (Operand::Move(x), Operand::Move(y)) => place(x, y),\n",
    tests: ["test/corpus.test.ts", "-t", "or_pattern_part_moves"],
  },
  {
    name: "alike-alternatives-written-alike",
    breaks: "two arms written alike, of no variant's field, are one, their tests joined where the program wrote each, React's getArrayKind's `typeof value === \"object\"` arm (rd430)",
    file: "src/lower/mir/alternatives.rs",
    find: "            && differ.get()\n",
    replace: "",
    tests: ["test/port-performance-track.test.ts", "-t", "getArrayKind"],
  },
];
