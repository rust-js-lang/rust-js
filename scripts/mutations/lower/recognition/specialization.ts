// Mutations of src/lower/recognition/specialization.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "held-facts-unreduced",
    breaks: "`TrustedLen` of a `&mut I` isn't `I`'s, as std's impl for it asks: no caller gives it, refused (ADR 0975)",
    file: "src/lower/recognition/specialization.rs",
    find: "    let [(imp, params)] = &impls[..] else {\n",
    replace: "    let [(imp, params), _] = &impls[..] else {\n",
    tests: ["test/iterator-forms.test.ts", "-t", "^copied_default_own_params with"],
  },
  {
    name: "never-returning-default-picked",
    breaks: "a default that only panics, std's general `try_get_unchecked`, is compiled as one the facts pick: its inlined `panic!` message is a raw pointer, refused (ADR 0975)",
    file: "src/lower/recognition/specialization.rs",
    find: "    if never_returns(tcx, selected) {\n",
    replace: "    if false && never_returns(tcx, selected) {\n",
    tests: ["test/std-package.test.ts"],
  },
  {
    name: "zip-specialization-unfacted",
    breaks: "a zip's sides' `TrustedRandomAccess` picks no impl: std's `ZipImpl` is left to no fact, so std's compiled `Zip` is refused (ADR 0975)",
    file: "src/lower/recognition/specialization.rs",
    find: "        (\"ZipImpl\", TypeFact::RandomAccess),\n",
    replace: "",
    tests: ["test/std-package.test.ts"],
  },
  {
    name: "assume-count-specialized",
    breaks: "std's `Filter::count` asks `SpecAssumeCount`'s `TrustedLen` impl, which rust-js refuses: std's bodies don't compile (ADR 0932)",
    file: "src/lower/recognition/specialization.rs",
    find: "const OPTIMIZING_BY_TYPE: &[&str] = &[\"SpecRangeSetup\", \"StepByImpl\", \"StepByBackImpl\", \"SpecAssumeCount\"];",
    replace: "const OPTIMIZING_BY_TYPE: &[&str] = &[\"SpecRangeSetup\", \"StepByImpl\", \"StepByBackImpl\"];",
    tests: ["test/std-package.test.ts"],
  },
  {
    name: "fused-specialization-unseen",
    breaks: "a std-private trait's method whose impl rustc leaves to the caller's type, `FuseImpl::next` of a `Fuse<I>`, isn't seen to be picked by `I` being a `FusedIterator`, and is refused",
    file: "src/lower/recognition/specialization.rs",
    find: "        .filter(|&other| {\n            other != impl_id\n",
    replace: "        .filter(|&other| {\n            false\n                && other != impl_id\n",
    tests: ["test/std-package.test.ts"],
  },
  {
    name: "step-by-specializations-unoptimizing",
    breaks: "std's `StepBy` range specializations aren't read as optimizations, so its `SpecRangeSetup::setup` can't be called in generic code (ADR 0917)",
    file: "src/lower/recognition/specialization.rs",
    find: "const OPTIMIZING_BY_TYPE: &[&str] = &[\"SpecRangeSetup\", \"StepByImpl\", \"StepByBackImpl\", \"SpecAssumeCount\"];\n",
    replace: "const OPTIMIZING_BY_TYPE: &[&str] = &[\"SpecAssumeCount\"];\n",
    tests: ["test/std-package.test.ts", "-t", "the committed @rust-js/std is std's bodies"],
  },
];
