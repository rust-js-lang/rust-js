// Mutations of src/lower/drops/ref_cells/bound.rs (ADR 0093).
import type { Mutation } from "../../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "plain-refcell-bound-kept",
    breaks:
      "a struct's or a variant's `RefCell` field a pattern binds by value, moved out, `Outcome::Finished { lines, tries }` of `outcome`, is taken for one never lent, the property, where it's a cell of its own (ADR 0863, amended)",
    file: "src/lower/drops/ref_cells/bound.rs",
    find: "                    _ => self.found.push((did, field.pattern.span)),\n",
    replace: "                    _ => {}\n",
    tests: ["test/corpus.test.ts", "-t", "^refcell_variant_fields.rs$"],
    snapshots: true,
  },
  {
    name: "bound-place-lent",
    breaks: "a pattern's binding by reference of a field, `Kind::Effect { cleanup, .. }` of `&e.kind`, lends it, so the field is `$fieldCell(kind, \"cleanup\")`, where it's the field's own place (#503)",
    file: "src/lower/drops/ref_cells/bound.rs",
    find: "                    } if object_field(self.recognition.tcx, did) => _ = self.places.insert(var, did),\n",
    replace: "                    } if false => _ = self.places.insert(var, did),\n",
    tests: ["test/lowering-field-cells.test.ts", "-t", "an enum variant's RefCell field"],
  },
];
