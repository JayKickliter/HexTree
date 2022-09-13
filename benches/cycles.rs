use iai::{black_box, main};
fn iai_benchmark_short() {}
mod iai_wrappers {
    pub fn iai_benchmark_short() {
        let _ = ::iai::black_box(super::iai_benchmark_short());
    }
}
fn main() {
    let benchmarks: &[&(&'static str, fn())] =
        &[&("iai_benchmark_short", iai_wrappers::iai_benchmark_short)];
    ::iai::runner(benchmarks);
}
