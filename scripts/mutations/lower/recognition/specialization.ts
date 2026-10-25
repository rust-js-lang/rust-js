// Mutations of src/lower/recognition/specialization.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "fused-specialization-unseen",
    breaks: "a std-private trait's method whose impl rustc leaves to the caller's type, `FuseImpl::next` of a `Fuse<I>`, isn't seen to be picked by `I` being a `FusedIterator`, and is refused",
    file: "src/lower/recognition/specialization.rs",
    find: "    let specializing = tcx.all_impls(trait_id).find(|&other| {\n        other != impl_id\n",
    replace: "    let specializing = tcx.all_impls(trait_id).find(|&other| {\n        false\n            && other != impl_id\n",
    tests: ["test/std-package.test.ts"],
  },
  {
    name: "one-shot-optimizing",
    breaks: "`OneShot`'s specialization of `FlattenCompat` is taken for an optimization, so a flatten of `Option`s is sized `(0, None)` where std's is `(0, upper)`",
    file: "src/lower/recognition/specialization.rs",
    find: "const OPTIMIZING: &[&str] = &[\"TrustedLen\"];\n",
    replace: "const OPTIMIZING: &[&str] = &[\"OneShot\", \"TrustedLen\"];\n",
    tests: [
      "test/std-package.test.ts"
    ]
  },
];
