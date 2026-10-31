// Mutations of src/lower/traits/evidence.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "default-dictionary-any-step",
    breaks:
      "a call leaves out a dictionary at its end whose entry is the default's but whose step isn't, a range's `{ next: $rangeNext }`, so the callee steps the range as a JS iterator (ADR 0995)",
    file: "src/lower/traits/evidence.rs",
    find: "if matches!(props.as_slice(), [js::Prop::Field(name, value)] if name == entry && value.helper_name() == Some(helper)))",
    replace:
      'if matches!(props.as_slice(), [js::Prop::Field(name, value)] if name == entry && value.helper_name().is_some() && helper != ""))',
    tests: [
      "test/corpus.test.ts",
      "-t",
      "^(std_map|cloned_counted_adapters|associated_iterator_steps)\\.rs$",
    ],
  },
];
