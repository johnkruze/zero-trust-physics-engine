//! Datasets made from saved engine runs, checked with no engine, and then
//! attacked. The crate runs no physics, so each dataset is a `list` plan
//! whose rows are the requests of runs the engine made and saved.

mod support;

use std::io::{self, BufReader, Read};
use std::sync::OnceLock;

use genesis_seal::dataset::{
    Cause, Chains, DatasetRefusal, DatasetReport, DatasetWriter, FileCheck, Head, RowProof, RowResult, assemble, prepare_row,
    check_dataset as check_dataset_whole, check_dataset_stream, check_row_proof, row_proof,
};
use genesis_seal::frames::{FramesFile, HEADER_LEN};
use genesis_seal::plan::DatasetRequest;
use genesis_seal::receipt::ReceiptCheck;
use genesis_seal::rows::{Outcome, Row, RunRefusal, lines};
use genesis_seal::run::{FRAME_LEN, TapeCause};
use serde_json::{Value, json};
use support::*;

/// Eight rows of 113 frames each: one revolution of the satellite, eight
/// different start states.
fn eight() -> &'static Files {
    static FILES: OnceLock<Files> = OnceLock::new();
    FILES.get_or_init(|| dataset_of(&eight_short_runs(), "frames"))
}

/// A reader that gives one byte at a time.
struct OneByte<'a>(&'a [u8]);

impl Read for OneByte<'_> {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        match (self.0.split_first(), buffer.first_mut()) {
            (Some((first, rest)), Some(slot)) => {
                *slot = *first;
                self.0 = rest;
                Ok(1)
            }
            _ => Ok(0),
        }
    }
}

/// Every entry point of the check, over the same bytes. All of them must give
/// the same answer, report or refusal, and the hash of the frame file the
/// stream took must be the hash of the bytes.
fn check_dataset(
    head: &[u8],
    rows: &[u8],
    frames: Option<&[u8]>,
) -> Result<DatasetReport, DatasetRefusal> {
    let whole = check_dataset_whole(head, rows, frames);
    let streams = [
        check_dataset_stream(head, rows, frames),
        check_dataset_stream(head, rows, frames.map(OneByte)),
        check_dataset_stream(head, rows, frames.map(|bytes| BufReader::with_capacity(7, bytes))),
        check_dataset_stream(head, rows, frames.map(|bytes| BufReader::with_capacity(100, bytes))),
    ];
    for streamed in streams {
        match (&whole, streamed) {
            (Ok(report), Ok(streamed)) => {
                assert_eq!(report, &streamed.report);
                assert_eq!(streamed.rows_sha384, genesis_seal::sealed_form::digest_hex(rows));
                assert_eq!(
                    streamed.frames_sha384,
                    frames.map(genesis_seal::sealed_form::digest_hex)
                );
            }
            (Err(refusal), Err(streamed)) => assert_eq!(refusal, &streamed),
            (whole, streamed) => panic!("the entry points differ: {whole:?} and {streamed:?}"),
        }
    }
    whole
}

fn check(head: &[u8], rows: &[u8], frames: Option<&[u8]>) -> DatasetReport {
    check_dataset(head, rows, frames)
        .unwrap_or_else(|refusal| panic!("the dataset was refused: {refusal}"))
}

fn check_files(files: &Files) -> DatasetReport {
    check(&files.head, &files.rows, files.frames.as_deref())
}

/// The first break a check names: its cause, its row and its frame index.
fn first_break(head: &[u8], rows: &[u8], frames: Option<&[u8]>) -> (Cause, Option<u64>, Option<u64>) {
    let report = check(head, rows, frames);
    assert!(!report.holds, "the dataset holds");
    (report.cause.expect("a break has a cause"), report.row, report.broken_link)
}

fn frames_of(files: &Files) -> &[u8] {
    files.frames.as_deref().expect("the dataset stores frames")
}

/// The byte in the frame file where frame `frame` of row `row` starts, for
/// the dataset of eight rows of 113 frames.
fn frame_start(row: usize, frame: usize) -> usize {
    HEADER_LEN + FRAME_LEN * (113 * row + frame)
}

#[test]
fn every_saved_run_is_a_row_of_a_dataset_that_holds() {
    needs_saved_runs!();
    let mut rows = 0;
    for (folder, expected_rows, expected_refused) in
        [(CAPSULE, 3, 1), (GENERATOR, 2, 0), (SATELLITE, 56, 0)]
    {
        let tapes: Vec<Value> = tapes_in(folder).into_iter().map(|(_, _, tape)| tape).collect();
        let files = dataset_of(&tapes, "frames");
        let report = check_files(&files);
        assert!(report.holds, "{folder}: {:?} at row {:?}", report.cause, report.row);
        assert_eq!(report.chains, Chains::FromFrames);
        assert_eq!(report.receipt, ReceiptCheck::Matches);
        assert_eq!(report.rows_file, FileCheck::Matches);
        assert_eq!(report.frames_file, FileCheck::Matches);
        let head = &files.assembled.head;
        assert_eq!(report.merkle_root.as_deref(), Some(head.merkle_root.as_str()));
        assert_eq!(report.dataset_seal.as_deref(), Some(head.dataset_seal.as_str()));
        assert_eq!(head.counts.rows, expected_rows, "{folder}");
        assert_eq!(head.counts.refused, expected_refused, "{folder}");
        assert_eq!(head.counts.completed, expected_rows - expected_refused);
        assert_eq!(head.counts.unadmitted, 0);
        assert_eq!(head.counts.engine_version, "genesis_kernel_v0.1.0");
        let recorded: u64 = tapes
            .iter()
            .map(|tape| tape["frames"].as_array().unwrap().len() as u64)
            .sum();
        assert_eq!(head.counts.frames, recorded);
        // Every row carries the run seal the engine gave that run alone.
        for (row, tape) in files.assembled.rows.iter().zip(&tapes) {
            assert_eq!(row.run_seal.as_deref(), tape["run_seal"].as_str());
        }
        rows += expected_rows;
    }
    assert_eq!(rows, 61);
}

#[test]
fn the_dataset_of_two_saved_runs_has_the_seal_worked_out_outside_this_crate() {
    needs_saved_runs!();
    // `vectors/outside_check_v2.py` (seal rule v2, ported from
    // evidence/verification/genesis_seal/outside_check.py) builds this
    // dataset from the design document with no code of this crate, and
    // prints these three hashes. Under seal rule v1 they were 6e5efdfd…3b70,
    // ca46864f…b10c and 8bdda680…35a6. The counts gained
    // `ended` in 0.7.0, and the leaf `reject_detail_sha384` in the version
    // after 0.3.0; the root and the seal were worked out again with Python's
    // hashlib from that program's own functions, with the leaf of five keys
    // (the leaf of four keys still gives the earlier root, 217411ee…f233,
    // and seal, 6c462cdb…4c1e).
    let tapes = vec![
        read_tape("satellite_lageos1_validation/sealed_lageos1_260912_d6_1356_steps.json"),
        read_tape("satellite_lageos1_validation/sealed_lageos1_260919_d6_1356_steps.json"),
    ];
    let files = dataset_of(&tapes, "frames");
    let head = &files.assembled.head;
    assert_eq!(
        head.plan_sha384,
        "1ca30f7cf6ab1b00a4dc709985829ab45c72996e04124b178fe1b788eac1b5a492c0b107db0ed6c21df8e2d306f1ccd5"
    );
    assert_eq!(
        head.merkle_root,
        "df3a8f536f426ca90f532dc215caba67f11483a17112a13262cf7879dd4b49fd0a4a325aea3d83683b72de379c05cf9d"
    );
    assert_eq!(
        head.dataset_seal,
        "0f0cc3552d3b1d08537ed7580e1a230c0ffe7cc9b2329b6cf7d2a1aabfd3a86b45146d289e1b2236b58d5f4617b7c8e3"
    );
    assert_eq!(
        head.counts.text(),
        r#"{"engine_version":"genesis_kernel_v0.1.0","rows":2,"completed":2,"ended":0,"refused":0,"unadmitted":0,"frames":226}"#
    );
}

#[test]
fn the_files_read_back_as_they_were_written() {
    needs_saved_runs!();
    let files = eight();
    let head = Head::read(&files.head).unwrap();
    assert_eq!(head, files.assembled.head);
    assert_eq!(head.counts.rows, 8);
    assert_eq!(head.counts.frames, 904);
    let read: Vec<Row> = lines(&files.rows)
        .unwrap()
        .into_iter()
        .map(|line| Row::read_line(line).unwrap())
        .collect();
    assert_eq!(read, files.assembled.rows);
    let file = FramesFile::read(frames_of(files)).unwrap();
    assert_eq!(file.header.rows, 8);
    assert_eq!(file.header.frames, 904);
    assert_eq!(hex::encode(file.header.dataset_seal), head.dataset_seal);
    assert_eq!(frames_of(files).len(), 80 + 64 * 904);
    assert_eq!(frames_of(files)[72..80], [0; 8]);
}

#[test]
fn the_same_plan_and_the_same_runs_give_the_same_bytes() {
    needs_saved_runs!();
    let again = dataset_of(&eight_short_runs(), "frames");
    assert_eq!(again.head, eight().head);
    assert_eq!(again.rows, eight().rows);
    assert_eq!(again.frames, eight().frames);
}

#[test]
fn the_order_of_keys_in_the_request_does_not_change_the_dataset_seal() {
    needs_saved_runs!();
    let tapes = eight_short_runs();
    let request = list_request(&tapes, "frames", "file");
    // The same request typed with every object's keys in reverse order.
    fn reversed(value: &Value) -> Value {
        match value {
            Value::Array(items) => Value::Array(items.iter().map(reversed).collect()),
            Value::Object(map) => {
                Value::Object(map.iter().rev().map(|(key, item)| (key.clone(), reversed(item))).collect())
            }
            other => other.clone(),
        }
    }
    let typed_backwards = reversed(request.value());
    assert_ne!(
        serde_json::to_string(&typed_backwards).unwrap(),
        serde_json::to_string(request.value()).unwrap()
    );
    let other = DatasetRequest::from_value(typed_backwards).unwrap();
    let results: Vec<RowResult> = tapes.iter().cloned().map(RowResult::Run).collect();
    let files = files_of(assemble(&other, "genesis_kernel_v0.1.0", &results).unwrap());
    let head = &files.assembled.head;
    assert_eq!(head.plan_sha384, eight().assembled.head.plan_sha384);
    assert_eq!(head.merkle_root, eight().assembled.head.merkle_root);
    assert_eq!(head.dataset_seal, eight().assembled.head.dataset_seal);
    assert_eq!(files.rows, eight().rows);
    assert_eq!(files.frames, eight().frames);
    assert!(check_files(&files).holds);
    // The two heads are the heads of one dataset although the requests were written differently,
    // and their written forms (the receipt covers the request as typed) differ.
    let first = &eight().assembled.head;
    assert_ne!(head.request.to_string(), first.request.to_string());
    assert_ne!(head.wire(), first.wire());
    assert!(head.is_same_dataset_as(first) && first.is_same_dataset_as(head));
    // Any other field changed, or another request, is another dataset.
    let mut other_seal = head.clone();
    other_seal.dataset_seal = "00".repeat(48);
    assert!(!other_seal.is_same_dataset_as(first));
    let mut other_hash = head.clone();
    other_hash.rows_sha384 = Some("11".repeat(48));
    assert!(!other_hash.is_same_dataset_as(first));
    let mut other_request = head.clone();
    other_request.request["record_every_changed"] = json!(1);
    assert!(!other_request.is_same_dataset_as(first));
}

#[test]
fn a_refused_run_and_a_row_that_was_not_admitted_are_rows() {
    needs_saved_runs!();
    let mut tapes: Vec<Value> = tapes_in(CAPSULE).into_iter().map(|(_, _, tape)| tape).collect();
    assert_eq!(tapes[2]["outcome"], json!("refused"));
    // A fourth request the door would refuse before computing: its radius
    // is under the ground. No run of it exists.
    let mut below_ground = tapes[0]["inputs"].clone();
    below_ground["initial"]["radius_m"] = json!(6000000.0);
    tapes.push(json!({ "inputs": below_ground, "engine_version": "genesis_kernel_v0.1.0" }));
    let request = list_request(&tapes, "frames", "file");
    let sentence = "Physical Violation: altitude is above 134.7 km or below 30 km.";
    let results = vec![
        RowResult::Run(tapes[0].clone()),
        RowResult::Run(tapes[1].clone()),
        RowResult::Run(tapes[2].clone()),
        RowResult::Unadmitted(sentence.to_string()),
    ];
    let files = files_of(assemble(&request, "genesis_kernel_v0.1.0", &results).unwrap());
    let counts = &files.assembled.head.counts;
    assert_eq!((counts.rows, counts.completed, counts.refused, counts.unadmitted), (4, 2, 1, 1));
    assert_eq!(counts.frames, 2800 + 5600 + 2865);
    let refused = &files.assembled.rows[2];
    assert_eq!(refused.outcome, Outcome::Refused);
    assert_eq!(refused.reject.as_deref(), Some("AltitudeOutsideCard"));
    assert_eq!(refused.run_seal.as_deref(), tapes[2]["run_seal"].as_str());
    let unadmitted = &files.assembled.rows[3];
    assert_eq!(unadmitted.outcome, Outcome::Unadmitted);
    assert_eq!(unadmitted.run_seal, None);
    assert_eq!(unadmitted.reject_detail.as_deref(), Some(sentence));
    assert_eq!(unadmitted.leaf["run_seal"], Value::Null);
    assert_eq!(unadmitted.leaf["outcome"], json!("unadmitted"));
    assert!(check_files(&files).holds);

    // The unadmitted row is sealed by its leaf: its request cannot change.
    let rows = edit_row(&files.rows, 3, |row| row["request"]["initial"]["radius_m"] = json!(6000001.0));
    assert_eq!(first_break(&files.head, &rows, files.frames.as_deref()).0, Cause::Plan);
    let rows = edit_row(&files.rows, 3, |row| row["leaf"]["outcome"] = json!("completed"));
    assert_eq!(
        first_break(&files.head, &rows, files.frames.as_deref()),
        (Cause::Leaf, Some(3), None)
    );
    // Its sentence is sealed by its leaf: the sentence rewritten, removed, or
    // given to a row that had none breaks the leaf of that row.
    let rows = edit_row(&files.rows, 3, |row| row["reject_detail"] = json!("another sentence"));
    assert_eq!(first_break(&files.head, &rows, files.frames.as_deref()), (Cause::Leaf, Some(3), None));
    let rows = edit_row(&files.rows, 3, |row| row["reject_detail"] = Value::Null);
    assert_eq!(first_break(&files.head, &rows, files.frames.as_deref()), (Cause::Leaf, Some(3), None));
    let rows = edit_row(&files.rows, 2, |row| row["reject_detail"] = json!("Physical Violation: another sentence."));
    assert_eq!(first_break(&files.head, &rows, files.frames.as_deref()), (Cause::Leaf, Some(2), None));
    let rows = edit_row(&files.rows, 0, |row| row["reject_detail"] = json!("a note"));
    assert_eq!(first_break(&files.head, &rows, files.frames.as_deref()), (Cause::Leaf, Some(0), None));
    // The leaf carries the hash of the sentence, not the sentence.
    assert_eq!(unadmitted.leaf["reject_detail_sha384"], json!(genesis_seal::sealed_form::digest_hex(sentence.as_bytes())));
    assert_eq!(files.assembled.rows[0].leaf["reject_detail_sha384"], Value::Null);
}

#[test]
fn a_dataset_that_stores_no_frames_is_checked_from_each_last_link_on() {
    needs_saved_runs!();
    let tapes = eight_short_runs();
    let files = dataset_of(&tapes, "none");
    assert!(files.frames.is_none());
    assert_eq!(files.assembled.head.frames_sha384, None);
    // The count of recorded frames is sealed whether or not they are stored.
    assert_eq!(files.assembled.head.counts.frames, 904);
    let report = check(&files.head, &files.rows, None);
    assert!(report.holds);
    assert_eq!(report.chains, Chains::FramesNotStored);
    assert_eq!(report.frames_file, FileCheck::NotGiven);
    // `store` is part of the request, so the two datasets have two seals.
    assert_ne!(files.assembled.head.dataset_seal, eight().assembled.head.dataset_seal);
    assert_eq!(files.assembled.head.merkle_root, eight().assembled.head.merkle_root);
    assert_eq!(
        check_dataset(&files.head, &files.rows, eight().frames.as_deref()).unwrap_err(),
        DatasetRefusal::FramesNotStored
    );
    // From the last link on, each seal is still held.
    let rows = edit_row(&files.rows, 5, |row| {
        let text = row["last_link"].as_str().unwrap().to_string();
        row["last_link"] = json!(flip_hex_digit(&text, 0));
    });
    assert_eq!(first_break(&files.head, &rows, None), (Cause::RunSeal, Some(5), None));
    let rows = edit_row(&files.rows, 5, |row| row["frames"] = json!(112));
    assert_eq!(first_break(&files.head, &rows, None), (Cause::Chain, Some(5), None));

    // The frames of a dataset that stores them may be left out of a check.
    let report = check(&eight().head, &eight().rows, None);
    assert!(report.holds);
    assert_eq!(report.chains, Chains::FramesNotGiven);
}

#[test]
fn two_rows_swapped_are_a_plan_failure_at_the_first_of_them() {
    needs_saved_runs!();
    let files = eight();
    let mut swapped = split_lines(&files.rows);
    swapped.swap(2, 6);
    assert_eq!(
        first_break(&files.head, &join_lines(&swapped), files.frames.as_deref()),
        (Cause::Plan, Some(2), None)
    );
}

#[test]
fn two_rows_of_one_request_swapped_are_a_leaf_failure() {
    needs_saved_runs!();
    // A plan may list one run twice. The two rows hold the same run and
    // differ in their row numbers, which their leaves carry.
    let mut tapes = eight_short_runs();
    tapes.truncate(3);
    tapes.push(tapes[1].clone());
    let files = dataset_of(&tapes, "frames");
    assert!(check_files(&files).holds);
    let rows = &files.assembled.rows;
    assert_eq!(rows[1].run_seal, rows[3].run_seal);
    assert_ne!(rows[1].leaf, rows[3].leaf);
    let mut swapped = split_lines(&files.rows);
    swapped.swap(1, 3);
    assert_eq!(
        first_break(&files.head, &join_lines(&swapped), files.frames.as_deref()),
        (Cause::Leaf, Some(1), None)
    );
    // With each row number set to match its new place, the leaves are no
    // longer the stored ones.
    let rewritten = edit_row(&join_lines(&swapped), 1, |row| row["row"] = json!(1));
    assert_eq!(
        first_break(&files.head, &rewritten, files.frames.as_deref()),
        (Cause::Leaf, Some(1), None)
    );
}

#[test]
fn a_row_dropped_is_a_plan_failure() {
    needs_saved_runs!();
    let files = eight();
    for dropped in [0, 4, 7] {
        let mut fewer = split_lines(&files.rows);
        fewer.remove(dropped);
        assert_eq!(
            first_break(&files.head, &join_lines(&fewer), files.frames.as_deref()),
            (Cause::Plan, None, None),
            "row {dropped}"
        );
    }
}

#[test]
fn a_row_repeated_is_a_plan_failure_and_doubling_the_last_row_does_not_keep_the_root() {
    needs_saved_runs!();
    let files = eight();
    let mut more = split_lines(&files.rows);
    more.insert(3, more[3].clone());
    assert_eq!(
        first_break(&files.head, &join_lines(&more), files.frames.as_deref()),
        (Cause::Plan, None, None)
    );
    let mut doubled = split_lines(&files.rows);
    doubled.push(doubled[7].clone());
    assert_eq!(
        first_break(&files.head, &join_lines(&doubled), files.frames.as_deref()),
        (Cause::Plan, None, None)
    );

    // An honest plan of seven rows, and the honest plan that lists its last
    // run twice. A tree that copies its last leaf to fill a level gives the
    // two one root. This tree does not, and the row number in each leaf
    // would part them even if it did.
    let tapes = eight_short_runs();
    let seven = dataset_of(&tapes[..7], "frames");
    let mut with_last_twice = tapes[..7].to_vec();
    with_last_twice.push(tapes[6].clone());
    let eight_rows = dataset_of(&with_last_twice, "frames");
    assert!(check_files(&seven).holds);
    assert!(check_files(&eight_rows).holds);
    assert_ne!(seven.assembled.head.merkle_root, eight_rows.assembled.head.merkle_root);
    assert_ne!(seven.assembled.head.dataset_seal, eight_rows.assembled.head.dataset_seal);
    // The seven-row head does not hold the eight rows, with or without the
    // seven-row plan swapped for the eight-row one.
    assert_eq!(
        first_break(&seven.head, &eight_rows.rows, eight_rows.frames.as_deref()).0,
        Cause::Plan
    );
    let head = edit_head(&seven.head, |head| {
        head["request"] = eight_rows.assembled.head.request.clone();
        head["plan_sha384"] = json!(eight_rows.assembled.head.plan_sha384);
    });
    assert_eq!(
        first_break(&head, &eight_rows.rows, eight_rows.frames.as_deref()).0,
        Cause::Root
    );
}

#[test]
fn a_row_from_another_plan_is_a_plan_failure() {
    needs_saved_runs!();
    let tapes = eight_short_runs();
    let ours = dataset_of(&tapes[..4], "frames");
    let theirs = dataset_of(&tapes[4..], "frames");
    assert!(check_files(&ours).holds);
    assert!(check_files(&theirs).holds);
    // Row 2 of the other dataset in place of our row 2, with its frames.
    let mut lines = split_lines(&ours.rows);
    lines[2] = split_lines(&theirs.rows)[2].clone();
    let mut frames = frames_of(&ours).to_vec();
    let at = frame_start(2, 0);
    frames[at..at + 113 * FRAME_LEN].copy_from_slice(&frames_of(&theirs)[at..at + 113 * FRAME_LEN]);
    assert_eq!(
        first_break(&ours.head, &join_lines(&lines), Some(&frames)),
        (Cause::Plan, Some(2), None)
    );
    // Our head over all of their rows and frames.
    assert_eq!(
        first_break(&ours.head, &theirs.rows, theirs.frames.as_deref()),
        (Cause::Plan, Some(0), None)
    );
    // Their plan put into our head beside our plan hash.
    let head = edit_head(&ours.head, |head| head["request"] = theirs.assembled.head.request.clone());
    assert_eq!(
        first_break(&head, &theirs.rows, theirs.frames.as_deref()),
        (Cause::Plan, None, None)
    );
    // Their plan and their plan hash put into our head: our root is not theirs.
    let head = edit_head(&ours.head, |head| {
        head["request"] = theirs.assembled.head.request.clone();
        head["plan_sha384"] = json!(theirs.assembled.head.plan_sha384);
    });
    assert_eq!(
        first_break(&head, &theirs.rows, theirs.frames.as_deref()).0,
        Cause::Root
    );
}

#[test]
fn a_changed_frame_byte_is_a_chain_failure_at_its_row_where_the_chain_stops_joining() {
    needs_saved_runs!();
    let files = eight();
    // The frame file stores no link beside each frame. A changed byte in
    // the body of frame k leaves frame k carrying the link before it, and
    // shows at frame k + 1, which no longer carries the link of frame k.
    for (row, frame, byte) in [(0, 0, 0), (3, 57, 20), (7, 111, 47)] {
        let mut frames = frames_of(files).to_vec();
        frames[frame_start(row, frame) + byte] ^= 0x01;
        assert_eq!(
            first_break(&files.head, &files.rows, Some(&frames)),
            (Cause::Chain, Some(row as u64), Some(frame as u64 + 1)),
            "row {row} frame {frame}"
        );
    }
    // A changed byte in the proof slot of frame k shows at frame k.
    for (row, frame, byte) in [(0, 0, 48), (3, 57, 55), (7, 112, 63)] {
        let mut frames = frames_of(files).to_vec();
        frames[frame_start(row, frame) + byte] ^= 0x01;
        assert_eq!(
            first_break(&files.head, &files.rows, Some(&frames)),
            (Cause::Chain, Some(row as u64), Some(frame as u64)),
            "row {row} frame {frame}"
        );
    }
    // A changed byte in the body of a row's last frame: every frame carries
    // its link, and the chain does not end at the row's last link.
    let mut frames = frames_of(files).to_vec();
    frames[frame_start(4, 112) + 9] ^= 0x01;
    assert_eq!(
        first_break(&files.head, &files.rows, Some(&frames)),
        (Cause::Chain, Some(4), Some(113))
    );
}

#[test]
fn a_removed_frame_and_two_swapped_frames_are_chain_failures() {
    needs_saved_runs!();
    let files = eight();
    // Frame 9 of row 4 taken out, and the header's count lowered to match.
    let mut frames = frames_of(files).to_vec();
    let at = frame_start(4, 9);
    frames.drain(at..at + FRAME_LEN);
    frames[16..24].copy_from_slice(&903_u64.to_le_bytes());
    assert_eq!(
        first_break(&files.head, &files.rows, Some(&frames)),
        (Cause::Chain, Some(4), Some(9))
    );
    // The last frame of the file taken out: row 7 runs out of frames.
    let mut frames = frames_of(files).to_vec();
    frames.truncate(frames.len() - FRAME_LEN);
    frames[16..24].copy_from_slice(&903_u64.to_le_bytes());
    assert_eq!(
        first_break(&files.head, &files.rows, Some(&frames)),
        (Cause::Chain, Some(7), Some(112))
    );
    // Two frames of one row swapped.
    let mut frames = frames_of(files).to_vec();
    let (first, second) = (frame_start(2, 30), frame_start(2, 80));
    for offset in 0..FRAME_LEN {
        frames.swap(first + offset, second + offset);
    }
    assert_eq!(
        first_break(&files.head, &files.rows, Some(&frames)),
        (Cause::Chain, Some(2), Some(30))
    );
    // The frames of two rows swapped whole: each row is chained from its own request.
    let mut frames = frames_of(files).to_vec();
    let (first, second) = (frame_start(1, 0), frame_start(5, 0));
    for offset in 0..113 * FRAME_LEN {
        frames.swap(first + offset, second + offset);
    }
    assert_eq!(
        first_break(&files.head, &files.rows, Some(&frames)),
        (Cause::Chain, Some(1), Some(0))
    );
}

#[test]
fn a_changed_last_link_or_frame_count_is_a_chain_failure() {
    needs_saved_runs!();
    let files = eight();
    let rows = edit_row(&files.rows, 6, |row| {
        let text = row["last_link"].as_str().unwrap().to_string();
        row["last_link"] = json!(flip_hex_digit(&text, 63));
    });
    assert_eq!(
        first_break(&files.head, &rows, files.frames.as_deref()),
        (Cause::Chain, Some(6), Some(113))
    );
    // One frame fewer claimed for row 6: its chain stops short of its last link.
    let rows = edit_row(&files.rows, 6, |row| row["frames"] = json!(112));
    assert_eq!(
        first_break(&files.head, &rows, files.frames.as_deref()),
        (Cause::Chain, Some(6), Some(112))
    );
    // One more claimed: frame 113 of row 6 is the first frame of row 7.
    let rows = edit_row(&files.rows, 6, |row| row["frames"] = json!(114));
    assert_eq!(
        first_break(&files.head, &rows, files.frames.as_deref()),
        (Cause::Chain, Some(6), Some(113))
    );
}

#[test]
fn a_changed_state_or_residual_hex_digit_is_a_state_failure_at_its_row() {
    needs_saved_runs!();
    let files = eight();
    let rows = edit_row(&files.rows, 3, |row| {
        let text = row["final_state"]["radius_m_hex"].as_str().unwrap().to_string();
        row["final_state"]["radius_m_hex"] = json!(flip_hex_digit(&text, 15));
    });
    assert_eq!(
        first_break(&files.head, &rows, files.frames.as_deref()),
        (Cause::State, Some(3), None)
    );
    let rows = edit_row(&files.rows, 0, |row| {
        let text = row["max_residual_conservation_error_hex"].as_str().unwrap().to_string();
        row["max_residual_conservation_error_hex"] = json!(flip_hex_digit(&text, 4));
    });
    assert_eq!(
        first_break(&files.head, &rows, files.frames.as_deref()),
        (Cause::State, Some(0), None)
    );
    let rows = edit_row(&files.rows, 7, |row| row["final_state"]["in_earth_shadow"] = json!(true));
    assert_eq!(
        first_break(&files.head, &rows, files.frames.as_deref()),
        (Cause::State, Some(7), None)
    );
    let rows = edit_row(&files.rows, 7, |row| {
        let text = row["state_digest"].as_str().unwrap().to_string();
        row["state_digest"] = json!(flip_hex_digit(&text, 1));
    });
    assert_eq!(
        first_break(&files.head, &rows, files.frames.as_deref()),
        (Cause::State, Some(7), None)
    );
}

#[test]
fn a_changed_outcome_step_count_refusal_name_or_run_seal_is_a_run_seal_failure_at_its_row() {
    needs_saved_runs!();
    let files = eight();
    let edits: [(&str, fn(&mut Value)); 5] = [
        ("outcome", |row| row["outcome"] = json!("refused")),
        ("steps_landed", |row| row["steps_landed"] = json!(1355)),
        ("steps_landed up", |row| row["steps_landed"] = json!(1357)),
        ("reject", |row| row["reject"] = json!("RadiusOutsideCard")),
        ("run_seal", |row| {
            let text = row["run_seal"].as_str().unwrap().to_string();
            row["run_seal"] = json!(flip_hex_digit(&text, 10));
        }),
    ];
    for (name, edit) in edits {
        let rows = edit_row(&files.rows, 5, edit);
        assert_eq!(
            first_break(&files.head, &rows, files.frames.as_deref()),
            (Cause::RunSeal, Some(5), None),
            "{name}"
        );
    }
}

#[test]
fn a_changed_leaf_is_a_leaf_failure_at_its_row() {
    needs_saved_runs!();
    let files = eight();
    let edits: [(&str, fn(&mut Value)); 5] = [
        ("leaf.row", |row| row["leaf"]["row"] = json!(5)),
        ("leaf.outcome", |row| row["leaf"]["outcome"] = json!("refused")),
        ("leaf.run_seal", |row| row["leaf"]["run_seal"] = json!("00".repeat(48))),
        ("leaf.request_sha384", |row| row["leaf"]["request_sha384"] = json!("00".repeat(48))),
        ("row", |row| row["row"] = json!(5)),
    ];
    for (name, edit) in edits {
        let rows = edit_row(&files.rows, 4, edit);
        assert_eq!(
            first_break(&files.head, &rows, files.frames.as_deref()),
            (Cause::Leaf, Some(4), None),
            "{name}"
        );
    }
    // The keys of a leaf are in one order.
    let rows = edit_row(&files.rows, 4, |row| {
        let leaf = row["leaf"].as_object().unwrap().clone();
        row["leaf"] = Value::Object(leaf.into_iter().rev().collect());
    });
    assert_eq!(
        first_break(&files.head, &rows, files.frames.as_deref()),
        (Cause::Leaf, Some(4), None)
    );
}

#[test]
fn a_changed_root_seal_count_or_engine_name_in_the_head_is_named() {
    needs_saved_runs!();
    let files = eight();
    let frames = files.frames.as_deref();
    let flipped = |head: &mut Value, key: &str| {
        let text = head[key].as_str().unwrap().to_string();
        head[key] = json!(flip_hex_digit(&text, 20));
    };
    let head = edit_head(&files.head, |head| flipped(head, "plan_sha384"));
    assert_eq!(first_break(&head, &files.rows, frames), (Cause::Plan, None, None));
    let head = edit_head(&files.head, |head| head["request"]["return"] = json!("inline"));
    assert_eq!(first_break(&head, &files.rows, frames), (Cause::Plan, None, None));
    let head = edit_head(&files.head, |head| flipped(head, "merkle_root"));
    assert_eq!(first_break(&head, &files.rows, frames), (Cause::Root, None, None));
    let head = edit_head(&files.head, |head| flipped(head, "dataset_seal"));
    assert_eq!(first_break(&head, &files.rows, frames), (Cause::DatasetSeal, None, None));
    for (key, value) in [
        ("completed", json!(7)),
        ("refused", json!(1)),
        ("unadmitted", json!(1)),
        ("frames", json!(905)),
        ("rows", json!(9)),
        ("engine_version", json!("genesis_kernel_v0.1.1")),
    ] {
        let head = edit_head(&files.head, |head| head[key] = value);
        assert_eq!(
            first_break(&head, &files.rows, frames),
            (Cause::DatasetSeal, None, None),
            "{key}"
        );
    }
    // A head that was only written again by another JSON writer still
    // holds. Its receipt and nothing else says its bytes changed.
    let rewritten = edit_head(&files.head, |_| {});
    assert_eq!(rewritten, files.head);
    let spaced = String::from_utf8(files.head.clone()).unwrap().replacen("\"rows\":8", "\"rows\": 8", 1);
    let report = check(spaced.as_bytes(), &files.rows, frames);
    assert!(report.holds);
    assert_eq!(report.receipt, ReceiptCheck::DoesNotMatch);
}

#[test]
fn a_frame_file_that_names_another_dataset_is_a_dataset_seal_failure() {
    needs_saved_runs!();
    let files = eight();
    let mut frames = frames_of(files).to_vec();
    frames[24] ^= 0x01;
    assert_eq!(
        first_break(&files.head, &files.rows, Some(&frames)),
        (Cause::DatasetSeal, None, None)
    );
    let mut frames = frames_of(files).to_vec();
    frames[8..16].copy_from_slice(&9_u64.to_le_bytes());
    assert_eq!(
        first_break(&files.head, &files.rows, Some(&frames)),
        (Cause::DatasetSeal, None, None)
    );
    // A frame added after the last row, with the header's count raised.
    let mut frames = frames_of(files).to_vec();
    frames.extend_from_slice(&[0; FRAME_LEN]);
    frames[16..24].copy_from_slice(&905_u64.to_le_bytes());
    assert_eq!(
        first_break(&files.head, &files.rows, Some(&frames)),
        (Cause::DatasetSeal, None, None)
    );
    let report = check(&files.head, &files.rows, Some(&frames));
    assert_eq!(report.frames_file, FileCheck::DoesNotMatch);
}

#[test]
fn the_first_break_is_named_in_the_stated_order() {
    needs_saved_runs!();
    let files = eight();
    // A state break in row 2 and a chain break in row 5: row 2 comes first.
    let rows = edit_row(&files.rows, 2, |row| row["final_state"]["in_earth_shadow"] = json!(true));
    let mut frames = frames_of(files).to_vec();
    frames[frame_start(5, 3)] ^= 0x01;
    assert_eq!(
        first_break(&files.head, &rows, Some(&frames)),
        (Cause::State, Some(2), None)
    );
    // In one row the chain is named before the state, the state before the
    // run seal, and the run seal before the leaf.
    let rows = edit_row(&files.rows, 5, |row| {
        row["leaf"]["row"] = json!(0);
        row["reject"] = json!("X");
        row["final_state"]["in_earth_shadow"] = json!(true);
    });
    assert_eq!(first_break(&files.head, &rows, Some(&frames)), (Cause::Chain, Some(5), Some(4)));
    assert_eq!(
        first_break(&files.head, &rows, files.frames.as_deref()),
        (Cause::State, Some(5), None)
    );
    let rows = edit_row(&files.rows, 5, |row| {
        row["leaf"]["row"] = json!(0);
        row["reject"] = json!("X");
    });
    assert_eq!(
        first_break(&files.head, &rows, files.frames.as_deref()),
        (Cause::RunSeal, Some(5), None)
    );
    // A plan break in a later row is named before a chain break in an earlier one.
    let rows = edit_row(&files.rows, 7, |row| row["request"]["dt_s"] = json!(5.0));
    assert_eq!(
        first_break(&files.head, &rows, Some(&frames)),
        (Cause::Plan, Some(7), None)
    );
    // A row break is named before the root and the seal.
    let head = edit_head(&files.head, |head| {
        head["merkle_root"] = json!("00".repeat(48));
        head["dataset_seal"] = json!("00".repeat(48));
    });
    assert_eq!(
        first_break(&head, &files.rows, Some(&frames)),
        (Cause::Chain, Some(5), Some(4))
    );
    assert_eq!(first_break(&head, &files.rows, files.frames.as_deref()).0, Cause::Root);
}

#[test]
fn a_malformed_head_row_or_frame_file_is_refused_by_name() {
    needs_saved_runs!();
    let files = eight();
    let frames = files.frames.as_deref();
    let head = edit_head(&files.head, |head| {
        head.as_object_mut().unwrap().remove("merkle_root");
    });
    assert_eq!(
        check_dataset(&head, &files.rows, frames).unwrap_err().to_string(),
        "The head was not read. Missing required parameter: 'merkle_root'"
    );
    let head = edit_head(&files.head, |head| head["note"] = json!(1));
    assert!(matches!(
        check_dataset(&head, &files.rows, frames).unwrap_err(),
        DatasetRefusal::Head(_)
    ));
    let repeated = String::from_utf8(files.head.clone()).unwrap().replacen("{\"request\":", "{\"rows\":8,\"request\":", 1);
    assert_eq!(
        check_dataset(repeated.as_bytes(), &files.rows, frames).unwrap_err().to_string(),
        "The head was not read. Physical Violation: the key 'rows' is repeated."
    );
    let head = edit_head(&files.head, |head| head["request"]["plan"]["kind"] = json!("sweep"));
    assert!(matches!(
        check_dataset(&head, &files.rows, frames).unwrap_err(),
        DatasetRefusal::Plan(_)
    ));
    let rows = edit_row(&files.rows, 1, |row| row["run_seal"] = json!("AB".repeat(48)));
    assert_eq!(
        check_dataset(&files.head, &rows, frames).unwrap_err().to_string(),
        "Row 1 was not read. Physical Violation: 'run_seal' must be 96 lowercase hex digits."
    );
    let rows = edit_row(&files.rows, 1, |row| {
        row["final_state"]["radius_m_hex"] = json!("416771cd036861a6\nin_earth_shadow:true")
    });
    assert!(matches!(
        check_dataset(&files.head, &rows, frames).unwrap_err(),
        DatasetRefusal::Row { row: 1, .. }
    ));
    let mut unended = files.rows.clone();
    unended.pop();
    assert_eq!(
        check_dataset(&files.head, &unended, frames).unwrap_err(),
        DatasetRefusal::RowsNotLines
    );
    let mut short = frames_of(files).to_vec();
    short.pop();
    assert!(matches!(
        check_dataset(&files.head, &files.rows, Some(&short)).unwrap_err(),
        DatasetRefusal::Frames(_)
    ));
}

#[test]
fn a_dataset_made_with_no_engine_holds() {
    // The limit of an unkeyed seal, stated in the design and shown here.
    // Every rule that seals a dataset is published, and none of them needs a
    // secret. So a dataset whose runs no engine ever made, sealed by those
    // rules, holds: the check shows that nothing was altered after sealing,
    // and it cannot show who sealed it. What exposes this dataset is a
    // replay: send any row's request to the engine and compare run seals.
    // The engine's run seal for that request is not the one stored here.
    // Until receipts are signed, that replay is the only proof of origin.
    let request = |temperature: f64| {
        json!({
            "domain": "energy", "card": "thermal",
            "initial": { "temperature_k": temperature }, "dt_s": 0.1,
            "segments": [{ "steps": 40, "load": { "heat_power_w": 1.0, "ambient_k": 293.15 } }],
            "record_every": 3
        })
    };
    let tapes = vec![
        fabricated_run(request(293.15), 40, 3, 0x11),
        fabricated_run(request(300.0), 40, 3, 0x22),
        fabricated_run(request(310.0), 40, 3, 0x33),
    ];
    let files = dataset_of(&tapes, "frames");
    let report = check_files(&files);
    assert!(report.holds);
    assert_eq!(report.chains, Chains::FromFrames);
    assert_eq!(files.assembled.head.counts.frames, 42);
    let proof = row_proof(&files.head, &files.rows, files.frames.as_deref(), 1).unwrap();
    assert!(check_row_proof(proof.to_text().as_bytes()).unwrap().holds);
}

#[test]
fn results_that_are_not_the_plans_rows_are_refused() {
    needs_saved_runs!();
    let tapes = eight_short_runs();
    let request = list_request(&tapes[..3], "frames", "file");
    let results: Vec<RowResult> = tapes[..2].iter().cloned().map(RowResult::Run).collect();
    assert_eq!(
        assemble(&request, "genesis_kernel_v0.1.0", &results).unwrap_err(),
        DatasetRefusal::ResultsNotRows { results: 2, rows: 3 }
    );
    // The runs given in another order than the plan's.
    let results = vec![
        RowResult::Run(tapes[0].clone()),
        RowResult::Run(tapes[2].clone()),
        RowResult::Run(tapes[1].clone()),
    ];
    assert_eq!(
        assemble(&request, "genesis_kernel_v0.1.0", &results).unwrap_err(),
        DatasetRefusal::NotThePlansRow(1)
    );
    // A run whose own seals do not hold.
    let mut broken = tapes[1].clone();
    broken["outcome"] = json!("refused");
    let results = vec![
        RowResult::Run(tapes[0].clone()),
        RowResult::Run(broken),
        RowResult::Run(tapes[2].clone()),
    ];
    match assemble(&request, "genesis_kernel_v0.1.0", &results).unwrap_err() {
        DatasetRefusal::Run { row: 1, refusal: RunRefusal::DoesNotHold(report) } => {
            assert_eq!(report.cause, Some(TapeCause::RunSeal));
        }
        other => panic!("refused as {other:?}"),
    }
}

#[test]
fn every_row_is_shown_alone_by_its_proof() {
    needs_saved_runs!();
    let files = eight();
    let seal = &files.assembled.head.dataset_seal;
    for row in 0..8 {
        let proof = row_proof(&files.head, &files.rows, files.frames.as_deref(), row).unwrap();
        assert_eq!(proof.audit_path.len(), 3);
        assert_eq!(proof.frames.as_ref().map(Vec::len), Some(113));
        let text = proof.to_text();
        assert_eq!(RowProof::read(text.as_bytes()).unwrap(), proof);
        let report = check_row_proof(text.as_bytes()).unwrap();
        assert!(report.holds, "row {row}: {:?}", report.cause);
        assert_eq!(report.row, row);
        assert!(report.chain_rebuilt);
        assert_eq!(&report.dataset_seal, seal);
    }
    // Without the frames the row is checked from its last link on.
    let proof = row_proof(&files.head, &files.rows, None, 3).unwrap();
    assert_eq!(proof.frames, None);
    let report = check_row_proof(proof.to_text().as_bytes()).unwrap();
    assert!(report.holds);
    assert!(!report.chain_rebuilt);
    assert_eq!(
        row_proof(&files.head, &files.rows, None, 8).unwrap_err(),
        DatasetRefusal::NoSuchRow { row: 8, rows: 8 }
    );
    // A tree of 7 has paths of two lengths.
    let seven = dataset_of(&eight_short_runs()[..7], "none");
    let lengths: Vec<usize> = (0..7)
        .map(|row| row_proof(&seven.head, &seven.rows, None, row).unwrap().audit_path.len())
        .collect();
    assert_eq!(lengths, [3, 3, 3, 3, 3, 3, 2]);
}

#[test]
fn a_proof_for_one_row_offered_for_its_neighbour_is_a_root_failure() {
    needs_saved_runs!();
    let files = eight();
    let frames = files.frames.as_deref();
    let ours = row_proof(&files.head, &files.rows, frames, 3).unwrap();
    let neighbour = row_proof(&files.head, &files.rows, frames, 4).unwrap();
    // Row 4 and its frames on the path of row 3.
    let mut forged = ours.clone();
    forged.row = neighbour.row.clone();
    forged.frames = neighbour.frames.clone();
    let report = check_row_proof(forged.to_text().as_bytes()).unwrap();
    assert_eq!((report.holds, report.cause), (false, Some(Cause::Root)));
    // Row 4 renumbered as row 3, with its stored leaf left alone.
    forged.row.row = 3;
    let report = check_row_proof(forged.to_text().as_bytes()).unwrap();
    assert_eq!((report.holds, report.cause), (false, Some(Cause::Leaf)));
    // Row 4 renumbered as row 3 in its leaf as well.
    forged.row.leaf["row"] = json!(3);
    let report = check_row_proof(forged.to_text().as_bytes()).unwrap();
    assert_eq!((report.holds, report.cause), (false, Some(Cause::Root)));
    // The path of row 4 under row 3.
    let mut forged = ours.clone();
    forged.audit_path = neighbour.audit_path.clone();
    let report = check_row_proof(forged.to_text().as_bytes()).unwrap();
    assert_eq!((report.holds, report.cause), (false, Some(Cause::Root)));
    // A path with a hash taken off, and a tree said to be of another size.
    let mut forged = ours.clone();
    forged.audit_path.pop();
    assert_eq!(check_row_proof(forged.to_text().as_bytes()).unwrap().cause, Some(Cause::Root));
    let mut forged = ours;
    forged.counts.rows = 9;
    assert_eq!(check_row_proof(forged.to_text().as_bytes()).unwrap().cause, Some(Cause::Root));
}

#[test]
fn a_proof_of_a_row_from_another_plan_does_not_lead_to_this_dataset_seal() {
    needs_saved_runs!();
    let tapes = eight_short_runs();
    let ours = dataset_of(&tapes[..4], "frames");
    let theirs = dataset_of(&tapes[4..], "frames");
    let proof = row_proof(&theirs.head, &theirs.rows, theirs.frames.as_deref(), 2).unwrap();
    // The proof holds, for their dataset. The seal it leads to is not ours.
    let report = check_row_proof(proof.to_text().as_bytes()).unwrap();
    assert!(report.holds);
    assert_eq!(report.dataset_seal, theirs.assembled.head.dataset_seal);
    assert_ne!(report.dataset_seal, ours.assembled.head.dataset_seal);
    // Our seal written into their proof.
    let mut forged = proof.clone();
    forged.dataset_seal = ours.assembled.head.dataset_seal.clone();
    assert_eq!(
        check_row_proof(forged.to_text().as_bytes()).unwrap().cause,
        Some(Cause::DatasetSeal)
    );
    // Our seal and our plan hash.
    forged.plan_sha384 = ours.assembled.head.plan_sha384.clone();
    assert_eq!(
        check_row_proof(forged.to_text().as_bytes()).unwrap().cause,
        Some(Cause::DatasetSeal)
    );
    // Our seal, our plan hash and our root.
    forged.merkle_root = ours.assembled.head.merkle_root.clone();
    assert_eq!(
        check_row_proof(forged.to_text().as_bytes()).unwrap().cause,
        Some(Cause::Root)
    );
}

#[test]
fn a_changed_row_frame_or_count_in_a_proof_is_named() {
    needs_saved_runs!();
    let files = eight();
    let proof = row_proof(&files.head, &files.rows, files.frames.as_deref(), 6).unwrap();
    let cause = |proof: &RowProof| {
        let report = check_row_proof(proof.to_text().as_bytes()).unwrap();
        assert!(!report.holds);
        (report.cause.unwrap(), report.broken_link)
    };
    let mut changed = proof.clone();
    changed.frames.as_mut().unwrap()[50][7] ^= 0x01;
    assert_eq!(cause(&changed), (Cause::Chain, Some(51)));
    let mut changed = proof.clone();
    changed.frames.as_mut().unwrap()[50][50] ^= 0x01;
    assert_eq!(cause(&changed), (Cause::Chain, Some(50)));
    let mut changed = proof.clone();
    changed.frames.as_mut().unwrap().pop();
    assert_eq!(cause(&changed), (Cause::Chain, Some(112)));
    let mut changed = proof.clone();
    changed.row.final_state.as_mut().unwrap()["in_earth_shadow"] = json!(true);
    assert_eq!(cause(&changed), (Cause::State, None));
    let mut changed = proof.clone();
    changed.row.steps_landed = Some(1355);
    assert_eq!(cause(&changed), (Cause::RunSeal, None));
    let mut changed = proof.clone();
    changed.row.request["dt_s"] = json!(5.0);
    assert_eq!(cause(&changed), (Cause::Chain, Some(0)));
    let mut changed = proof.clone();
    changed.counts.completed = 7;
    assert_eq!(cause(&changed), (Cause::DatasetSeal, None));
    let mut changed = proof.clone();
    changed.counts.engine_version = "another_engine".to_string();
    assert_eq!(cause(&changed), (Cause::DatasetSeal, None));
    // Without frames a changed request shows in the leaf's request hash.
    let mut changed = proof;
    changed.frames = None;
    changed.row.request["dt_s"] = json!(5.0);
    assert_eq!(cause(&changed), (Cause::Leaf, None));
}

#[test]
fn a_malformed_proof_is_refused_by_name() {
    needs_saved_runs!();
    let files = eight();
    let proof = row_proof(&files.head, &files.rows, None, 0).unwrap();
    let edited = |edit: &dyn Fn(&mut Value)| {
        let mut value: Value = serde_json::from_str(&proof.to_text()).unwrap();
        edit(&mut value);
        check_row_proof(serde_json::to_string(&value).unwrap().as_bytes())
            .unwrap_err()
            .to_string()
    };
    assert_eq!(
        edited(&|proof| {
            proof.as_object_mut().unwrap().remove("audit_path");
        }),
        "The proof was not read. Missing required parameter: 'audit_path'"
    );
    assert_eq!(
        edited(&|proof| proof["audit_path"][0] = json!("ab")),
        "The proof was not read. Physical Violation: 'audit_path' must be 96 lowercase hex digits."
    );
    assert_eq!(
        edited(&|proof| proof["frames_hex"] = json!(["00"])),
        "The proof was not read. Physical Violation: 'frames_hex' must be 128 lowercase hex digits."
    );
    assert_eq!(
        edited(&|proof| proof["row"]["outcome"] = json!("done")),
        "The proof was not read. Physical Violation: 'outcome' must be \"completed\", \"ended\", \"refused\" or \"unadmitted\"."
    );
    // A dataset that does not hold gives no proof.
    let rows = edit_row(&files.rows, 2, |row| row["reject"] = json!("X"));
    match row_proof(&files.head, &rows, None, 0).unwrap_err() {
        DatasetRefusal::DoesNotHold(report) => {
            assert_eq!((report.cause, report.row), (Some(Cause::RunSeal), Some(2)));
        }
        other => panic!("refused as {other:?}"),
    }
}

/// The four results of a plan that holds a refused row and an unadmitted row.
fn four_results() -> (Vec<Value>, Vec<RowResult>) {
    let mut tapes: Vec<Value> = tapes_in(CAPSULE).into_iter().map(|(_, _, tape)| tape).collect();
    let mut below_ground = tapes[0]["inputs"].clone();
    below_ground["initial"]["radius_m"] = json!(6000000.0);
    tapes.push(json!({ "inputs": below_ground, "engine_version": "genesis_kernel_v0.1.0" }));
    let results = vec![
        RowResult::Run(tapes[0].clone()),
        RowResult::Run(tapes[1].clone()),
        RowResult::Run(tapes[2].clone()),
        RowResult::Unadmitted("Physical Violation: altitude is above 134.7 km or below 30 km.".to_string()),
    ];
    (tapes, results)
}

type Memory = std::io::Cursor<Vec<u8>>;

fn written(
    request: &genesis_seal::plan::DatasetRequest,
    results: &[RowResult],
) -> genesis_seal::dataset::Written<Vec<u8>, Memory> {
    let frames = (request.store() == genesis_seal::plan::Store::Frames).then(Memory::default);
    let mut writer =
        DatasetWriter::new(request, "genesis_kernel_v0.1.0", Vec::new(), frames).unwrap();
    for (index, result) in results.iter().enumerate() {
        writer.push(index as u64, result).unwrap();
    }
    writer.finish().unwrap()
}

#[test]
fn the_writer_and_assemble_give_the_same_head_rows_and_frames() {
    needs_saved_runs!();
    let (tapes, results) = four_results();
    for store in ["frames", "none"] {
        let request = list_request(&tapes, store, "file");
        let assembled = assemble(&request, "genesis_kernel_v0.1.0", &results).unwrap();
        let done = written(&request, &results);
        assert_eq!(done.head.wire(), assembled.head.wire(), "{store}");
        assert_eq!(done.rows, assembled.rows_text(), "{store}");
        assert_eq!(done.frames.map(Memory::into_inner), assembled.frames_bytes(), "{store}");
        // `assemble` leaves the two file hashes null for an inline job.
        let inline = list_request(&tapes, store, "inline");
        let assembled = assemble(&inline, "genesis_kernel_v0.1.0", &results).unwrap();
        assert_eq!(assembled.head.rows_sha384, None);
        let done = written(&inline, &results);
        let mut head = done.head.clone();
        head.rows_sha384 = None;
        head.frames_sha384 = None;
        assert_eq!(head, assembled.head, "{store}");
        assert_eq!(done.head.frames_sha384.is_some(), store == "frames");
    }
}

#[test]
fn the_writers_files_hold_and_match_the_hashes_in_the_head() {
    needs_saved_runs!();
    let (tapes, results) = four_results();
    for store in ["frames", "none"] {
        let request = list_request(&tapes, store, "file");
        let done = written(&request, &results);
        let frames = done.frames.map(Memory::into_inner);
        let report = check_dataset(done.head.wire().as_bytes(), &done.rows, frames.as_deref()).unwrap();
        assert!(report.holds, "{store}");
        assert_eq!(report.rows_file, FileCheck::Matches);
        assert_eq!(
            report.frames_file,
            if store == "frames" { FileCheck::Matches } else { FileCheck::NotGiven }
        );
    }
}

#[test]
fn the_writer_refuses_a_row_out_of_order_too_few_rows_and_too_many() {
    needs_saved_runs!();
    let (tapes, results) = four_results();
    let request = list_request(&tapes, "frames", "file");
    let start = || DatasetWriter::new(&request, "genesis_kernel_v0.1.0", Vec::new(), Some(Memory::default())).unwrap();

    let mut writer = start();
    assert_eq!(
        writer.push(1, &results[1]).unwrap_err(),
        DatasetRefusal::RowOutOfOrder { given: 1, expected: 0 }
    );
    writer.push(0, &results[0]).unwrap();
    assert_eq!(
        writer.push(0, &results[0]).unwrap_err(),
        DatasetRefusal::RowOutOfOrder { given: 0, expected: 1 }
    );
    assert_eq!(writer.next_row(), 1);

    let mut writer = start();
    for (index, result) in results.iter().take(3).enumerate() {
        writer.push(index as u64, result).unwrap();
    }
    assert_eq!(
        writer.finish().err().unwrap(),
        DatasetRefusal::ResultsNotRows { results: 3, rows: 4 }
    );

    let mut writer = start();
    for (index, result) in results.iter().enumerate() {
        writer.push(index as u64, result).unwrap();
    }
    assert_eq!(
        writer.push(4, &results[0]).unwrap_err(),
        DatasetRefusal::ResultsNotRows { results: 5, rows: 4 }
    );

    // A result that is another row's run, or that does not hold, is refused as `assemble` refuses it.
    let mut writer = start();
    assert_eq!(writer.push(0, &results[1]).unwrap_err(), DatasetRefusal::NotThePlansRow(0));
    let mut broken = tapes[0].clone();
    broken["outcome"] = json!("refused");
    assert!(matches!(
        writer.push(0, &RowResult::Run(broken)).unwrap_err(),
        DatasetRefusal::Run { row: 0, .. }
    ));

    // A frame sink goes with `store` frames only.
    assert_eq!(
        DatasetWriter::<Vec<u8>, Memory>::new(&request, "v", Vec::new(), None).err().unwrap(),
        DatasetRefusal::SinkNotAsStored
    );
    let none = list_request(&tapes, "none", "file");
    assert_eq!(
        DatasetWriter::new(&none, "v", Vec::new(), Some(Memory::default())).err().unwrap(),
        DatasetRefusal::SinkNotAsStored
    );
}

#[test]
fn a_prepared_row_pushed_out_of_order_or_after_the_last_is_refused() {
    needs_saved_runs!();
    let (tapes, results) = four_results();
    let request = list_request(&tapes, "frames", "file");
    let prepare = |index: usize| prepare_row(&request, index as u64, &results[index]).unwrap();
    let mut writer =
        DatasetWriter::new(&request, "genesis_kernel_v0.1.0", Vec::new(), Some(Memory::default())).unwrap();
    assert_eq!(
        writer.push_prepared(prepare(1)).unwrap_err(),
        DatasetRefusal::RowOutOfOrder { given: 1, expected: 0 }
    );
    writer.push_prepared(prepare(0)).unwrap();
    assert_eq!(
        writer.push_prepared(prepare(0)).unwrap_err(),
        DatasetRefusal::RowOutOfOrder { given: 0, expected: 1 }
    );
    assert_eq!(writer.next_row(), 1);
    for index in 1..4 {
        writer.push_prepared(prepare(index)).unwrap();
    }
    assert_eq!(
        writer.push_prepared(prepare(0)).unwrap_err(),
        DatasetRefusal::ResultsNotRows { results: 5, rows: 4 }
    );
    // A row that does not hold is refused at preparation, whatever its turn.
    assert_eq!(prepare_row(&request, 0, &results[1]).unwrap_err(), DatasetRefusal::NotThePlansRow(0));
}

#[test]
fn the_prepared_path_and_the_old_path_give_the_same_bytes() {
    needs_saved_runs!();
    let (tapes, results) = four_results();
    for store in ["frames", "none"] {
        let request = list_request(&tapes, store, "file");
        let old = written(&request, &results);
        // Prepare in the reverse of plan order, as workers may finish, then push in plan order.
        let mut prepared: Vec<_> = (0..results.len())
            .rev()
            .map(|index| prepare_row(&request, index as u64, &results[index]).unwrap())
            .collect();
        prepared.reverse();
        let frames = (request.store() == genesis_seal::plan::Store::Frames).then(Memory::default);
        let mut writer = DatasetWriter::new(&request, "genesis_kernel_v0.1.0", Vec::new(), frames).unwrap();
        for row in prepared {
            writer.push_prepared(row).unwrap();
        }
        let new = writer.finish().unwrap();
        assert_eq!(new.head.wire(), old.head.wire(), "{store}");
        assert_eq!(new.rows, old.rows, "{store}");
        assert_eq!(new.frames.map(Memory::into_inner), old.frames.map(Memory::into_inner), "{store}");
    }
}

// The check over a frame file read as a stream.

#[test]
fn a_frame_file_that_ends_inside_a_frame_or_has_extra_frames_is_refused_like_the_whole_file() {
    needs_saved_runs!();
    let files = eight();
    let frames = frames_of(files);
    for cut in [1, 31, 63, 64, 65, 113 * 64, 113 * 64 + 17, frames.len() - HEADER_LEN] {
        let short = &frames[..frames.len() - cut];
        let refusal = check_dataset(&files.head, &files.rows, Some(short)).unwrap_err();
        assert!(matches!(refusal, DatasetRefusal::Frames(_)), "cut {cut}: {refusal}");
    }
    let mut longer = frames.to_vec();
    longer.extend_from_slice(&[0; FRAME_LEN]);
    assert!(matches!(
        check_dataset(&files.head, &files.rows, Some(&longer)).unwrap_err(),
        DatasetRefusal::Frames(_)
    ));
    longer.extend_from_slice(&[9; 5]);
    assert!(matches!(
        check_dataset(&files.head, &files.rows, Some(&longer)).unwrap_err(),
        DatasetRefusal::Frames(_)
    ));
    // A frame file with a wrong length is refused before a bad rows file is.
    let mut unended = files.rows.clone();
    unended.pop();
    let short = &frames[..frames.len() - 3];
    assert!(matches!(
        check_dataset(&files.head, &unended, Some(short)).unwrap_err(),
        DatasetRefusal::Frames(_)
    ));
    assert_eq!(
        check_dataset(&files.head, &unended, Some(frames)).unwrap_err(),
        DatasetRefusal::RowsNotLines
    );
    let rows = edit_row(&files.rows, 0, |row| row["run_seal"] = json!("AB".repeat(48)));
    assert!(matches!(
        check_dataset(&files.head, &rows, Some(short)).unwrap_err(),
        DatasetRefusal::Frames(_)
    ));
    assert!(matches!(
        check_dataset(&files.head, &rows, Some(frames)).unwrap_err(),
        DatasetRefusal::Row { row: 0, .. }
    ));
}

#[test]
fn a_header_that_disagrees_with_the_dataset_is_named_the_same_way() {
    needs_saved_runs!();
    let files = eight();
    let frames = frames_of(files);
    // The count of rows, then the count of frames, then the seal.
    let mut rows_changed = frames.to_vec();
    rows_changed[8] ^= 1;
    let report = check(&files.head, &files.rows, Some(&rows_changed));
    assert_eq!((report.holds, report.cause), (false, Some(Cause::DatasetSeal)));
    // A count of frames that does not match the length is a refusal.
    let mut frames_changed = frames.to_vec();
    frames_changed[16] ^= 1;
    assert!(matches!(
        check_dataset(&files.head, &files.rows, Some(&frames_changed)).unwrap_err(),
        DatasetRefusal::Frames(_)
    ));
    let mut seal_changed = frames.to_vec();
    seal_changed[40] ^= 1;
    let report = check(&files.head, &files.rows, Some(&seal_changed));
    assert_eq!((report.holds, report.cause), (false, Some(Cause::DatasetSeal)));
    // The header's own refusals.
    for (at, value) in [(0, b'X'), (4, 1), (76, 1)] {
        let mut bytes = frames.to_vec();
        bytes[at] = value;
        assert!(matches!(
            check_dataset(&files.head, &files.rows, Some(&bytes)).unwrap_err(),
            DatasetRefusal::Frames(_)
        ));
    }
    for length in [0, 1, 79] {
        assert!(matches!(
            check_dataset(&files.head, &files.rows, Some(&frames[..length])).unwrap_err(),
            DatasetRefusal::Frames(_)
        ));
    }
    // A frame file for a plan that stores none is refused before it is read.
    let none = dataset_of(&eight_short_runs(), "none");
    assert_eq!(
        check_dataset(&none.head, &none.rows, Some(frames)).unwrap_err(),
        DatasetRefusal::FramesNotStored
    );
}

struct FailsAfter<'a>(&'a [u8], usize);

impl Read for FailsAfter<'_> {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        if self.1 == 0 {
            return Err(io::Error::other("the disk went away"));
        }
        let count = buffer.len().min(self.0.len()).min(self.1).min(50);
        buffer[..count].copy_from_slice(&self.0[..count]);
        self.0 = &self.0[count..];
        self.1 -= count;
        Ok(count)
    }
}

#[test]
fn a_frame_file_that_cannot_be_read_is_refused_and_not_taken_for_a_break() {
    needs_saved_runs!();
    let files = eight();
    let frames = frames_of(files);
    let refusal =
        check_dataset_stream(&files.head, &files.rows, Some(FailsAfter(frames, 5000))).unwrap_err();
    assert_eq!(
        refusal.to_string(),
        "The frame file could not be read: the disk went away"
    );
    let refusal =
        check_dataset_stream(&files.head, &files.rows, Some(FailsAfter(frames, 10))).unwrap_err();
    assert!(matches!(refusal, DatasetRefusal::Frames(_)));
}

/// The frame file is never asked for more than a block at a time.
struct Counting<'a> {
    bytes: &'a [u8],
    largest: usize,
}

impl Read for Counting<'_> {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        self.largest = self.largest.max(buffer.len());
        let count = buffer.len().min(self.bytes.len());
        buffer[..count].copy_from_slice(&self.bytes[..count]);
        self.bytes = &self.bytes[count..];
        Ok(count)
    }
}

#[test]
fn the_stream_asks_the_reader_for_a_block_at_a_time_and_never_the_whole_file() {
    needs_saved_runs!();
    let files = eight();
    let frames = frames_of(files);
    let mut reader = Counting { bytes: frames, largest: 0 };
    let streamed = check_dataset_stream(&files.head, &files.rows, Some(&mut reader)).unwrap();
    assert!(streamed.report.holds);
    assert!(reader.largest <= 1 << 14, "asked for {} bytes", reader.largest);
    assert!(reader.largest < frames.len());
}

#[test]
fn the_cases_of_the_conversion_tool_give_the_same_report_through_every_entry_point() {
    needs_saved_runs!();
    let files = eight();
    let frames = frames_of(files).to_vec();
    let at = |row: usize, frame: usize, byte: usize| frame_start(row, frame) + byte;
    let mut cases: Vec<(String, Vec<u8>, Vec<u8>, Vec<u8>)> = Vec::new();
    let mut case = |name: &str, head: &[u8], rows: &[u8], frames: &[u8]| {
        cases.push((name.to_string(), head.to_vec(), rows.to_vec(), frames.to_vec()));
    };
    case("holds", &files.head, &files.rows, &frames);
    for (row, frame, byte) in [(0, 0, 0), (3, 57, 20), (7, 112, 63), (4, 112, 9), (2, 5, 48)] {
        let mut f = frames.clone();
        f[at(row, frame, byte)] ^= 1;
        case(&format!("byte {row}/{frame}/{byte}"), &files.head, &files.rows, &f);
    }
    let flip_radius = |rows: &[u8], row: usize| {
        edit_row(rows, row, |r| {
            let hex = r["final_state"]["radius_m_hex"].as_str().unwrap().to_string();
            r["final_state"]["radius_m_hex"] = json!(flip_hex_digit(&hex, 5));
        })
    };
    let state_5 = flip_radius(&files.rows, 5);
    let state_1 = flip_radius(&files.rows, 1);
    let mut broken_2 = frames.clone();
    broken_2[at(2, 10, 3)] ^= 1;
    case("chain 2 then state 5", &files.head, &state_5, &broken_2);
    case("state 1 then chain 2", &files.head, &state_1, &broken_2);
    let other = dataset_of(&eight_short_runs()[..4], "frames");
    let mut wrong_seal = frames.clone();
    wrong_seal[24..56].copy_from_slice(&other.frames.as_ref().unwrap()[24..56]);
    case("header seal", &files.head, &files.rows, &wrong_seal);
    let mut swapped = split_lines(&files.rows);
    swapped.swap(0, 1);
    case("plan", &files.head, &join_lines(&swapped), &frames);
    let mut head_root = files.assembled.head.clone();
    head_root.merkle_root = "0".repeat(96);
    case("root", head_root.wire().as_bytes(), &files.rows, &frames);
    // a changed frame and a truncated or lengthened file, with a row also broken
    let mut short = broken_2.clone();
    short.truncate(short.len() - 100);
    case("chain break in a short file", &files.head, &files.rows, &short);
    let mut long = frames.clone();
    long.extend_from_slice(&[7; 64]);
    case("extra frame", &files.head, &state_5, &long);

    for (name, head, rows, frames) in cases {
        let result = check_dataset(&head, &rows, Some(&frames));
        let refused = name.contains("short") || name.contains("extra");
        assert_eq!(result.is_err(), refused, "{name}");
    }
}

/// A frame file that never ends: the true bytes, then zeros for ever.
struct EndlessAfter<'a>(&'a [u8]);

impl Read for EndlessAfter<'_> {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        let taken = self.0.len().min(buffer.len());
        buffer[..taken].copy_from_slice(&self.0[..taken]);
        buffer[taken..].fill(0);
        self.0 = &self.0[taken..];
        Ok(buffer.len())
    }
}

#[test]
fn a_reader_that_never_ends_is_ended_by_the_caller_at_the_size_taken_at_open() {
    needs_saved_runs!();
    // The library reads a frame file to its end, so that the length it reports
    // and the hash it takes are those of the whole file; stopping at the
    // header's count plus one byte would change the reported length of a finite
    // file with more extra bytes. The caller bounds the reader: `Read::take`
    // at the size the open file had, which is what the gateway, the program and
    // the Parquet tool do. A file that grew, or is endless, then reads as its
    // old size plus nothing, and the report is the one for that many bytes.
    let files = eight();
    let frames = frames_of(files);
    let exact = check_dataset_stream(&files.head, &files.rows, Some(EndlessAfter(frames).take(frames.len() as u64))).unwrap();
    assert!(exact.report.holds);
    assert_eq!(exact.frames_sha384, Some(genesis_seal::sealed_form::digest_hex(frames)));
    // One byte more than the file had: the extra byte is a length fault, by name, and the call returns.
    let more = check_dataset_stream(&files.head, &files.rows, Some(EndlessAfter(frames).take(frames.len() as u64 + 1)));
    assert!(matches!(more, Err(DatasetRefusal::Frames(genesis_seal::frames::FramesRefusal::WrongLength { .. }))), "{more:?}");
}
