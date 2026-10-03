// Ordinary Rust, end to end: stock and orders, as a person would write
// them. Items in a `HashMap`, grouped into a `BTreeMap` and sorted by two
// keys; orders parsed with `?` and `collect()` into a `Result`, placed
// or rolled back, errors as an enum with `Display`; a trait's default
// method; a `VecDeque` of work; tables by width and alignment.
use std::collections::{BTreeMap, HashMap, HashSet, VecDeque};
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
enum Category {
    Tools,
    Garden,
    Kitchen,
}

impl fmt::Display for Category {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let name = match self {
            Category::Tools => "tools",
            Category::Garden => "garden",
            Category::Kitchen => "kitchen",
        };
        f.pad(name)
    }
}

#[derive(Debug, Clone, PartialEq)]
struct Item {
    sku: String,
    name: String,
    category: Category,
    price_cents: u32,
    stock: u32,
}

#[derive(Debug, PartialEq)]
enum StockError {
    UnknownSku(String),
    OutOfStock { sku: String, wanted: u32, have: u32 },
    EmptyOrder,
}

impl fmt::Display for StockError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            StockError::UnknownSku(sku) => write!(f, "no item {sku}"),
            StockError::OutOfStock { sku, wanted, have } => {
                write!(f, "{sku}: wanted {wanted}, have {have}")
            }
            StockError::EmptyOrder => write!(f, "an order needs a line"),
        }
    }
}

trait Priced {
    fn unit_cents(&self) -> u32;
    fn quantity(&self) -> u32;
    fn total_cents(&self) -> u64 {
        self.unit_cents() as u64 * self.quantity() as u64
    }
}

struct Line {
    sku: String,
    quantity: u32,
    unit_cents: u32,
}

impl Priced for Line {
    fn unit_cents(&self) -> u32 {
        self.unit_cents
    }
    fn quantity(&self) -> u32 {
        self.quantity
    }
}

struct Inventory {
    items: HashMap<String, Item>,
    log: Vec<String>,
}

fn money(cents: u64) -> String {
    format!("${}.{:02}", cents / 100, cents % 100)
}

impl Inventory {
    fn new(items: Vec<Item>) -> Self {
        let items = items.into_iter().map(|item| (item.sku.clone(), item)).collect();
        Inventory { items, log: Vec::new() }
    }

    fn reserve(&mut self, sku: &str, wanted: u32) -> Result<Line, StockError> {
        let item = self
            .items
            .get_mut(sku)
            .ok_or_else(|| StockError::UnknownSku(sku.to_string()))?;
        if item.stock < wanted {
            return Err(StockError::OutOfStock { sku: sku.to_string(), wanted, have: item.stock });
        }
        item.stock -= wanted;
        self.log.push(format!("reserved {wanted} of {sku}"));
        Ok(Line { sku: sku.to_string(), quantity: wanted, unit_cents: item.price_cents })
    }

    fn place(&mut self, order: &[(&str, u32)]) -> Result<Vec<Line>, StockError> {
        if order.is_empty() {
            return Err(StockError::EmptyOrder);
        }
        let mut lines = Vec::new();
        for &(sku, quantity) in order {
            match self.reserve(sku, quantity) {
                Ok(line) => lines.push(line),
                Err(error) => {
                    // Put back what this order took before failing.
                    for line in &lines {
                        if let Some(item) = self.items.get_mut(&line.sku) {
                            item.stock += line.quantity;
                        }
                    }
                    self.log.push(format!("rolled back {} lines", lines.len()));
                    return Err(error);
                }
            }
        }
        Ok(lines)
    }

    fn by_category(&self) -> BTreeMap<Category, Vec<&Item>> {
        let mut groups: BTreeMap<Category, Vec<&Item>> = BTreeMap::new();
        for item in self.items.values() {
            groups.entry(item.category).or_default().push(item);
        }
        for items in groups.values_mut() {
            items.sort_by(|a, b| b.price_cents.cmp(&a.price_cents).then_with(|| a.name.cmp(&b.name)));
        }
        groups
    }

    fn low_stock(&self, below: u32) -> Vec<String> {
        let mut skus: Vec<String> = self
            .items
            .values()
            .filter(|item| item.stock < below)
            .map(|item| item.sku.clone())
            .collect();
        skus.sort();
        skus
    }

    fn value_cents(&self) -> u64 {
        self.items.values().map(|item| item.price_cents as u64 * item.stock as u64).sum()
    }
}

fn parse_order(text: &str) -> Result<Vec<(String, u32)>, String> {
    text.split(',')
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .map(|part| {
            let (sku, quantity) = part.split_once('x').ok_or(format!("{part:?} has no x"))?;
            let quantity: u32 = quantity.trim().parse().map_err(|e| format!("{part:?}: {e}"))?;
            Ok((sku.trim().to_uppercase(), quantity))
        })
        .collect()
}

fn main() {
    let mut inventory = Inventory::new(vec![
        Item { sku: "HAM-1".into(), name: "Hammer".into(), category: Category::Tools, price_cents: 1299, stock: 10 },
        Item { sku: "SAW-2".into(), name: "Saw".into(), category: Category::Tools, price_cents: 2450, stock: 3 },
        Item { sku: "RAK-1".into(), name: "Rake".into(), category: Category::Garden, price_cents: 1899, stock: 5 },
        Item { sku: "HOS-9".into(), name: "Hose".into(), category: Category::Garden, price_cents: 1899, stock: 2 },
        Item { sku: "PAN-3".into(), name: "Pan".into(), category: Category::Kitchen, price_cents: 3500, stock: 4 },
        Item { sku: "KNF-4".into(), name: "Knife".into(), category: Category::Kitchen, price_cents: 999, stock: 0 },
    ]);
    println!("stock value: {}", money(inventory.value_cents()));

    for (category, items) in inventory.by_category() {
        println!("[{category:^9}]");
        for item in items {
            println!("  {:<8}|{:>8}|{:>4}", item.name, money(item.price_cents as u64), item.stock);
        }
    }

    let mut queue: VecDeque<&str> = VecDeque::from(vec!["ham-1 x 2, saw-2 x1", "rak-1 x 9", "", "pan-3 x two", "hos-9x2,ham-1x1", "zzz-0 x1"]);
    let mut totals = Vec::new();
    while let Some(text) = queue.pop_front() {
        let parsed = match parse_order(text) {
            Ok(parsed) => parsed,
            Err(message) => {
                println!("bad order: {message}");
                continue;
            }
        };
        let order: Vec<(&str, u32)> = parsed.iter().map(|(sku, n)| (sku.as_str(), *n)).collect();
        match inventory.place(&order) {
            Ok(lines) => {
                let total: u64 = lines.iter().map(Priced::total_cents).sum();
                totals.push(total);
                let skus: Vec<&str> = lines.iter().map(|line| line.sku.as_str()).collect();
                println!("placed {:?} for {}", skus, money(total));
            }
            Err(error) => println!("refused: {error} ({error:?})"),
        }
    }

    println!("low stock: {:?}", inventory.low_stock(3));
    println!("log: {}", inventory.log.join("; "));
    let mean = totals.iter().sum::<u64>() as f64 / totals.len() as f64 / 100.0;
    println!("orders: {}, mean {:.2}, max {:?}", totals.len(), mean, totals.iter().max().map(|c| money(*c)));

    let mut seen = HashSet::new();
    let mut names: Vec<String> = inventory.items.values().map(|item| item.name.to_lowercase()).collect();
    names.sort_by_key(|name| (name.len(), name.clone()));
    names.retain(|name| seen.insert(name.chars().next().unwrap()));
    println!("first letters: {}", names.join(","));

    let mut prices: Vec<u32> = inventory.items.values().map(|item| item.price_cents).collect();
    prices.sort_unstable();
    prices.dedup();
    println!("prices: {:?}, 1899 at {:?}, 2000 at {:?}", prices, prices.binary_search(&1899), prices.binary_search(&2000));

    let restock: HashMap<&str, u32> = [("KNF-4", 6), ("SAW-2", 2)].into_iter().collect();
    let mut restocked: Vec<(String, u32)> = inventory
        .items
        .iter_mut()
        .filter_map(|(sku, item)| {
            let more = *restock.get(sku.as_str())?;
            item.stock += more;
            Some((sku.clone(), item.stock))
        })
        .collect();
    restocked.sort();
    println!("restocked: {restocked:?}, value now {}", money(inventory.value_cents()));
}
