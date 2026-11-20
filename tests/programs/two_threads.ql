fn work[@a, @b]@(a, b)() {
    print(1);
}

fn main() {
    let first = thread(work@a);
    let second = thread(work@b);
    join(first);
    join(second);
}
