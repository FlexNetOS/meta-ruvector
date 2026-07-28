//! `rvf filter` -- Create a MEMBERSHIP_SEG with include/exclude filter.

use clap::Args;
use std::path::Path;

use rvf_runtime::{MembershipFilter, RvfStore};

use super::map_rvf_err;

#[derive(Args)]
pub struct FilterArgs {
    /// Path to the RVF store
    pub file: String,
    /// Comma-separated list of vector IDs to include
    #[arg(long, value_delimiter = ',')]
    pub include_ids: Option<Vec<u64>>,
    /// Comma-separated list of vector IDs to exclude
    #[arg(long, value_delimiter = ',')]
    pub exclude_ids: Option<Vec<u64>>,
    /// Output path (if different from input, creates a derived file)
    #[arg(short, long)]
    pub output: Option<String>,
    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}

pub fn run(args: FilterArgs) -> Result<(), Box<dyn std::error::Error>> {
    let (include_mode, ids) = match (&args.include_ids, &args.exclude_ids) {
        (Some(inc), None) => (true, inc.clone()),
        (None, Some(exc)) => (false, exc.clone()),
        (Some(_), Some(_)) => {
            return Err("Cannot specify both --include-ids and --exclude-ids".into());
        }
        (None, None) => {
            return Err("Must specify either --include-ids or --exclude-ids".into());
        }
    };

    let target_path = args.output.as_deref().unwrap_or(&args.file);

    // If output is different, create a real COW branch so the filtered child
    // retains queryable parent vectors after close and reopen.
    if target_path != args.file {
        let parent = RvfStore::open_readonly(Path::new(&args.file)).map_err(map_rvf_err)?;
        let child = parent.branch(Path::new(target_path)).map_err(map_rvf_err)?;
        child.close().map_err(map_rvf_err)?;
    }

    let mut store = RvfStore::open(Path::new(target_path)).map_err(map_rvf_err)?;
    let persisted_vector_count = store
        .membership_filter()
        .map(MembershipFilter::vector_count)
        .unwrap_or_else(|| {
            store
                .iter_vectors()
                .map(|(id, _)| id)
                .max()
                .map_or(0, |id| id.saturating_add(1))
        });
    let vector_count = ids
        .iter()
        .copied()
        .max()
        .map_or(persisted_vector_count, |id| {
            persisted_vector_count.max(id.saturating_add(1))
        });
    let mut filter = if include_mode {
        MembershipFilter::new_include(vector_count)
    } else {
        MembershipFilter::new_exclude(vector_count)
    };
    for &id in &ids {
        filter.add(id);
    }
    store
        .append_membership_filter(filter)
        .map_err(map_rvf_err)?;
    store.close().map_err(map_rvf_err)?;

    let mode_str = if include_mode { "include" } else { "exclude" };
    if args.json {
        crate::output::print_json(&serde_json::json!({
            "status": "filtered",
            "mode": mode_str,
            "ids_count": ids.len(),
            "target": target_path,
        }));
    } else {
        println!("Membership filter created:");
        crate::output::print_kv("Mode:", mode_str);
        crate::output::print_kv("IDs:", &ids.len().to_string());
        crate::output::print_kv("Target:", target_path);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rvf_runtime::options::DistanceMetric;
    use rvf_runtime::{QueryOptions, RvfOptions};
    use tempfile::TempDir;

    #[test]
    fn output_reopens_as_queryable_filtered_cow_branch() {
        let dir = TempDir::new().unwrap();
        let parent_path = dir.path().join("parent.rvf");
        let child_path = dir.path().join("filtered.rvf");
        let mut parent = RvfStore::create(
            &parent_path,
            RvfOptions {
                dimension: 2,
                metric: DistanceMetric::L2,
                ..Default::default()
            },
        )
        .unwrap();
        parent
            .ingest_batch(&[&[0.0f32, 0.0], &[10.0f32, 10.0]], &[0, 1], None)
            .unwrap();
        parent.close().unwrap();

        run(FilterArgs {
            file: parent_path.to_string_lossy().into_owned(),
            include_ids: Some(vec![1]),
            exclude_ids: None,
            output: Some(child_path.to_string_lossy().into_owned()),
            json: false,
        })
        .unwrap();

        let child = RvfStore::open(&child_path).unwrap();
        assert!(child.is_cow_child());
        assert_eq!(child.parent_path(), Some(parent_path.as_path()));
        let results = child
            .query(
                &[0.0, 0.0],
                2,
                &QueryOptions {
                    force_exact: true,
                    ..Default::default()
                },
            )
            .unwrap();
        assert_eq!(
            results.iter().map(|result| result.id).collect::<Vec<_>>(),
            vec![1]
        );
        child.close().unwrap();
    }
}
