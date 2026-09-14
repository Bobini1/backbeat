use std::fs;
use std::path::{Path, PathBuf};

use criterion::{Criterion, Throughput, black_box, criterion_group, criterion_main};
use rg_formats::{bms, bmson, ksh, kson, sm, sm_msd, ssc};

fn manifest_dir() -> PathBuf {
	Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf()
}

fn read_fixture(rel: impl AsRef<Path>) -> (PathBuf, Vec<u8>) {
	let path = manifest_dir().join(rel.as_ref());
	let bytes = fs::read(&path).expect("failed to read fixture");
	(path, bytes)
}

fn bench_parse_bytes(c: &mut Criterion, name: &str, bytes: &[u8], f: impl Fn(&[u8]) + Clone) {
	let mut group = c.benchmark_group("parse");
	group.throughput(Throughput::Bytes(bytes.len() as u64));
	group.bench_function(name, |b| {
		let f = f.clone();
		b.iter(|| f(black_box(bytes)));
	});
	group.finish();
}

fn bench_bms(c: &mut Criterion) {
	let (_, bytes) = read_fixture("test_files/bms/_AK_minec24.bms");
	bench_parse_bytes(c, "bms", &bytes, |bytes| {
		bms::from_bytes(bytes, "bms", bms::BmsRandomStrategy::AlwaysFirstBranch).unwrap();
	});
}

fn bench_bme(c: &mut Criterion) {
	let (_, bytes) = read_fixture("test_files/bms/gengaozogod24s.bme");
	bench_parse_bytes(c, "bme", &bytes, |bytes| {
		bms::from_bytes(bytes, "bme", bms::BmsRandomStrategy::AlwaysFirstBranch).unwrap();
	});
}

fn bench_bml(c: &mut Criterion) {
	let (_, bytes) = read_fixture("test_files/bms/_AK_minec24.bms");
	bench_parse_bytes(c, "bml", &bytes, |bytes| {
		bms::from_bytes(bytes, "bml", bms::BmsRandomStrategy::AlwaysFirstBranch).unwrap();
	});
}

fn bench_pms(c: &mut Criterion) {
	let (_, bytes) = read_fixture("test_files/bms/00_arcology-5btn.pms");
	bench_parse_bytes(c, "pms", &bytes, |bytes| {
		bms::from_bytes(bytes, "pms", bms::BmsRandomStrategy::AlwaysFirstBranch).unwrap();
	});
}

fn bench_bmson(c: &mut Criterion) {
	let (_, bytes) = read_fixture("test_files/bmson/minimal.bmson");
	bench_parse_bytes(c, "bmson", &bytes, |bytes| {
		bmson::from_bytes(bytes).unwrap();
	});
}

fn bench_ksh(c: &mut Criterion) {
	let (_, bytes) = read_fixture("test_files/usc/mxm.ksh");
	bench_parse_bytes(c, "ksh", &bytes, |bytes| {
		ksh::from_bytes(bytes).unwrap();
	});
}

fn bench_kson(c: &mut Criterion) {
	let (_, bytes) = read_fixture("test_files/usc/mxm.kson");
	bench_parse_bytes(c, "kson", &bytes, |bytes| {
		kson::from_bytes(bytes).unwrap();
	});
}

fn bench_sm_small(c: &mut Criterion) {
	let (path, bytes) = read_fixture("test_files/sm/zk_test.sm");
	bench_parse_bytes(c, "sm_small", &bytes, |bytes| {
		sm::from_bytes(bytes, &path).unwrap();
	});
}

fn bench_sm_large(c: &mut Criterion) {
	let (path, bytes) = read_fixture("test_files/sm/xsfull.sm");
	bench_parse_bytes(c, "sm_large", &bytes, |bytes| {
		sm::from_bytes(bytes, &path).unwrap();
	});
}

fn bench_sm_msd(c: &mut Criterion) {
	let (_, bytes) = read_fixture("test_files/sm/zk_test.sm");
	bench_parse_bytes(c, "sm_msd", &bytes, |bytes| {
		sm_msd::from_bytes(bytes);
	});
}

fn bench_ssc(c: &mut Criterion) {
	let (path, bytes) = read_fixture("test_files/ssc/minimal.ssc");
	bench_parse_bytes(c, "ssc", &bytes, |bytes| {
		ssc::from_bytes(bytes, &path);
	});
}

criterion_group!(
	benches,
	bench_bms,
	bench_bme,
	bench_bml,
	bench_pms,
	bench_bmson,
	bench_ksh,
	bench_kson,
	bench_sm_small,
	bench_sm_large,
	bench_sm_msd,
	bench_ssc,
);
criterion_main!(benches);
