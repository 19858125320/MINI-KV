use criterion::{criterion_group, criterion_main, BatchSize, Criterion};
use kvs::{KvStore, KVEngine, SledStore};
use rand::prelude::*;
use tempfile::TempDir;

fn set_bench(c: &mut Criterion) {
    let mut group = c.benchmark_group("set_bench");
    group.bench_function("kvs", |b| {
        b.iter_batched(
            || {
                let temp_dir = TempDir::new().unwrap();
                (KvStore::open(temp_dir.path()).unwrap(), temp_dir)
            },
            |(mut store, _temp_dir)| {
                for i in 1..(1 << 12) {
                    store.set(format!("key{}", i), "value".to_string()).unwrap();
                }
            },
            BatchSize::SmallInput,
        )
    });
    group.bench_function("sled", |b| {
        b.iter_batched(
            || {
                let temp_dir = TempDir::new().unwrap();
                (SledStore::open(temp_dir.path()).unwrap(), temp_dir)
            },
            |(mut db, _temp_dir)| {
                for i in 1..(1 << 12) {
                    db.set(format!("key{}", i), "value".to_string()).unwrap();
                }
            },
            BatchSize::SmallInput,
        )
    });
    group.finish();
}

fn get_bench(c: &mut Criterion) {
    let mut group = c.benchmark_group("get_bench");
    for i in &vec![8, 12, 16, 20] {
        group.bench_with_input(format!("kvs_{}", i), i, |b, i| {
            let temp_dir = TempDir::new().unwrap();
            let mut store = KvStore::open(temp_dir.path()).unwrap();
            for key_i in 1..(1 << i) {
                store
                    .set(format!("key{}", key_i), "value".to_string())
                    .unwrap();
            }
            let mut rng = SmallRng::from_seed([0; 16]);
            b.iter(|| {
                store
                    .get(format!("key{}", rng.gen_range(1, 1 << i)))
                    .unwrap();
            })
        });
    }
    for i in &vec![8, 12, 16, 20] {
        group.bench_with_input(format!("sled_{}", i), i, |b, i| {
            let temp_dir = TempDir::new().unwrap();
            let mut db = SledStore::open(temp_dir.path()).unwrap();
            for key_i in 1..(1 << i) {
                db.set(format!("key{}", key_i), "value".to_string())
                    .unwrap();
            }
            let mut rng = SmallRng::from_seed([0; 16]);
            b.iter(|| {
                db.get(format!("key{}", rng.gen_range(1, 1 << i))).unwrap();
            })
        });
    }
    group.finish();
}

criterion_group!(benches, set_bench, get_bench);
criterion_main!(benches);