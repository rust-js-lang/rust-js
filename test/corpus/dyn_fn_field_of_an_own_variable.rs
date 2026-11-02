// A field written a variable bound once to the crate's own closure, React's
// Flight reply's `FlightStreamController { enqueue_model, close, error }`
// of `let error: Rc<Listen> = Rc::new(move |error| ..)`, holds that closure,
// which reads no `this`: called through its object, `controller.error(4)`,
// as upstream writes it. A binding's function of the same signature,
// `Math.abs`, is a `dyn Fn` the program holds too, which may read the
// `this` JS gives it: its field's is called with none, `(0, listener.listen)`
// (rd579, rd655).
use std::cell::Cell;
use std::rc::Rc;

#[cfg_attr(rust_js, rust_js::link_name = "Math.abs")]
fn abs(x: f64) -> f64 {
    x.abs()
}

struct Controller {
    error: Rc<dyn Fn(f64) -> f64>,
    close: Rc<dyn Fn(f64) -> f64>,
}

struct Listener {
    listen: Rc<dyn Fn(f64) -> f64>,
}

fn main() {
    let closed = Rc::new(Cell::new(false));
    let error: Rc<dyn Fn(f64) -> f64> = Rc::new({
        let closed = Rc::clone(&closed);
        move |reason| {
            if closed.get() {
                return 0.0;
            }
            closed.set(true);
            reason * 2.0
        }
    });
    let close = Rc::new({
        let error = Rc::clone(&error);
        move |reason| error(reason) + 1.0
    });
    let controller = Controller { error, close };
    println!("{}", (controller.close)(-1.5));
    println!("{}", (controller.error)(4.0));
    let listener = Listener {
        listen: Rc::new(abs),
    };
    println!("{}", (listener.listen)(-2.5));
}
