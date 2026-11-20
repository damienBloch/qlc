fn work() {}

fn main() {
    let t = thread(work@host);
    join(t);
}
