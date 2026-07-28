//! Times the source scan the way the app calls it. Run against any root:
//!   cargo run --manifest-path core/Cargo.toml --example scanbench -- <root> [limit]
//!   cargo run --release --manifest-path core/Cargo.toml --example scanbench -- <root> [limit]
use std::path::PathBuf;
use std::time::Instant;

fn main() {
    let mut args = std::env::args().skip(1);
    let root = PathBuf::from(args.next().expect("usage: scanbench <root> [limit]"));
    let limit: usize = args.next().and_then(|s| s.parse().ok()).unwrap_or(10_000);
    for pass in 1..=3 {
        let start = Instant::now();
        let list = mcb_core::source::list_source_files(&root, limit, None).expect("scan failed");
        println!(
            "pass {pass}: {} files in {:?}",
            list.files.len(),
            start.elapsed()
        );
    }
}
