//! The conversion, end to end through the program, and the check that comes first.

mod support;

use arrow_array::cast::AsArray;
use arrow_array::types::{Float64Type, UInt64Type};
use arrow_array::{Array, RecordBatch};
use genesis_seal::dataset::{Cause, check_dataset};
use genesis_seal_parquet::{Failure, check};
use serde_json::{Value, json};
use support::*;

fn column<'a>(batches: &'a [RecordBatch], name: &str) -> Vec<&'a dyn Array> {
    batches.iter().map(|b| b.column_by_name(name).unwrap_or_else(|| panic!("no column {name}")).as_ref()).collect()
}

fn floats(batches: &[RecordBatch], name: &str) -> Vec<Option<f64>> {
    let mut out = Vec::new();
    for array in column(batches, name) {
        let array = array.as_primitive::<Float64Type>();
        out.extend((0..array.len()).map(|i| array.is_valid(i).then(|| array.value(i))));
    }
    out
}

fn strings(batches: &[RecordBatch], name: &str) -> Vec<Option<String>> {
    let mut out = Vec::new();
    for array in column(batches, name) {
        let array = array.as_string::<i32>();
        out.extend((0..array.len()).map(|i| array.is_valid(i).then(|| array.value(i).to_string())));
    }
    out
}

fn whole(batches: &[RecordBatch], name: &str) -> Vec<Option<u64>> {
    let mut out = Vec::new();
    for array in column(batches, name) {
        let array = array.as_primitive::<UInt64Type>();
        out.extend((0..array.len()).map(|i| array.is_valid(i).then(|| array.value(i))));
    }
    out
}

fn flags(batches: &[RecordBatch], name: &str) -> Vec<Option<bool>> {
    let mut out = Vec::new();
    for array in column(batches, name) {
        let array = array.as_boolean();
        out.extend((0..array.len()).map(|i| array.is_valid(i).then(|| array.value(i))));
    }
    out
}

fn bits_of(hex: &str) -> u64 {
    u64::from_str_radix(hex, 16).unwrap()
}

fn row_values(rows: &[u8]) -> Vec<Value> {
    split_lines(rows).iter().map(|l| serde_json::from_slice(l).unwrap()).collect()
}

/// Every `_hex` of every row's state, and both residuals, against the Parquet columns, bit for bit.
fn assert_exact(rows: &[u8], batches: &[RecordBatch]) -> usize {
    let mut compared = 0;
    for (index, row) in row_values(rows).iter().enumerate() {
        let Some(state) = row["final_state"].as_object() else { continue };
        for (key, value) in state {
            let Some(base) = key.strip_suffix("_hex") else {
                if let Some(flag) = value.as_bool() {
                    assert_eq!(flags(batches, key)[index], Some(flag), "{key}");
                    compared += 1;
                }
                continue;
            };
            match value {
                Value::String(hex) => {
                    let got = floats(batches, base)[index].unwrap();
                    assert_eq!(got.to_bits(), bits_of(hex), "row {index} {base}");
                    compared += 1;
                }
                Value::Array(items) => {
                    for (i, item) in items.iter().enumerate() {
                        let got = floats(batches, &format!("{base}_{i}"))[index].unwrap();
                        assert_eq!(got.to_bits(), bits_of(item.as_str().unwrap()), "row {index} {base}_{i}");
                        compared += 1;
                    }
                }
                _ => panic!("a hex that is neither text nor an array"),
            }
        }
        for (column_name, key) in [
            ("final_residual_conservation_error", "final_residual_conservation_error_hex"),
            ("max_residual_conservation_error", "max_residual_conservation_error_hex"),
        ] {
            let got = floats(batches, column_name)[index].unwrap();
            assert_eq!(got.to_bits(), bits_of(row[key].as_str().unwrap()), "row {index} {column_name}");
            compared += 1;
        }
    }
    compared
}

#[test]
fn a_dataset_that_holds_converts_and_every_exact_value_is_the_hex_bit_for_bit() {
    needs_saved_runs!();
    for (name, files) in [
        ("satellite", dataset_of(&eight_short_runs(), "frames")),
        ("capsule", capsule_with_unadmitted("frames")),
    ] {
        let dir = folder(&format!("exact_{name}"));
        let out = dir.join("out");
        let ran = convert(&dir, &files.head, &files.rows, files.frames.as_deref(), &out);
        assert_eq!((ran.code, ran.stderr.as_str()), (0, ""), "{}", ran.stdout);
        assert!(ran.stdout.starts_with("holds: true\ncause: none\n"), "{}", ran.stdout);
        let (batches, _) = read_parquet(&out.join("rows.parquet"));
        let compared = assert_exact(&files.rows, &batches);
        assert!(compared > 40, "{name}: only {compared} values compared");
        // The request is the row's own JSON text.
        let requests = strings(&batches, "request");
        for (i, row) in row_values(&files.rows).iter().enumerate() {
            assert_eq!(serde_json::from_str::<Value>(requests[i].as_ref().unwrap()).unwrap(), row["request"]);
        }
    }
}

#[test]
fn the_satellite_dataset_has_its_3_vectors_as_three_columns_each() {
    needs_saved_runs!();
    let files = dataset_of(&eight_short_runs(), "frames");
    let dir = folder("columns");
    let ran = convert(&dir, &files.head, &files.rows, files.frames.as_deref(), &dir.join("out"));
    assert_eq!(ran.code, 0);
    let (batches, _) = read_parquet(&dir.join("out/rows.parquet"));
    let names: Vec<String> = batches[0].schema().fields().iter().map(|f| f.name().clone()).collect();
    for expected in ["position_m_0", "position_m_1", "position_m_2", "velocity_m_s_2", "in_earth_shadow", "radius_m",
        "final_residual_conservation_error", "max_residual_conservation_error", "run_seal", "state_digest"] {
        assert!(names.iter().any(|n| n == expected), "{expected} in {names:?}");
    }
    assert!(!names.iter().any(|n| n.ends_with("_hex")));
}

#[test]
fn a_float_is_taken_from_the_hex_and_never_from_the_decimal() {
    needs_saved_runs!();
    let files = dataset_of(&eight_short_runs(), "frames");
    // The decimal beside the hex is changed in a copy. The seals do not cover
    // a decimal, so the seals hold; the file is no longer the one the head names.
    let changed = edit_row(&files.rows, 2, |row| {
        row["final_state"]["radius_m"] = json!(1.5);
        row["final_state"]["position_m"] = json!([9.0, 9.0, 9.0]);
    });
    assert_ne!(changed, files.rows);
    let report = check_dataset(&files.head, &changed, files.frames.as_deref()).unwrap();
    assert!(report.holds);

    // Head not changed: the rows file is not the one the head names. Exit 3.
    let dir = folder("decimal_a");
    let ran = convert(&dir, &files.head, &changed, files.frames.as_deref(), &dir.join("out"));
    assert_eq!(ran.code, 3, "{}{}", ran.stdout, ran.stderr);
    assert_eq!(contents(&dir.join("out")), None);

    // Head names the changed file: it converts, and the column is the hex value.
    let head = head_naming(&files.head, &changed, files.frames.as_deref());
    let dir = folder("decimal_b");
    let ran = convert(&dir, &head, &changed, files.frames.as_deref(), &dir.join("out"));
    assert_eq!(ran.code, 0, "{}{}", ran.stdout, ran.stderr);
    let (batches, _) = read_parquet(&dir.join("out/rows.parquet"));
    let radius = floats(&batches, "radius_m")[2].unwrap();
    assert_ne!(radius, 1.5);
    let original = row_values(&files.rows)[2]["final_state"]["radius_m_hex"].as_str().unwrap().to_string();
    assert_eq!(radius.to_bits(), bits_of(&original));
    assert_exact(&changed, &batches);
}

#[test]
fn one_changed_byte_in_a_frame_is_exit_1_and_nothing_is_written() {
    needs_saved_runs!();
    let files = dataset_of(&eight_short_runs(), "frames");
    let mut frames = files.frames.clone().unwrap();
    frames[64 + 3 * 113 * 64 + 57 * 64 + 20] ^= 1; // row 3, frame 57, byte 20
    let dir = folder("frame_byte");
    let out = dir.join("out");
    let ran = convert(&dir, &files.head, &files.rows, Some(&frames), &out);
    assert_eq!(ran.code, 1, "{}{}", ran.stdout, ran.stderr);
    assert!(ran.stdout.contains("holds: false\ncause: chain\nrow: 3\nbroken_link: 58\n"), "{}", ran.stdout);
    assert_eq!(contents(&out), None);
    // Even when the head is made to name the changed file, the chain is broken.
    let head = head_naming(&files.head, &files.rows, Some(&frames));
    let ran = convert(&dir, &head, &files.rows, Some(&frames), &out);
    assert_eq!(ran.code, 1);
    assert_eq!(contents(&out), None);
}

#[test]
fn a_swapped_row_is_exit_1_and_nothing_is_written() {
    needs_saved_runs!();
    let files = dataset_of(&eight_short_runs(), "frames");
    let mut lines = split_lines(&files.rows);
    lines.swap(1, 4);
    let swapped = join_lines(&lines);
    let head = head_naming(&files.head, &swapped, files.frames.as_deref());
    let dir = folder("swapped");
    let out = dir.join("out");
    let ran = convert(&dir, &head, &swapped, files.frames.as_deref(), &out);
    assert_eq!(ran.code, 1, "{}{}", ran.stdout, ran.stderr);
    assert!(ran.stdout.contains("cause: plan\nrow: 1\n"), "{}", ran.stdout);
    assert_eq!(contents(&out), None);
}

#[test]
fn a_rows_file_that_is_not_the_one_the_head_names_is_exit_3_and_nothing_is_written() {
    needs_saved_runs!();
    let files = capsule_with_unadmitted("frames");
    // The sentence of an unadmitted row is covered by no seal. Only the hash of the file covers it.
    let changed = edit_row(&files.rows, 3, |row| row["reject_detail"] = json!("another sentence"));
    let dir = folder("rows_hash");
    let out = dir.join("out");
    let ran = convert(&dir, &files.head, &changed, files.frames.as_deref(), &out);
    assert_eq!(ran.code, 3, "{}{}", ran.stdout, ran.stderr);
    assert!(ran.stdout.contains("holds: true\n") && ran.stdout.contains("rows_file: does not match the hash in the head"));
    assert_eq!(contents(&out), None);
    // A frame file that is not the one named: also 3. A frame file with a byte of the
    // unused header changed is refused (2) by the sealing crate's own reader.
    let mut frames = files.frames.clone().unwrap();
    frames[60] = 1;
    let ran = convert(&dir, &files.head, &files.rows, Some(&frames), &out);
    assert_eq!(ran.code, 2, "{}{}", ran.stdout, ran.stderr);
    assert!(ran.stderr.contains("refused: Bytes 56 to 63"), "{}", ran.stderr);
    assert_eq!(contents(&out), None);
}

#[test]
fn a_refused_row_and_an_unadmitted_row_convert_as_rows_with_nulls() {
    needs_saved_runs!();
    let files = capsule_with_unadmitted("frames");
    let dir = folder("nulls");
    let out = dir.join("out");
    let ran = convert(&dir, &files.head, &files.rows, files.frames.as_deref(), &out);
    assert_eq!(ran.code, 0, "{}{}", ran.stdout, ran.stderr);
    let (batches, metadata) = read_parquet(&out.join("rows.parquet"));
    assert_eq!(strings(&batches, "outcome"), ["completed", "completed", "refused", "unadmitted"].map(|s| Some(s.to_string())));
    // The refused row stopped after landed steps: it has a state.
    assert_eq!(strings(&batches, "reject")[2].as_deref(), Some("AltitudeOutsideCard"));
    assert!(floats(&batches, "altitude_m")[2].is_some());
    // The unadmitted row: every run column is null, the sentence is kept, frames are 0.
    for name in ["altitude_m", "mach", "final_residual_conservation_error", "max_residual_conservation_error"] {
        assert_eq!(floats(&batches, name)[3], None, "{name}");
    }
    for name in ["drogue_mach_window", "past_flight_deployment"] {
        assert_eq!(flags(&batches, name)[3], None, "{name}");
    }
    for name in ["steps_landed"] {
        assert_eq!(whole(&batches, name)[3], None);
    }
    for name in ["reject", "state_digest", "last_link", "run_seal"] {
        assert_eq!(strings(&batches, name)[3], None, "{name}");
    }
    assert!(strings(&batches, "reject_detail")[3].as_deref().unwrap().starts_with("Physical Violation"));
    assert_eq!(whole(&batches, "frames")[3], Some(0));
    assert_eq!(meta(&metadata, "unadmitted"), "1");
    assert_eq!(meta(&metadata, "refused"), "1");
    assert_exact(&files.rows, &batches);
}

#[test]
fn with_frames_and_without_frames() {
    needs_saved_runs!();
    // 1. frames stored and given: both files.
    let files = dataset_of(&eight_short_runs(), "frames");
    let dir = folder("with_frames");
    let out = dir.join("out");
    let ran = convert(&dir, &files.head, &files.rows, files.frames.as_deref(), &out);
    assert_eq!(ran.code, 0);
    assert_eq!(contents(&out).unwrap(), ["frames.parquet", "rows.parquet"]);
    let (frame_batches, metadata) = read_parquet(&out.join("frames.parquet"));
    assert_eq!(meta(&metadata, "table"), "frames");
    // every stored frame, byte for byte, in order, with its proof slot
    let source = files.frames.as_ref().unwrap();
    let mut index = 0;
    for batch in &frame_batches {
        let row = batch.column_by_name("row").unwrap().as_primitive::<UInt64Type>();
        let frame = batch.column_by_name("frame").unwrap().as_primitive::<UInt64Type>();
        let bytes = batch.column_by_name("frame_bytes").unwrap().as_fixed_size_binary();
        let proof = batch.column_by_name("proof_slot").unwrap().as_fixed_size_binary();
        for i in 0..batch.num_rows() {
            let start = 64 + index * 64;
            assert_eq!(bytes.value(i), &source[start..start + 64]);
            assert_eq!(proof.value(i), &source[start + 48..start + 64]);
            assert_eq!((row.value(i), frame.value(i)), ((index / 113) as u64, (index % 113) as u64));
            index += 1;
        }
    }
    assert_eq!(index, 8 * 113);

    // 2. frames stored, no file given: the rows convert, the chains are not rebuilt, no frames.parquet.
    let dir = folder("frames_not_given");
    let out = dir.join("out");
    let ran = convert(&dir, &files.head, &files.rows, None, &out);
    assert_eq!(ran.code, 0, "{}{}", ran.stdout, ran.stderr);
    assert!(ran.stdout.contains("chains: not rebuilt: the frames were not given"));
    assert_eq!(contents(&out).unwrap(), ["rows.parquet"]);
    let (_, metadata) = read_parquet(&out.join("rows.parquet"));
    assert_eq!(meta(&metadata, "source_frames_sha256"), "not given");

    // 3. a plan that stores no frames: rows only; a frame file given to it is refused.
    let none = dataset_of(&eight_short_runs(), "none");
    assert!(none.frames.is_none());
    let dir = folder("store_none");
    let out = dir.join("out");
    let ran = convert(&dir, &none.head, &none.rows, None, &out);
    assert_eq!(ran.code, 0, "{}{}", ran.stdout, ran.stderr);
    assert_eq!(contents(&out).unwrap(), ["rows.parquet"]);
    let ran = convert(&dir, &none.head, &none.rows, files.frames.as_deref(), &dir.join("out2"));
    assert_eq!(ran.code, 2);
    assert_eq!(contents(&dir.join("out2")), None);
}

#[test]
fn the_same_input_twice_gives_identical_bytes() {
    needs_saved_runs!();
    let files = capsule_with_unadmitted("frames");
    let mut seen: Vec<(Vec<u8>, Vec<u8>)> = Vec::new();
    for n in 0..2 {
        let dir = folder(&format!("twice_{n}"));
        let out = dir.join("out");
        assert_eq!(convert(&dir, &files.head, &files.rows, files.frames.as_deref(), &out).code, 0);
        seen.push((std::fs::read(out.join("rows.parquet")).unwrap(), std::fs::read(out.join("frames.parquet")).unwrap()));
        std::thread::sleep(std::time::Duration::from_millis(1100));
    }
    assert_eq!(seen[0], seen[1]);
    assert!(seen[0].0.len() > 1000 && seen[0].1.len() > 1000);
}

#[test]
fn the_row_count_and_the_frame_count_in_parquet_equal_the_heads() {
    needs_saved_runs!();
    for files in [capsule_with_unadmitted("frames"), dataset_of(&eight_short_runs(), "frames")] {
        let counts = files.assembled.head.counts.clone();
        let dir = folder("counts");
        let out = dir.join("out");
        let ran = convert(&dir, &files.head, &files.rows, files.frames.as_deref(), &out);
        assert_eq!(ran.code, 0);
        let (rows, rows_meta) = read_parquet(&out.join("rows.parquet"));
        let (frames, frames_meta) = read_parquet(&out.join("frames.parquet"));
        assert_eq!(rows.iter().map(|b| b.num_rows() as u64).sum::<u64>(), counts.rows);
        assert_eq!(frames.iter().map(|b| b.num_rows() as u64).sum::<u64>(), counts.frames);
        // the frames column of the rows adds up to the same number
        assert_eq!(whole(&rows, "frames").iter().map(|n| n.unwrap()).sum::<u64>(), counts.frames);
        for metadata in [&rows_meta, &frames_meta] {
            let head = &files.assembled.head;
            assert_eq!(meta(metadata, "dataset_seal"), head.dataset_seal);
            assert_eq!(meta(metadata, "merkle_root"), head.merkle_root);
            assert_eq!(meta(metadata, "plan_sha256"), head.plan_sha256);
            assert_eq!(meta(metadata, "engine_version"), counts.engine_version);
            assert_eq!(meta(metadata, "rows"), counts.rows.to_string());
            assert_eq!(meta(metadata, "frames"), counts.frames.to_string());
            assert_eq!(meta(metadata, "completed"), counts.completed.to_string());
            assert_eq!(meta(metadata, "source_rows_sha256"), head.rows_sha256.clone().unwrap());
            assert_eq!(meta(metadata, "source_frames_sha256"), head.frames_sha256.clone().unwrap());
            assert!(meta(metadata, "note").contains("The sealed record is the three source files"));
        }
    }
}

#[test]
fn a_row_whose_state_disagrees_with_the_first_is_refused_by_name_and_nothing_is_written() {
    needs_saved_runs!();
    let request = without_pair(&eight_short_runs()[0]["inputs"]);
    let mut request = request;
    request["record_every"] = json!(1);
    let state_a = json!({ "x_m": 1.0, "x_m_hex": "3ff0000000000000", "on": true });
    let state_b = json!({ "x_m": 1.0, "x_m_hex": "3ff0000000000000", "y_m": 2.0, "y_m_hex": "4000000000000000", "on": true });
    let mut other = request.clone();
    other["dt_s"] = json!(2.5);
    let mut tapes = Vec::new();
    for (inputs, state) in [(&request, state_a), (&other, state_b)] {
        let mut full = inputs.clone();
        full["domain"] = eight_short_runs()[0]["inputs"]["domain"].clone();
        full["card"] = eight_short_runs()[0]["inputs"]["card"].clone();
        tapes.push(fabricated_run(full, 3, 7, state));
    }
    let files = dataset_of(&tapes, "frames");
    assert!(check_dataset(&files.head, &files.rows, files.frames.as_deref()).unwrap().holds);
    let dir = folder("disagree");
    let out = dir.join("out");
    let ran = convert(&dir, &files.head, &files.rows, files.frames.as_deref(), &out);
    assert_eq!(ran.code, 2, "{}{}", ran.stdout, ran.stderr);
    assert!(ran.stderr.contains("Row 1: final_state does not agree with row 0: it has 'y_m' where the schema has 'on'"), "{}", ran.stderr);
    assert_eq!(contents(&out), None);
    // Two rows that agree convert.
    let files = dataset_of(&tapes[..1], "frames");
    let ran = convert(&dir, &files.head, &files.rows, files.frames.as_deref(), &out);
    assert_eq!(ran.code, 0, "{}", ran.stderr);
    let (batches, _) = read_parquet(&out.join("rows.parquet"));
    assert_eq!(floats(&batches, "x_m"), [Some(1.0)]);
    assert_eq!(flags(&batches, "on"), [Some(true)]);
}

#[test]
fn a_failure_after_the_check_leaves_nothing_and_no_new_folder() {
    needs_saved_runs!();
    // An output path that cannot be a folder: a file stands where the folder should be.
    let files = capsule_with_unadmitted("frames");
    let dir = folder("cannot_write");
    let blocker = write(&dir, "out", b"a file");
    let ran = convert(&dir, &files.head, &files.rows, files.frames.as_deref(), std::path::Path::new(&blocker));
    assert_eq!(ran.code, 2);
    assert_eq!(std::fs::read(&blocker).unwrap(), b"a file");
}

#[test]
fn the_check_names_what_check_dataset_names_given_the_whole_frame_file() {
    needs_saved_runs!();
    let files = dataset_of(&eight_short_runs(), "frames");
    let frames = files.frames.clone().unwrap();
    let dir = folder("agree");
    let at = |row: usize, frame: usize, byte: usize| 64 + (row * 113 + frame) * 64 + byte;
    let mut cases: Vec<(String, Vec<u8>, Vec<u8>, Vec<u8>)> = Vec::new();
    let case = |name: &str, head: &[u8], rows: &[u8], frames: &[u8], cases: &mut Vec<_>| {
        cases.push((name.to_string(), head.to_vec(), rows.to_vec(), frames.to_vec()));
    };
    case("holds", &files.head, &files.rows, &frames, &mut cases);
    for (row, frame, byte) in [(0, 0, 0), (3, 57, 20), (7, 112, 63), (4, 112, 9), (2, 5, 48)] {
        let mut f = frames.clone();
        f[at(row, frame, byte)] ^= 1;
        case(&format!("byte {row}/{frame}/{byte}"), &files.head, &files.rows, &f, &mut cases);
    }
    // a chain break in row 2 and a state break in row 5, and the reverse
    let state_5 = edit_row(&files.rows, 5, |r| {
        let hex = r["final_state"]["radius_m_hex"].as_str().unwrap().to_string();
        r["final_state"]["radius_m_hex"] = json!(flip(&hex));
    });
    let state_1 = edit_row(&files.rows, 1, |r| {
        let hex = r["final_state"]["radius_m_hex"].as_str().unwrap().to_string();
        r["final_state"]["radius_m_hex"] = json!(flip(&hex));
    });
    let mut broken_2 = frames.clone();
    broken_2[at(2, 10, 3)] ^= 1;
    case("chain 2 then state 5", &files.head, &state_5, &broken_2, &mut cases);
    case("state 1 then chain 2", &files.head, &state_1, &broken_2, &mut cases);
    // frame file of another dataset: the dataset seal
    let other = dataset_of(&eight_short_runs()[..4], "frames");
    let mut wrong_seal = frames.clone();
    wrong_seal[24..56].copy_from_slice(&other.frames.as_ref().unwrap()[24..56]);
    case("header seal", &files.head, &files.rows, &wrong_seal, &mut cases);
    let mut swapped = split_lines(&files.rows);
    swapped.swap(0, 1);
    case("plan", &files.head, &join_lines(&swapped), &frames, &mut cases);
    let mut head_root = files.assembled.head.clone();
    head_root.merkle_root = "0".repeat(64);
    case("root", head_root.wire().as_bytes(), &files.rows, &frames, &mut cases);

    for (name, head, rows, frames) in cases {
        let theirs = check_dataset(&head, &rows, Some(&frames)).unwrap();
        let path = write(&dir, "frames.bin", &frames);
        let ours = match check(&head, &rows, Some(std::path::Path::new(&path))) {
            Ok(checked) => checked.report,
            Err(Failure::Report { report, .. }) => *report,
            Err(Failure::Refused(reason)) => panic!("{name}: refused: {reason}"),
        };
        assert_eq!(ours, theirs, "{name}");
    }
    let _ = Cause::Plan;
}

fn flip(hex: &str) -> String {
    let mut bytes = hex.as_bytes().to_vec();
    bytes[5] = if bytes[5] == b'0' { b'1' } else { b'0' };
    String::from_utf8(bytes).unwrap()
}

#[test]
fn a_frame_file_that_changed_after_the_check_is_not_converted() {
    needs_saved_runs!();
    let files = dataset_of(&eight_short_runs(), "frames");
    let frames = files.frames.clone().unwrap();
    let head = head_naming(&files.head, &files.rows, Some(&frames));
    let dir = folder("changed_after_check");
    let checked_path = write(&dir, "checked.bin", &frames);
    let checked = check(&head, &files.rows, Some(std::path::Path::new(&checked_path))).unwrap();
    // The same length, one byte changed in the last frame, after the check.
    let mut changed = frames.clone();
    let last = changed.len() - 1;
    changed[last] ^= 1;
    let changed_path = write(&dir, "changed.bin", &changed);
    let out = dir.join("out");
    let refusal = genesis_seal_parquet::convert::convert(
        &checked,
        &files.rows,
        Some(std::path::Path::new(&changed_path)),
        &out,
    )
    .unwrap_err();
    assert!(refusal.contains("changed between the check and the conversion"), "{refusal}");
    assert!(!out.exists(), "nothing is left behind");
    // The file that was checked converts.
    let written = genesis_seal_parquet::convert::convert(
        &checked,
        &files.rows,
        Some(std::path::Path::new(&checked_path)),
        &out,
    )
    .unwrap();
    assert_eq!(written.frame_records, Some(8 * 113));
}

/// The program with a time limit: a hang fails the test instead of the build.
fn run_within(arguments: &[String], seconds: u64) -> (i32, String) {
    let mut child = std::process::Command::new(env!("CARGO_BIN_EXE_genesis_seal_parquet"))
        .args(arguments)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    let start = std::time::Instant::now();
    while child.try_wait().unwrap().is_none() {
        if start.elapsed().as_secs() >= seconds {
            let _ = child.kill();
            panic!("the program did not end within {seconds} s");
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    let output = child.wait_with_output().unwrap();
    (output.status.code().unwrap(), String::from_utf8(output.stderr).unwrap())
}

#[test]
fn a_named_pipe_a_link_and_a_device_as_the_frame_file_are_exit_2_by_name_and_nothing_is_written() {
    needs_saved_runs!();
    let files = dataset_of(&eight_short_runs(), "frames");
    let dir = folder("not_regular_frames");
    let head = write(&dir, "head.json", &files.head);
    let rows = write(&dir, "rows.jsonl", &files.rows);
    let out = dir.join("out");
    let _ = std::fs::remove_dir_all(&out);
    let fifo = dir.join("frames.fifo");
    let _ = std::fs::remove_file(&fifo);
    assert!(std::process::Command::new("mkfifo").arg(&fifo).status().unwrap().success());
    let link = dir.join("frames.link");
    let _ = std::fs::remove_file(&link);
    std::os::unix::fs::symlink("/dev/zero", &link).unwrap();
    for (path, word) in [(fifo.display().to_string(), "a named pipe"), (link.display().to_string(), "symbolic link"), ("/dev/zero".to_string(), "a device")] {
        let (code, err) = run_within(&[head.clone(), rows.clone(), path.clone(), out.display().to_string()], 20);
        assert_eq!(code, 2, "{path}: {err}");
        assert!(err.contains(word), "{path}: {err}");
        assert!(contents(&out).is_none(), "nothing is written for {path}");
    }
    // The head and the rows file are held to the same rule.
    let (code, err) = run_within(&[fifo.display().to_string(), rows.clone(), out.display().to_string()], 20);
    assert_eq!(code, 2);
    assert!(err.contains("a named pipe"), "{err}");
}

#[test]
fn a_symbolic_link_to_the_true_frame_file_is_refused_and_the_file_itself_converts() {
    needs_saved_runs!();
    let files = dataset_of(&eight_short_runs(), "frames");
    let dir = folder("link_to_true");
    let out = dir.join("out");
    let _ = std::fs::remove_dir_all(&out);
    let real = write(&dir, "frames.bin", files.frames.as_deref().unwrap());
    let link = dir.join("frames.link");
    let _ = std::fs::remove_file(&link);
    std::os::unix::fs::symlink(&real, &link).unwrap();
    let head = write(&dir, "head.json", &files.head);
    let rows = write(&dir, "rows.jsonl", &files.rows);
    let (code, err) = run_within(&[head.clone(), rows.clone(), link.display().to_string(), out.display().to_string()], 20);
    assert_eq!(code, 2);
    assert!(err.contains("symbolic link"), "{err}");
    let (code, err) = run_within(&[head, rows, real, out.display().to_string()], 60);
    assert_eq!(code, 0, "{err}");
}

#[test]
fn a_stale_partial_file_from_a_stopped_run_is_removed_at_the_next_run_into_the_folder() {
    needs_saved_runs!();
    let files = dataset_of(&eight_short_runs(), "frames");
    let dir = folder("stale_partial");
    let out = dir.join("out");
    let _ = std::fs::remove_dir_all(&out);
    std::fs::create_dir_all(&out).unwrap();
    std::fs::write(out.join(".rows.parquet.partial"), b"left by a run that was killed").unwrap();
    std::fs::write(out.join(".frames.parquet.partial"), b"left by a run that was killed").unwrap();
    // Even a run that is refused clears them.
    let mut broken = files.frames.clone().unwrap();
    broken[100] ^= 1;
    let ran = convert(&dir, &files.head, &files.rows, Some(&broken), &out);
    assert_eq!(ran.code, 1, "{}", ran.stderr);
    assert_eq!(contents(&out), Some(vec![]));
    std::fs::write(out.join(".rows.parquet.partial"), b"stale").unwrap();
    let ran = convert(&dir, &files.head, &files.rows, files.frames.as_deref(), &out);
    assert_eq!(ran.code, 0, "{}", ran.stderr);
    assert_eq!(contents(&out), Some(vec!["frames.parquet".to_string(), "rows.parquet".to_string()]));
}

/// A dataset that holds an ended run: the row converts with its outcome and
/// the end's name, the head's ended count is carried into both files, and the
/// count is checked: the row's outcome changed to refused, or the head's
/// count changed, is not converted.
#[test]
fn an_ended_run_converts_and_its_count_is_carried_and_checked() {
    needs_saved_runs!();
    let mut request = without_pair(&eight_short_runs()[0]["inputs"]);
    request["record_every"] = json!(1);
    request["domain"] = eight_short_runs()[0]["inputs"]["domain"].clone();
    request["card"] = eight_short_runs()[0]["inputs"]["card"].clone();
    let state = json!({ "x_m": 1.0, "x_m_hex": "3ff0000000000000", "on": true });
    let mut later = request.clone();
    later["dt_s"] = json!(2.5);
    let tapes = vec![
        fabricated_run(request, 3, 7, state.clone()),
        fabricated_ended_run(later, 3, 2, 9, state, "PastFirstMaximum"),
    ];
    let files = dataset_of(&tapes, "frames");
    let counts = &files.assembled.head.counts;
    assert_eq!((counts.completed, counts.ended, counts.refused, counts.unadmitted), (1, 1, 0, 0));
    assert!(check_dataset(&files.head, &files.rows, files.frames.as_deref()).unwrap().holds);

    let dir = folder("ended");
    let out = dir.join("out");
    let ran = convert(&dir, &files.head, &files.rows, files.frames.as_deref(), &out);
    assert_eq!(ran.code, 0, "{}{}", ran.stdout, ran.stderr);
    let (batches, rows_meta) = read_parquet(&out.join("rows.parquet"));
    assert_eq!(strings(&batches, "outcome"), ["completed", "ended"].map(|s| Some(s.to_string())));
    assert_eq!(strings(&batches, "reject")[1].as_deref(), Some("PastFirstMaximum"));
    assert_eq!(whole(&batches, "steps_landed"), [Some(3), Some(2)]);
    assert_eq!(strings(&batches, "run_seal")[1], tapes[1]["run_seal"].as_str().map(str::to_string));
    let (frame_batches, frames_meta) = read_parquet(&out.join("frames.parquet"));
    assert_eq!(frame_batches.iter().map(|b| b.num_rows()).sum::<usize>(), 5);
    for metadata in [&rows_meta, &frames_meta] {
        assert_eq!(meta(metadata, "ended"), "1");
        assert_eq!(meta(metadata, "completed"), "1");
        assert_eq!(meta(metadata, "refused"), "0");
    }
    assert_exact(&files.rows, &batches);

    // The ended row made refused: its run seal no longer holds.
    let refused = edit_row(&files.rows, 1, |row| row["outcome"] = json!("refused"));
    let head = head_naming(&files.head, &refused, files.frames.as_deref());
    let out = dir.join("out_refused");
    let ran = convert(&dir, &head, &refused, files.frames.as_deref(), &out);
    assert_eq!(ran.code, 1, "{}{}", ran.stdout, ran.stderr);
    assert!(ran.stdout.contains("holds: false"), "{}", ran.stdout);
    assert_eq!(contents(&out), None);

    // The head's ended count changed: the dataset seal no longer holds.
    let mut head: Value = serde_json::from_slice(&files.head).unwrap();
    head["ended"] = json!(0);
    head["completed"] = json!(2);
    let head = serde_json::to_vec(&head).unwrap();
    let out = dir.join("out_count");
    let ran = convert(&dir, &head, &files.rows, files.frames.as_deref(), &out);
    assert_ne!(ran.code, 0, "{}{}", ran.stdout, ran.stderr);
    assert_eq!(contents(&out), None);
}
