fn work[@a, @b]@a() {}

fn main() {
    let t = thread(work@b);
    join(t);
}
