fn work[@a]@a() {
    let d = sim_device();
    print(7);
    delete(d);
}

fn main() {
    let t = thread(work@a);
    join(t);
}
