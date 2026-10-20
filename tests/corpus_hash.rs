//! The vendored corpus is the one its manifest and `conformance/SOURCE.json`
//! describe: `corpus_hash` recomputed by the reference's rule (the corpus
//! README's "The format"), the string `sha256:` followed by the lowercase hex
//! SHA-256 of the case files' bytes concatenated in the order of the
//! manifest's `files`, equals the hash both record.
//!
//! A mismatch means the vendored copy is not the one recorded; it is never
//! repaired by rewriting a hash.

use std::fmt::Write as _;
use std::fs;

use serde_json::Value;
use sha2::{Digest, Sha256};

/// The vendored corpus directory.
const CORPUS: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/conformance");

fn json(file: &str) -> Value {
    let text = fs::read_to_string(format!("{CORPUS}/{file}")).unwrap();
    serde_json::from_str(&text).unwrap_or_else(|error| panic!("{file}: {error}"))
}

/// `sha256:` and the lowercase hex SHA-256 of the files the manifest lists,
/// their bytes concatenated in its order.
fn corpus_hash(manifest: &Value) -> String {
    let mut hasher = Sha256::new();
    for entry in manifest["files"].as_array().unwrap() {
        let file = entry["file"].as_str().unwrap();
        hasher.update(fs::read(format!("{CORPUS}/{file}")).unwrap());
    }
    hasher
        .finalize()
        .iter()
        .fold(String::from("sha256:"), |mut hex, byte| {
            write!(hex, "{byte:02x}").unwrap();
            hex
        })
}

// Sabotage: hashing the manifest's files in reverse order turns this red.
#[test]
fn the_case_files_hash_to_the_manifest_s_corpus_hash() {
    let manifest = json("manifest.json");
    let recorded = manifest["corpus_hash"].as_str().unwrap();
    assert_eq!(corpus_hash(&manifest), recorded);
}

// Sabotage: one changed byte in a vendored case file turns this red.
#[test]
fn source_json_records_the_same_hash_and_the_tag_it_came_from() {
    let source = json("SOURCE.json");
    let keys: Vec<&str> = source
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect();
    assert_eq!(keys, ["corpus_hash", "repo", "sha", "tag"]);
    assert_eq!(source["repo"], "riddler/uxid-ex");
    let tag = source["tag"].as_str().unwrap();
    assert!(tag.starts_with('v'), "{tag}");
    let sha = source["sha"].as_str().unwrap();
    assert_eq!(sha.len(), 40, "{sha}");
    assert!(
        sha.bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase()),
        "{sha}"
    );

    let manifest = json("manifest.json");
    assert_eq!(source["corpus_hash"], manifest["corpus_hash"]);
    assert_eq!(
        source["corpus_hash"].as_str().unwrap(),
        corpus_hash(&manifest)
    );
}
