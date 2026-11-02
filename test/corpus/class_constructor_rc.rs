// A class's constructor that gives its instance shared, `Rc<Self>`, as
// React's Flight server's `RequestInstance` hands its root task the
// request it makes (rd554, ADR 0761 amended): the `Rc` is the instance,
// `this`, which each task it makes holds, an early return gives too, and
// `Request::new(..)` is `new RequestInstance(..)`. What Rust reads of it is
// what it does natively.
use std::cell::{Cell, RefCell};
use std::rc::Rc;

struct Task {
    request: Rc<Request>,
    id: u32,
}

#[cfg_attr(rust_js, rust_js::class, rust_js::name = "RequestInstance")]
struct Request {
    kind: u8,
    next_id: Cell<u32>,
    pinged_tasks: RefCell<Vec<Rc<Task>>>,
}

fn create_task(request: &Rc<Request>) -> Rc<Task> {
    let id = request.next_id.get();
    request.next_id.set(id + 1);
    Rc::new(Task {
        request: Rc::clone(request),
        id,
    })
}

#[cfg_attr(rust_js, rust_js::class)]
impl Request {
    fn new(
        kind: u8,
        tasks: u32, // the tasks it starts with
    ) -> Rc<Self> {
        let request = Rc::new(Request {
            kind,
            next_id: Cell::new(0),
            pinged_tasks: RefCell::new(vec![]),
        });
        if kind == 0 {
            return request;
        }
        for _ in 0..tasks {
            let task = create_task(&request);
            request.pinged_tasks.borrow_mut().push(task);
        }
        request
    }

    fn shown(&self) -> String {
        let tasks = self.pinged_tasks.borrow();
        let ids: Vec<u32> = tasks.iter().map(|task| task.id).collect();
        let held = tasks.iter().all(|task| std::ptr::eq(&*task.request, self));
        format!("{} {:?} {} {}", self.kind, ids, self.next_id.get(), held)
    }
}

#[cfg_attr(rust_js, rust_js::class)]
struct Hints {
    count: Cell<u32>,
}

#[cfg_attr(rust_js, rust_js::class)]
impl Hints {
    fn new() -> Rc<Self> {
        Rc::new(Hints { count: Cell::new(0) })
    }
}

fn main() {
    for (kind, tasks) in [(0, 2), (1, 3), (2, 0)] {
        let request = Request::new(kind, tasks);
        println!("{}", request.shown());
        let task = create_task(&request);
        println!("{} {}", task.id, task.request.next_id.get());
    }
    let hints = Hints::new();
    hints.count.set(hints.count.get() + 2);
    println!("{}", hints.count.get());
}
