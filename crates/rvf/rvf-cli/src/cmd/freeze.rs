//! `rvf freeze` -- Snapshot-freeze the current state of an RVF store.

use clap::Args;
use std::path::Path;

use rvf_runtime::RvfStore;
use rvf_types::{RefcountHeader, SegmentType, REFCOUNT_MAGIC};

use super::map_rvf_err;

#[derive(Args)]
pub struct FreezeArgs {
    /// Path to the RVF store
    pub file: String,
    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}

pub fn run(args: FreezeArgs) -> Result<(), Box<dyn std::error::Error>> {
    let mut store = RvfStore::open(Path::new(&args.file)).map_err(map_rvf_err)?;
    let status = store.status();
    let snapshot_epoch = status.current_epoch + 1;

    let header = RefcountHeader {
        magic: REFCOUNT_MAGIC,
        version: 1,
        refcount_width: 1,
        _pad: 0,
        cluster_count: 0,
        max_refcount: 0,
        array_offset: 32,
        snapshot_epoch,
        _reserved: 0,
    };
    store
        .append_segment(SegmentType::Refcount, &header.to_bytes())
        .map_err(map_rvf_err)?;

    // Emit a witness event for the snapshot
    // (witness writing would go through the store's witness path when available)

    store.close().map_err(map_rvf_err)?;

    if args.json {
        crate::output::print_json(&serde_json::json!({
            "status": "frozen",
            "snapshot_epoch": snapshot_epoch,
        }));
    } else {
        println!("Store frozen:");
        crate::output::print_kv("Snapshot epoch:", &snapshot_epoch.to_string());
        println!("  All further writes will create a new derived generation.");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rvf_runtime::RvfOptions;
    use rvf_types::SEGMENT_HEADER_SIZE;
    use tempfile::TempDir;

    #[test]
    fn freeze_appends_manifested_canonical_refcount_frame() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("freeze.rvf");
        RvfStore::create(
            &path,
            RvfOptions {
                dimension: 2,
                ..Default::default()
            },
        )
        .unwrap()
        .close()
        .unwrap();

        run(FreezeArgs {
            file: path.to_string_lossy().into_owned(),
            json: true,
        })
        .unwrap();

        let reopened = RvfStore::open_readonly(&path).unwrap();
        let &(_, offset, payload_len, _) = reopened
            .segment_dir()
            .iter()
            .rev()
            .find(|entry| entry.3 == SegmentType::Refcount as u8)
            .expect("freeze must manifest-link a refcount segment");
        assert_eq!(payload_len, 32);

        let bytes = std::fs::read(&path).unwrap();
        let (segment_header, payload) = rvf_wire::read_segment(&bytes[offset as usize..]).unwrap();
        rvf_wire::validate_segment(&segment_header, payload).unwrap();
        assert_eq!(segment_header.checksum_algo, 2);
        assert_eq!(
            (SEGMENT_HEADER_SIZE + payload.len() + segment_header.alignment_pad as usize) % 64,
            0
        );

        let header =
            RefcountHeader::from_bytes(payload.try_into().expect("32-byte refcount header"))
                .unwrap();
        assert_eq!(header.snapshot_epoch, 1);
        reopened.close().unwrap();
    }
}
