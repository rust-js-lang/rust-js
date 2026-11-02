import { expect, test } from "bun:test";
import { writeFileSync } from "node:fs";
import { join } from "node:path";
import { compiler, fixture, run } from "./support";

test("a Result field holding a mutable map entry replaces its pointee", async () => {
  const dir = fixture("result-mut-map-replacement");
  const source = `use std::collections::{BTreeMap, HashMap};

struct Account { balance: u32 }
struct Holder<'a> { account: &'a mut Account }

pub fn scenario() -> Result<Vec<u32>, String> {
    let mut tree = BTreeMap::from([("demo", Account { balance: 0 })]);
    *tree.get_mut("demo").ok_or_else(|| "missing".to_string())? = Account { balance: 42 };
    let mut hash = HashMap::from([("demo", Account { balance: 0 })]);
    *hash.get_mut("demo").ok_or_else(|| "missing".to_string())? = Account { balance: 43 };
    let mut option = Some(Account { balance: 0 });
    *option.as_mut().ok_or_else(|| "missing".to_string())? = Account { balance: 44 };
    let mut account = Account { balance: 0 };
    let holder = Holder { account: &mut account };
    *holder.account = Account { balance: 45 };
    let mut values = vec![Account { balance: 0 }];
    values[0] = Account { balance: 46 };
    Ok(vec![tree["demo"].balance, hash["demo"].balance, option.unwrap().balance, account.balance, values[0].balance])
}

fn main() { println!("{:?}", scenario().unwrap()); }
`;
  writeFileSync(join(dir, "lib.rs"), source);
  run(["rustc", "--edition=2024", join(dir, "lib.rs"), "-o", join(dir, "native")]);
  const expected = JSON.parse(run([join(dir, "native")]));
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const app = await import(join(dir, "lib.js"));
  expect(expected).toEqual([42, 43, 44, 45, 46]);
  expect(app.scenario()).toEqual({ TAG: "Ok", _0: expected });
});
