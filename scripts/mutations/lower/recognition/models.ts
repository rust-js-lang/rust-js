// Mutations of crates/rust-js-lower/src/recognition/models.rs (ADR 0093):
// the one catalog of the runtime's models (Campaign 5's step 1, ADR 1202).
import type { Mutation } from "../../../mutations";

const file = "crates/rust-js-lower/src/recognition/models.rs";

export const mutations: Mutation[] = [
  {
    name: "models-trait-method-unresolved",
    breaks: "a model's std item that's a trait's method, `DoubleEndedIterator::rfold`, names nothing where `bun run std` reads std, so it fails there",
    file,
    find: "        DefKind::Trait => tcx.associated_item_def_ids(owner).to_vec(),\n",
    replace: "        DefKind::Trait => Vec::new(),\n",
    tests: ["test/std-sites.test.ts"],
  },
  {
    name: "models-glue-as-a-model",
    breaks: "glue, rust-js's own, is printed as a model standing for no std item, so the count of what's left to bind (#2513) is past its ratchet",
    file,
    find: '                Glue => ("glue", false, Some("glue"), &[]),\n',
    replace: '                Glue => ("model", false, Some("glue"), &[]),\n',
    tests: ["test/twin-net.test.ts", "-t", "how many models stand for no std item yet"],
  },
  {
    name: "models-pitted-unsaid",
    breaks: "a pitted model is printed unproven, so the twin net takes no helper for pitted and its verdict reads every pitted helper as a gap",
    file,
    find: '                Model(Pitted, stands) => ("model", true, None, stands),\n',
    replace: '                Model(Pitted, stands) => ("model", false, Some("iterators"), stands),\n',
    tests: ["test/twin-net.test.ts", "-t", "a gap fails a run"],
  },
  {
    name: "models-row-said-written",
    breaks: "a model's row of `$stdSites` none writes isn't said, so its sites would be `undefined` where `bun run std` should refuse the catalog (Campaign 5 step 2a)",
    file,
    find: "                && !written(row)\n",
    replace: "                && written(row)\n",
    tests: ["test/std-sites.test.ts"],
  },
  {
    name: "models-form-as-a-helper",
    breaks: "a form the lowering writes is printed as a runtime helper, `$clonedAt`, which the runtime doesn't declare",
    file,
    find: "                Form(_) => entry.helper.to_string(),\n",
    replace: "                Form(_) => format!(\"${}\", entry.helper),\n",
    tests: ["test/twin-net.test.ts", "-t", "each helper the catalog names is one the runtime declares"],
  },
];
