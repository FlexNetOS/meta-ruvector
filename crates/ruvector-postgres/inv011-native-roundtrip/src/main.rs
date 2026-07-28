use std::collections::{BTreeSet, HashMap};
use std::env;
use std::fs::{self, File};
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

use rvf_runtime::{CowMap, MembershipFilter, QueryOptions, RvfOptions, RvfStore};
use rvf_types::{CowMapEntry, DerivationType, SegmentType, COW_MAP_V2, SEGMENT_HEADER_SIZE};
use rvf_wire::DecodedCowMapHeader;
use serde::Deserialize;
use serde_json::json;

const INPUT_SCHEMA: &str = "lifeos.native-rvf-postgres-input.v1";
const DIMENSION: u16 = 2;
const CLUSTER_SIZE: u32 = 4096;
const VECTORS_PER_CLUSTER: u32 = CLUSTER_SIZE / (DIMENSION as u32 * 4);

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Input {
    schema: String,
    parent_generation_id: u64,
    parent_rows: Vec<MembershipRow>,
    generation_id: u64,
    rows: Vec<MembershipRow>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct MembershipRow {
    relation_name: String,
    logical_key_digest: String,
    vector_id: u64,
    operation: String,
    tombstone: bool,
}

fn fail(message: impl AsRef<str>) -> ! {
    eprintln!("INV-011 native RVF roundtrip failed: {}", message.as_ref());
    std::process::exit(1);
}

fn validate_hex_digest(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn validate_generation(value: u64, label: &str) -> Result<u32, String> {
    let generation_id = u32::try_from(value)
        .map_err(|_| format!("SQL {label} generation {value} exceeds u32::MAX"))?;
    if generation_id == 0 {
        return Err(format!("SQL {label} generation must be non-zero"));
    }
    Ok(generation_id)
}

fn validate_rows(rows: &[MembershipRow], label: &str) -> Result<(), String> {
    let mut identities: HashMap<u64, (&str, &str)> = HashMap::new();
    for row in rows {
        if row.relation_name.trim().is_empty() || !validate_hex_digest(&row.logical_key_digest) {
            return Err(format!(
                "{label} membership row has an invalid relation or key digest"
            ));
        }
        if !matches!(row.operation.as_str(), "insert" | "update" | "delete")
            || row.tombstone != (row.operation == "delete")
        {
            return Err(format!(
                "operation/tombstone mismatch for vector {}",
                row.vector_id
            ));
        }
        match identities.insert(row.vector_id, (&row.relation_name, &row.logical_key_digest)) {
            Some(previous) if previous != (&row.relation_name, &row.logical_key_digest) => {
                return Err(format!(
                    "vector {} maps to more than one durable identity",
                    row.vector_id
                ));
            }
            Some(_) => {
                return Err(format!(
                    "duplicate resolved membership row for vector {}",
                    row.vector_id
                ));
            }
            None => {}
        }
    }
    Ok(())
}

fn validate(input: &Input) -> Result<(u32, u32), String> {
    if input.schema != INPUT_SCHEMA {
        return Err(format!("unsupported input schema: {}", input.schema));
    }
    let parent_generation_id = validate_generation(input.parent_generation_id, "parent")?;
    let generation_id = validate_generation(input.generation_id, "child")?;
    if generation_id <= parent_generation_id {
        return Err("child SQL generation must be newer than its parent generation".to_string());
    }
    validate_rows(&input.parent_rows, "parent")?;
    validate_rows(&input.rows, "child")?;
    let parent_identities: HashMap<u64, (&str, &str)> = input
        .parent_rows
        .iter()
        .map(|row| {
            (
                row.vector_id,
                (row.relation_name.as_str(), row.logical_key_digest.as_str()),
            )
        })
        .collect();
    for row in &input.rows {
        if parent_identities.get(&row.vector_id)
            != Some(&(row.relation_name.as_str(), row.logical_key_digest.as_str()))
        {
            return Err(format!(
                "child vector {} is not the same durable parent identity",
                row.vector_id
            ));
        }
    }
    Ok((parent_generation_id, generation_id))
}

fn read_payloads(
    path: &Path,
    directory: &[(u64, u64, u64, u8)],
) -> Result<Vec<(u8, Vec<u8>)>, String> {
    let mut file = File::open(path).map_err(|error| error.to_string())?;
    let mut payloads = Vec::new();
    for &(_, offset, _, segment_type) in directory {
        if segment_type != SegmentType::CowMap as u8
            && segment_type != SegmentType::Membership as u8
        {
            continue;
        }
        file.seek(SeekFrom::Start(offset))
            .map_err(|error| error.to_string())?;
        let mut header = [0u8; SEGMENT_HEADER_SIZE];
        file.read_exact(&mut header)
            .map_err(|error| error.to_string())?;
        let payload_len = usize::try_from(u64::from_le_bytes(
            header[0x10..0x18]
                .try_into()
                .map_err(|_| "invalid segment header payload length")?,
        ))
        .map_err(|_| "segment payload does not fit in memory")?;
        let mut payload = vec![0u8; payload_len];
        file.read_exact(&mut payload)
            .map_err(|error| error.to_string())?;
        payloads.push((segment_type, payload));
    }
    Ok(payloads)
}

fn run(input_path: &Path, output_dir: &Path) -> Result<serde_json::Value, String> {
    let input: Input = serde_json::from_slice(
        &fs::read(input_path).map_err(|error| format!("read input: {error}"))?,
    )
    .map_err(|error| format!("parse input: {error}"))?;
    let (parent_generation_id, generation_id) = validate(&input)?;
    fs::create_dir(output_dir).map_err(|error| format!("create output directory: {error}"))?;

    let parent_path = output_dir.join("agentdb-parent.rvf");
    let child_path = output_dir.join("agentdb-child.rvf");
    let cow_payload_path = output_dir.join("cow-map.payload");
    let parent_membership_payload_path = output_dir.join("parent-membership.payload");
    let membership_payload_path = output_dir.join("membership.payload");

    let max_vector_id = input.parent_rows.iter().map(|row| row.vector_id).max();
    let vector_count = max_vector_id
        .map(|value| value.checked_add(1).ok_or("vector_id capacity overflow"))
        .transpose()?
        .unwrap_or(0);
    let cluster_count = u32::try_from(vector_count.div_ceil(VECTORS_PER_CLUSTER as u64))
        .map_err(|_| "RVF cluster count exceeds u32::MAX")?;

    let options = RvfOptions {
        dimension: DIMENSION,
        ..Default::default()
    };
    let mut parent =
        RvfStore::create(&parent_path, options.clone()).map_err(|error| error.to_string())?;
    let vectors: Vec<[f32; 2]> = input
        .parent_rows
        .iter()
        .map(|row| {
            [
                (row.vector_id & 0xffff) as f32,
                ((row.vector_id >> 16) & 0xffff) as f32,
            ]
        })
        .collect();
    let vector_refs: Vec<&[f32]> = vectors.iter().map(|vector| vector.as_slice()).collect();
    let vector_ids: Vec<u64> = input.parent_rows.iter().map(|row| row.vector_id).collect();
    if !vector_ids.is_empty() {
        let ingested = parent
            .ingest_batch(&vector_refs, &vector_ids, None)
            .map_err(|error| error.to_string())?;
        if ingested.accepted != vector_ids.len() as u64 || ingested.rejected != 0 {
            return Err("parent RVF rejected a canonical membership vector".to_string());
        }
    }
    let mut parent_membership = MembershipFilter::new_include(vector_count);
    for row in &input.parent_rows {
        if !row.tombstone {
            parent_membership.add(row.vector_id);
        }
    }
    parent
        .append_membership_filter_at_generation(parent_membership, parent_generation_id)
        .map_err(|error| error.to_string())?;

    let mut child = parent
        .derive(&child_path, DerivationType::Clone, Some(options))
        .map_err(|error| error.to_string())?;
    child
        .append_cow_map_at_generation(
            CowMap::new_parent_ref(cluster_count),
            CLUSTER_SIZE,
            VECTORS_PER_CLUSTER,
            *parent.file_id(),
            child.file_identity().parent_hash,
            generation_id,
        )
        .map_err(|error| error.to_string())?;
    let mut membership = MembershipFilter::new_include(vector_count);
    for row in &input.rows {
        if !row.tombstone {
            membership.add(row.vector_id);
        }
    }
    child
        .append_membership_filter_at_generation(membership, generation_id)
        .map_err(|error| error.to_string())?;
    child.close().map_err(|error| error.to_string())?;
    parent.close().map_err(|error| error.to_string())?;

    let reopened = RvfStore::open(&child_path).map_err(|error| error.to_string())?;
    if !reopened.is_cow_child() || reopened.lineage_depth() != 1 {
        return Err("reopened child lost its RVF parent lineage".to_string());
    }
    let reopened_membership = reopened
        .membership_filter()
        .ok_or("reopened child has no MEMBERSHIP segment")?;
    if reopened_membership.generation_id() != generation_id
        || reopened_membership.vector_count() != vector_count
    {
        return Err("reopened MEMBERSHIP generation/capacity mismatch".to_string());
    }
    let reopened_map = reopened
        .cow_map()
        .ok_or("reopened child has no COW_MAP segment")?;
    if reopened_map.cluster_count() != cluster_count
        || reopened_map
            .entries()
            .iter()
            .any(|entry| *entry != CowMapEntry::ParentRef)
    {
        return Err("reopened COW_MAP did not reconstruct parent references".to_string());
    }

    let membership_rows: Vec<serde_json::Value> = input
        .rows
        .iter()
        .map(|row| {
            let visible = reopened_membership.contains(row.vector_id);
            json!({
                "logical_key_digest": row.logical_key_digest,
                "operation": row.operation,
                "relation_name": row.relation_name,
                "tombstone": row.tombstone,
                "vector_id": row.vector_id,
                "visible": visible,
            })
        })
        .collect();
    if input
        .rows
        .iter()
        .any(|row| reopened_membership.contains(row.vector_id) == row.tombstone)
    {
        return Err("reopened RVF membership differs from SQL tombstone semantics".to_string());
    }

    let expected_visible: BTreeSet<u64> = input
        .rows
        .iter()
        .filter(|row| !row.tombstone)
        .map(|row| row.vector_id)
        .collect();
    let queried_visible: BTreeSet<u64> = if expected_visible.is_empty() {
        BTreeSet::new()
    } else {
        reopened
            .query(
                &[0.0, 0.0],
                expected_visible.len(),
                &QueryOptions {
                    force_exact: true,
                    ..Default::default()
                },
            )
            .map_err(|error| error.to_string())?
            .into_iter()
            .map(|result| result.id)
            .collect()
    };
    if queried_visible != expected_visible {
        return Err(
            "reopened child query did not reconstruct exact visible membership".to_string(),
        );
    }

    let directory = reopened.segment_dir().to_vec();
    let payloads = read_payloads(&child_path, &directory)?;
    let cow_payloads: Vec<&Vec<u8>> = payloads
        .iter()
        .filter(|(kind, _)| *kind == SegmentType::CowMap as u8)
        .map(|(_, payload)| payload)
        .collect();
    let membership_payloads: Vec<&Vec<u8>> = payloads
        .iter()
        .filter(|(kind, _)| *kind == SegmentType::Membership as u8)
        .map(|(_, payload)| payload)
        .collect();
    if cow_payloads.len() != 1 || membership_payloads.len() != 1 {
        return Err("expected exactly one canonical COW_MAP and MEMBERSHIP segment".to_string());
    }
    let decoded_cow =
        rvf_wire::decode_cow_map(cow_payloads[0]).map_err(|error| error.to_string())?;
    let cow_generation = match decoded_cow.header {
        DecodedCowMapHeader::V2(header) if header.version == COW_MAP_V2 => header.generation_id,
        _ => return Err("COW_MAP is not canonical V2".to_string()),
    };
    let decoded_membership =
        rvf_wire::decode_membership(membership_payloads[0]).map_err(|error| error.to_string())?;
    if cow_generation != generation_id || decoded_membership.header.generation_id != generation_id {
        return Err("SQL generation is not bound to both native segment headers".to_string());
    }
    fs::write(&cow_payload_path, cow_payloads[0])
        .map_err(|error| format!("write COW_MAP payload: {error}"))?;
    fs::write(&membership_payload_path, membership_payloads[0])
        .map_err(|error| format!("write MEMBERSHIP payload: {error}"))?;

    let child_file_id = reopened
        .file_id()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    reopened.close().map_err(|error| error.to_string())?;
    let reopened_parent = RvfStore::open(&parent_path).map_err(|error| error.to_string())?;
    let parent_filter = reopened_parent
        .membership_filter()
        .ok_or("reopened parent has no MEMBERSHIP segment")?;
    if parent_filter.generation_id() != parent_generation_id
        || input
            .parent_rows
            .iter()
            .any(|row| parent_filter.contains(row.vector_id) == row.tombstone)
    {
        return Err("reopened parent membership differs from SQL state".to_string());
    }
    let parent_directory = reopened_parent.segment_dir().to_vec();
    let parent_payloads = read_payloads(&parent_path, &parent_directory)?;
    let parent_membership_payloads: Vec<&Vec<u8>> = parent_payloads
        .iter()
        .filter(|(kind, _)| *kind == SegmentType::Membership as u8)
        .map(|(_, payload)| payload)
        .collect();
    if parent_membership_payloads.len() != 1 {
        return Err("expected one canonical parent MEMBERSHIP segment".to_string());
    }
    let decoded_parent_membership = rvf_wire::decode_membership(parent_membership_payloads[0])
        .map_err(|error| error.to_string())?;
    if decoded_parent_membership.header.generation_id != parent_generation_id {
        return Err("parent SQL generation is not bound to MEMBERSHIP".to_string());
    }
    fs::write(
        &parent_membership_payload_path,
        parent_membership_payloads[0],
    )
    .map_err(|error| format!("write parent MEMBERSHIP payload: {error}"))?;
    reopened_parent.close().map_err(|error| error.to_string())?;

    Ok(json!({
        "schema": "lifeos.native-rvf-postgres-output.v1",
        "status": "passed",
        "generation_id": generation_id,
        "parent_generation_id": parent_generation_id,
        "child_file_id": child_file_id,
        "lineage_depth": 1,
        "row_count": input.rows.len(),
        "visible_count": expected_visible.len(),
        "cow_map": {
            "generation_id": cow_generation,
            "segment_count": cow_payloads.len(),
            "payload_path": cow_payload_path,
        },
        "membership": {
            "generation_id": decoded_membership.header.generation_id,
            "member_count": decoded_membership.header.member_count,
            "segment_count": membership_payloads.len(),
            "payload_path": membership_payload_path,
            "rows": membership_rows,
        },
        "parent_membership": {
            "generation_id": decoded_parent_membership.header.generation_id,
            "member_count": decoded_parent_membership.header.member_count,
            "payload_path": parent_membership_payload_path,
        },
        "parent_path": parent_path,
        "child_path": child_path,
    }))
}

fn main() {
    let arguments: Vec<String> = env::args().collect();
    if arguments.len() != 3 {
        fail("usage: ruvector-postgres-inv011-roundtrip INPUT_JSON OUTPUT_DIRECTORY");
    }
    let input_path = PathBuf::from(&arguments[1]);
    let output_dir = PathBuf::from(&arguments[2]);
    match run(&input_path, &output_dir) {
        Ok(report) => println!("{}", serde_json::to_string(&report).unwrap()),
        Err(error) => fail(error),
    }
}
