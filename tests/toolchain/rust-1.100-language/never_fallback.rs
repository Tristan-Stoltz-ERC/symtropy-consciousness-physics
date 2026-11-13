#![allow(unknown_lints)]
// Rust 1.99 still has the historical unit fallback in editions 2018/2021,
// but its compatibility lint warns/denies by default. Allow that lint only
// in this diagnostic fixture so the test observes the fallback's actual type.
// On Rust 1.100 the fallback is !, so the Default::default() expression
// remains a real type error even though the now-removed lint is allowed.
#![allow(dependency_on_unit_never_type_fallback)]

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
