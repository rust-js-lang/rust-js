// What a `Box` holds, moved out of it on one way, `take(*n)`, or in part,
// `Pair { a, ..*p }`: what's left of it is dropped where the box is, as
// its own drop flag says, as rustc's `open_drop_for_box_contents` drops it
// (ADR 0364, #967).
struct Noisy(&'static str);

impl Drop for Noisy {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

struct Pair {
    a: Noisy,
    b: Noisy,
}

fn take(n: Noisy) {
    println!("took {}", n.0);
}

fn main() {
    for i in 0..2 {
        let n = Box::new(Noisy(if i == 0 { "given" } else { "kept" }));
        if i == 0 {
            take(*n);
        }
        println!("end of {i}");
    }
    {
        let p = Box::new(Pair {
            a: Noisy("a"),
            b: Noisy("b"),
        });
        let q = Pair {
            a: Noisy("c"),
            ..*p
        };
        println!("made {} {}", q.a.0, q.b.0);
    }
    {
        let p = Box::new(Pair {
            a: Noisy("x"),
            b: Noisy("y"),
        });
        let Pair { a, .. } = *p;
        println!("took {}", a.0);
    }
    println!("end");
}
