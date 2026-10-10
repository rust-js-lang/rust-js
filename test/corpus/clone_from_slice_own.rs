// `clone_from_slice` clones each item by its `clone_from`, a type's own
// where it has one, as std's does.

struct Counted(u32);

impl Clone for Counted {
    fn clone(&self) -> Self {
        println!("clone {}", self.0);
        Counted(self.0)
    }

    fn clone_from(&mut self, source: &Self) {
        println!("clone_from {} into {}", source.0, self.0);
        self.0 = source.0;
    }
}

#[derive(Debug, PartialEq)]
enum Light {
    On,
    Off,
}

impl Clone for Light {
    fn clone(&self) -> Self {
        println!("clone light");
        if *self == Light::On { Light::On } else { Light::Off }
    }

    fn clone_from(&mut self, source: &Self) {
        println!("clone_from {source:?} into {self:?}");
        *self = if *source == Light::On { Light::On } else { Light::Off };
    }
}

fn main() {
    let source = [Counted(1), Counted(2)];
    let mut target = [Counted(7), Counted(8)];
    target.clone_from_slice(&source);
    println!("{} {}", target[0].0, target[1].0);
    let mut lights = [Light::Off, Light::On];
    lights.clone_from_slice(&[Light::On, Light::Off]);
    println!("{lights:?}");
}
