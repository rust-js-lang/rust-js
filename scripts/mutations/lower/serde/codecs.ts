// Mutations of src/lower/serde/codecs.rs (ADRs 0080, 0081, 0459).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "hand-written-codec-unadapted",
    breaks:
      "a hand-written impl's function, which gives a `Result`, is called as a codec, and what's written is the `Result`",
    file: "src/lower/serde/codecs.rs",
    find: '            false => Expr::call(Expr::helper("$jsonCodec"), vec![callee]),\n',
    replace: "            false => callee,\n",
    tests: ["test/serde.test.ts", "-t", "hand-written"],
  },
  {
    name: "codec-selected-without-type-args",
    breaks: "a type's codec is selected without its type arguments, so no impl of `Wrap<u32>` and `Wrap<String>` is chosen, or the first of its type is (#1550)",
    file: "src/lower/serde/codecs.rs",
    find: "            _ => ty.into(),\n",
    replace: "            _ => self.tcx.type_of(adt.did()).instantiate_identity().skip_normalization().into(),\n",
    tests: [
      "test/serde.test.ts",
      "-t",
      "impl rustc selects"
    ]
  },
  {
    name: "codec-given-the-types-first-argument",
    breaks: "a generic codec is given the writer or reader of its instance's first argument for each of its parameters, `String`'s for `T` of `Pair<String, T>` (#1550)",
    file: "src/lower/serde/codecs.rs",
    find: "        codec_type_params(own).map(|p| args.type_at(p.index as usize)).collect()\n",
    replace: "        codec_type_params(own).map(|_| args.type_at(0)).collect()\n",
    tests: [
      "test/serde.test.ts",
      "-t",
      "impl rustc selects"
    ]
  },
  {
    name: "item-serde-evidence-unasked",
    breaks: "generic code's codec of an associated type whose trait declares it `Serialize` isn't asked of its trait's dictionary, so it's refused (#1522)",
    file: "src/lower/serde/codecs.rs",
    find: "            .or_else(|| self.item_serde_evidence(ty, serialize))\n",
    replace: "",
    tests: ["test/serde-evidence.test.ts"],
  },
];
