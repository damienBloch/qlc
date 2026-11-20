fn work[@a]@a() {
    print(1);
}

fn main() {
    let t = thread(work@a);
    print(2);
    join(t);
}
