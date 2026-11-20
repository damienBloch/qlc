fn work[@a]@a() {}

fn main() {
    let t = thread(work@a);
    join(t);
    join(t);
}
