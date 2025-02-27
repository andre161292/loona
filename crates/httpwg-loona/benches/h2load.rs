use codspeed_criterion_compat::{criterion_group, criterion_main, Criterion};
use httpwg_loona::{Mode, Proto};

pub fn h2load(c: &mut Criterion) {
    c.bench_function("h2load", |b| {
        b.iter(|| {
            httpwg_loona::do_main(0, Proto::H2, Mode::H2Load);
        })
    });
}

criterion_group!(benches, h2load);
criterion_main!(benches);
