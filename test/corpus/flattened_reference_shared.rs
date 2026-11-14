// A flattened field that's a reference, as react.dev's runESLint's report
// holds its item (ADR 0204, amended): its parent's object holds a copy of
// the referent's keys, `{ ...item, severity }`, which can't be told from
// the referent itself where the referent is never changed in place nor
// compared by address, and has no cell of its own (#1819). What it holds
// behind a pointer, an `Rc<RefCell<..>>`, is the referent's own, shared:
// written through the source, read through the parent, and back.
use std::cell::RefCell;
use std::rc::Rc;

pub struct Item {
    pub line: u32,
    pub message: String,
    pub seen: Rc<RefCell<Vec<u32>>>,
}

pub struct Report<'a> {
    #[cfg_attr(rust_js, rust_js::flatten)]
    pub item: &'a Item,
    pub severity: u32,
}

fn describe(report: &Report) -> String {
    format!(
        "{}:{} {} {:?}",
        report.item.line,
        report.severity,
        report.item.message,
        report.item.seen.borrow()
    )
}

fn main() {
    let item = Item {
        line: 3,
        message: "unused".to_owned(),
        seen: Rc::new(RefCell::new(vec![1])),
    };
    let reports: Vec<Report> = (1..3).map(|severity| Report { item: &item, severity }).collect();
    item.seen.borrow_mut().push(2);
    reports[0].item.seen.borrow_mut().push(3);
    for report in &reports {
        println!("{}", describe(report));
    }
    println!("{} {:?}", item.line, item.seen.borrow());
}
