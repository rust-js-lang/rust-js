// Mutations of crates/rust-js-lower/src/recognition/open_calls.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "new-in-1-100-unnamed",
    breaks: "a refused call of a std item Rust 1.100 stabilized names no issue, where its tracker, #1656, says what's to come (ADR 0109, amended)",
    file: "crates/rust-js-lower/src/recognition/open_calls.rs",
    find: "            } if (version.major, version.minor) == (1, 100))\n",
    replace: "            } if (version.major, version.minor) == (0, 0))\n",
    tests: ["test/diagnostics.test.ts", "-t", "new in 1.100"],
  },
  {
    name: "dyn-of-new-in-1-100-unnamed",
    breaks: "a refused value of a `dyn` of a trait Rust 1.100 stabilized, `dyn Allocator`, names no issue, where its trait's tracker, #1656, says what's to come (ADR 0109, amended)",
    file: "crates/rust-js-lower/src/recognition/open_calls.rs",
    find: "        ty::Dynamic(predicates, ..) => predicates.principal_def_id()?,\n",
    replace: "",
    tests: ["test/diagnostics.test.ts", "-t", "a dyn of a std trait new in 1.100"],
  },
  {
    name: "thread-spawn-not-open",
    breaks: "`thread::spawn` and `thread::scope` are no open call of a thread's, so a closure they're given is taken for one called here",
    file: "crates/rust-js-lower/src/recognition/open_calls.rs",
    find: "    (spawns && super::std_path_under(tcx, id, StdPath::ThreadMod)).then_some(OpenCall::Thread)\n",
    replace: "    (spawns && false).then_some(OpenCall::Thread)\n",
    tests: ["test/diagnostics.test.ts"],
  },
  {
    name: "serde-json-error-api-untagged",
    breaks: "`serde_json::Error::line` is refused naming no issue, where it's #1627's",
    file: "crates/rust-js-lower/src/recognition/open_calls.rs",
    find: "        Some(\"Error\") => Some(OpenCall::SerdeJsonErrorApi),\n",
    replace: "        Some(\"Error\") => None,\n",
    tests: ["test/diagnostics.test.ts", "-t", "serde_json's Error's line"],
  },
  {
    name: "serde-json-stream-untagged",
    breaks: "a serde_json `Deserializer`'s stream is refused naming no issue, where it's #1626's",
    file: "crates/rust-js-lower/src/recognition/open_calls.rs",
    find: "        Some(\"Deserializer\" | \"StreamDeserializer\" | \"Serializer\") => Some(OpenCall::SerdeJsonBytes),\n",
    replace: "        Some(\"Deserializer\" | \"StreamDeserializer\" | \"Serializer\") => None,\n",
    tests: ["test/diagnostics.test.ts", "-t", "serde_json's stream of values"],
  },
  {
    name: "net-text-untagged",
    breaks: "`{}` of an `Ipv4Addr` is refused naming no issue, where it's #1644's",
    file: "crates/rust-js-lower/src/recognition/open_calls.rs",
    find: "    .then_some(OpenCall::NetText)\n",
    replace: "    .then_some(OpenCall::NetText)\n    .filter(|_| false)\n",
    tests: ["test/diagnostics.test.ts", "-t", "of an Ipv4Addr"],
  },
  {
    name: "open-impl-unfound",
    breaks: "`Default` of a `Once` is refused naming no issue, where its impl is 1.100's (#1656)",
    file: "crates/rust-js-lower/src/recognition/open_calls.rs",
    find: "        .find_map(|imp| open_item(tcx, imp))\n",
    replace: "        .find_map(|_| None)\n",
    tests: ["test/diagnostics.test.ts", "-t", "Default of a Once, new in 1.100"],
  },
];
