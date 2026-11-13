// Deliberate compatibility fixture: this code relies on the historical
// never-to-any fallback being () in editions 2015/2018/2021.
// Rust 1.100 changes that fallback to ! on every edition.
fn fallback_probe() {
    if true {
        return;
    } else {
        Default::default()
    };
}

fn main() {
    fallback_probe();
}
