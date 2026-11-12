// Mutations of src/lower/display/composed.rs (ADR 0093): a std value's
// `Debug` of a part that may stop part-way, through std's builders (#1561).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "composed-may-stop-never",
    breaks: "a std value's `Debug` of a part that may give `Err` is made whole, so the `String` it's written into loses `Some(` and `[None, ` before the part (#1561)",
    file: "src/lower/display/composed.rs",
    find: "        !local && (self.krate.prints.stops_composed(ty).is_some() || self.fmt_may_fail(self.debug_trait(), ty))\n",
    replace: "        !local && self.krate.prints.stops_composed(ty).is_some()\n",
    tests: ["test/corpus.test.ts", "-t", "^failing_std_value_debug_keeps_what_was_written.rs$"],
  },
  {
    name: "composed-map-keys-as-values",
    breaks: "a map's `Debug` through std's builders shows each entry's value as its key",
    file: "src/lower/display/composed.rs",
    find: "                _ => Expr::index(Expr::var(\"entry\"), Expr::int(i as i128)),\n",
    replace: "                _ => Expr::index(Expr::var(\"entry\"), Expr::int(0)),\n",
    tests: ["test/corpus.test.ts", "-t", "^(failing_std_value_debug_keeps_what_was_written|printing_std_value_stopped_part_way).rs$"],
  },
];
