//! `rvf rebuild-refcounts` -- Recompute REFCOUNT_SEG from COW map chain.

use clap::Args;
use std::path::Path;

use rvf_runtime::RvfStore;
use rvf_types::{RefcountHeader, SegmentType, REFCOUNT_MAGIC};

use super::map_rvf_err;

#[derive(Args)]
pub struct RebuildRefcountsArgs {
    /// Path to the RVF store
    pub file: String,
    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}

pub fn run(args: RebuildRefcountsArgs) -> Result<(), Box<dyn std::error::Error>> {
    let mut store = RvfStore::open(Path::new(&args.file)).map_err(map_rvf_err)?;
    let Some(stats) = store.cow_stats() else {
        if args.json {
            crate::output::print_json(&serde_json::json!({
                "status": "no_cow_map",
                "message": "No COW map found; nothing to rebuild",
            }));
        } else {
            println!("No COW map found in file. Nothing to rebuild.");
        }
        return Ok(());
    };
    let cluster_count = stats.cluster_count;
    let local_cluster_count = stats.local_cluster_count;

    // Build refcount array: 1 byte per cluster, all set to 1 (base reference)
    let refcount_array = vec![1u8; cluster_count as usize];

    let header = RefcountHeader {
        magic: REFCOUNT_MAGIC,
        version: 1,
        refcount_width: 1,
        _pad: 0,
        cluster_count,
        max_refcount: 1,
        array_offset: 32,
        snapshot_epoch: 0,
        _reserved: 0,
    };
    let payload = [header.to_bytes().as_slice(), refcount_array.as_slice()].concat();
    store
        .append_segment(SegmentType::Refcount, &payload)
        .map_err(map_rvf_err)?;
    store.close().map_err(map_rvf_err)?;

    if args.json {
        crate::output::print_json(&serde_json::json!({
            "status": "rebuilt",
            "cluster_count": cluster_count,
            "local_clusters": local_cluster_count,
        }));
    } else {
        println!("Refcounts rebuilt:");
        crate::output::print_kv("Cluster count:", &cluster_count.to_string());
        crate::output::print_kv("Local clusters:", &local_cluster_count.to_string());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rvf_runtime::RvfOptions;
    use tempfile::TempDir;

    #[test]
    fn cli_reader_uses_manifested_v2_cow_map_and_manifests_refcount() {
        let dir = TempDir::new().unwrap();
        let parent_path = dir.path().join("parent.rvf");
        let child_path = dir.path().join("child.rvf");
        let mut parent = RvfStore::create(
            &parent_path,
            RvfOptions {
                dimension: 2,
                ..Default::default()
            },
        )
        .unwrap();
        parent.ingest_batch(&[&[1.0, 2.0]], &[0], None).unwrap();
        let child = parent.branch(&child_path).unwrap();
        child.close().unwrap();
        parent.close().unwrap();

        run(RebuildRefcountsArgs {
            file: child_path.to_string_lossy().into_owned(),
            json: true,
        })
        .unwrap();

        let reopened = RvfStore::open_readonly(&child_path).unwrap();
        assert!(reopened
            .segment_dir()
            .iter()
            .any(|entry| entry.3 == SegmentType::Refcount as u8));
        reopened.close().unwrap();
    }
}
