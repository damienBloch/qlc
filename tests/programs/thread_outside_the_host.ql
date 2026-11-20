fn work[@a]@a() {}

fn spawner[@a]@a() {
    let t = thread(work@a);
    join(t);
}

fn main() {}
