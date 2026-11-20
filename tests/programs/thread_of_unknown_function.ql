fn main() {
    let t = thread(work@a);
    join(t);
}
