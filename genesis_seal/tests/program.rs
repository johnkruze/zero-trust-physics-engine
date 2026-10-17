//! The program, run from end to end on saved engine runs.

mod support;

use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::{Value, json};
use support::*;

const PROGRAM: &str = env!("CARGO_BIN_EXE_genesis_seal");

/// A folder for one test's files, under the build's own temporary folder.
fn folder(name: &str) -> PathBuf {
    let path = Path::new(env!("CARGO_TARGET_TMPDIR")).join(name);
    std::fs::create_dir_all(&path).expect("a test folder is made");
    path
}

fn write(folder: &Path, name: &str, bytes: &[u8]) -> String {
    let path = folder.join(name);
    std::fs::write(&path, bytes).expect("a test file is written");
    path.to_string_lossy().into_owned()
}

/// Run the program. The answer is its exit code, what it printed, and what
/// it printed as an error.
fn run(arguments: &[&str]) -> (i32, String, String) {
    let output = Command::new(PROGRAM)
        .args(arguments)
        .output()
        .expect("the program runs");
    (
        output.status.code().expect("the program exits"),
        String::from_utf8(output.stdout).expect("the program prints text"),
        String::from_utf8(output.stderr).expect("the program prints text"),
    )
}

fn saved(relative: &str) -> String {
    runs().join(relative).to_string_lossy().into_owned()
}

#[test]
fn verify_run_holds_a_saved_run_and_names_a_changed_one() {
    needs_saved_runs!();
    let (code, out, _) = run(&["verify-run", &saved(CAPSULE_2800)]);
    assert_eq!(code, 0);
    assert_eq!(out, "holds: true\nbroken_link: none\ncause: none\nreceipt: matches\n");
    let (code, out, _) = run(&["verify-run", &saved(CAPSULE_REFUSED)]);
    assert_eq!(code, 0);
    assert_eq!(out, "holds: true\nbroken_link: none\ncause: none\nreceipt: matches\n");

    let folder = folder("verify_run");
    let mut tape = read_tape(CAPSULE_2800);
    tape["outcome"] = json!("refused");
    let changed = write(&folder, "outcome.json", &serde_json::to_vec(&tape).unwrap());
    let (code, out, _) = run(&["verify-run", &changed]);
    assert_eq!(code, 1);
    assert_eq!(out, "holds: false\nbroken_link: 2800\ncause: run_seal\nreceipt: does not match\n");

    let mut tape = read_tape(CAPSULE_2800);
    let text = tape["frames"][41]["frame_hex"].as_str().unwrap().to_string();
    tape["frames"][41]["frame_hex"] = json!(flip_hex_digit(&text, 0));
    let changed = write(&folder, "frame.json", &serde_json::to_vec(&tape).unwrap());
    let (code, out, _) = run(&["verify-run", &changed]);
    assert_eq!(code, 1);
    assert!(out.starts_with("holds: false\nbroken_link: 41\ncause: chain\n"), "{out}");

    let mut tape = read_tape(CAPSULE_2800);
    let text = tape["final_state"]["mach_hex"].as_str().unwrap().to_string();
    tape["final_state"]["mach_hex"] = json!(flip_hex_digit(&text, 7));
    let changed = write(&folder, "state.json", &serde_json::to_vec(&tape).unwrap());
    let (code, out, _) = run(&["verify-run", &changed]);
    assert_eq!(code, 1);
    assert!(out.starts_with("holds: false\nbroken_link: none\ncause: state\n"), "{out}");
}

#[test]
fn verify_run_tells_a_changed_decimal_from_a_broken_seal_by_its_exit_code() {
    needs_saved_runs!();
    let folder = folder("verify_run_decimal");
    let text = String::from_utf8(read_bytes(CAPSULE_2800)).unwrap();
    let changed = write(
        &folder,
        "decimal.json",
        text.replacen("\"mach\":1.1", "\"mach\":9.9", 1).as_bytes(),
    );
    let (code, out, _) = run(&["verify-run", &changed]);
    assert_eq!(code, 3);
    assert_eq!(out, "holds: true\nbroken_link: none\ncause: none\nreceipt: does not match\n");
}

#[test]
fn verify_run_refuses_a_repeated_key_and_a_file_that_is_not_there() {
    needs_saved_runs!();
    let folder = folder("verify_run_refused");
    let text = String::from_utf8(read_bytes(CAPSULE_2800)).unwrap();
    let repeated = write(
        &folder,
        "repeated.json",
        text.replacen("\"dt_s\":", "\"dt_s\":0.05,\"dt_s\":", 1).as_bytes(),
    );
    let (code, out, error) = run(&["verify-run", &repeated]);
    assert_eq!(code, 2);
    assert_eq!(out, "");
    assert_eq!(error, "refused: Physical Violation: the key 'dt_s' is repeated.\n");

    let (code, _, error) = run(&["verify-run", &folder.join("absent.json").to_string_lossy()]);
    assert_eq!(code, 2);
    assert!(error.starts_with("refused: "), "{error}");
    let (code, _, error) = run(&["verify-everything"]);
    assert_eq!(code, 2);
    assert!(error.starts_with("usage:"), "{error}");
    let (code, _, _) = run(&[]);
    assert_eq!(code, 2);
}

#[test]
fn verify_dataset_holds_a_dataset_of_saved_runs_and_names_the_first_break() {
    needs_saved_runs!();
    let folder = folder("verify_dataset");
    let files = dataset_of(&eight_short_runs(), "frames");
    let head = write(&folder, "head.json", &files.head);
    let rows = write(&folder, "rows.jsonl", &files.rows);
    let frames = write(&folder, "frames.bin", files.frames.as_deref().unwrap());

    let (code, out, _) = run(&["verify-dataset", &head, &rows, &frames]);
    assert_eq!(code, 0, "{out}");
    assert_eq!(
        out,
        format!(
            "holds: true\ncause: none\nrow: none\nbroken_link: none\nchains: rebuilt from the frames\nmerkle_root: {}\ndataset_seal: {}\nreceipt: matches\nrows_file: matches the hash in the head\nframes_file: matches the hash in the head\n",
            files.assembled.head.merkle_root, files.assembled.head.dataset_seal
        )
    );
    let (code, out, _) = run(&["verify-dataset", &head, &rows]);
    assert_eq!(code, 0, "{out}");
    assert!(out.contains("chains: not rebuilt: the frames were not given\n"), "{out}");
    assert!(out.contains("frames_file: not given\n"), "{out}");

    let mut changed = files.frames.clone().unwrap();
    changed[64 + 64 * (113 * 3 + 20)] ^= 0x01;
    let bad_frames = write(&folder, "frames_changed.bin", &changed);
    let (code, out, _) = run(&["verify-dataset", &head, &rows, &bad_frames]);
    assert_eq!(code, 1);
    assert!(
        out.starts_with("holds: false\ncause: chain\nrow: 3\nbroken_link: 21\n"),
        "{out}"
    );
    assert!(out.contains("merkle_root: none\ndataset_seal: none\n"), "{out}");

    let mut swapped = split_lines(&files.rows);
    swapped.swap(1, 2);
    let bad_rows = write(&folder, "rows_swapped.jsonl", &join_lines(&swapped));
    let (code, out, _) = run(&["verify-dataset", &head, &bad_rows, &frames]);
    assert_eq!(code, 1);
    assert!(out.starts_with("holds: false\ncause: plan\nrow: 1\n"), "{out}");

    let bad_head = write(
        &folder,
        "head_changed.json",
        &edit_head(&files.head, |head| head["refused"] = json!(1)),
    );
    let (code, out, _) = run(&["verify-dataset", &bad_head, &rows, &frames]);
    assert_eq!(code, 1);
    assert!(out.starts_with("holds: false\ncause: dataset_seal\nrow: none\n"), "{out}");

    // A refusal sentence is sealed by its row's leaf: one given to a row
    // that had none breaks that leaf.
    let detail = write(
        &folder,
        "rows_detail.jsonl",
        &edit_row(&files.rows, 0, |row| row["reject_detail"] = json!("a note")),
    );
    let (code, out, _) = run(&["verify-dataset", &head, &detail, &frames]);
    assert_eq!(code, 1);
    assert!(out.starts_with("holds: false\ncause: leaf\nrow: 0\n"), "{out}");
    assert!(out.ends_with("fault: A row was changed: the leaf of row 0 is not the leaf its number, its request, its outcome, its run_seal and its reject_detail give.\n"), "{out}");

    let (code, _, error) = run(&["verify-dataset", &rows, &head, &frames]);
    assert_eq!(code, 2);
    assert!(error.starts_with("refused: The head was not read."), "{error}");
}

/// The head of `rows`, with its rows_sha256 and its receipt worked out
/// again, as a forger who changed the rows would write it.
fn head_for(head: &[u8], rows: &[u8]) -> Vec<u8> {
    let mut read = genesis_seal::dataset::Head::read(head).unwrap();
    read.rows_sha256 = Some(genesis_seal::sealed_form::digest_hex(rows));
    read.wire().into_bytes()
}

#[test]
fn verify_dataset_names_the_row_of_a_dropped_or_repeated_row_and_of_a_decimal_that_is_not_its_hex() {
    needs_saved_runs!();
    let folder = folder("verify_dataset_rows");
    let files = dataset_of(&eight_short_runs(), "none");
    let lines = split_lines(&files.rows);
    let ask = |name: &str, rows: Vec<u8>| -> (i32, String) {
        let head = write(&folder, &format!("{name}_head.json"), &head_for(&files.head, &rows));
        let rows = write(&folder, &format!("{name}_rows.jsonl"), &rows);
        let (code, out, _) = run(&["verify-dataset", &head, &rows]);
        (code, out)
    };

    // The last row dropped.
    let (code, out) = ask("last_dropped", join_lines(&lines[..7]));
    assert_eq!(code, 1);
    assert!(out.starts_with("holds: false\ncause: plan\nrow: 7\n"), "{out}");
    assert!(out.ends_with("fault: A row was dropped: every row read stands in its own place, and row 7 is the first row missing, after the last row read. Rows read: 7; rows in the plan: 8.\n"), "{out}");

    // Row 2 dropped.
    let mut dropped = lines.clone();
    dropped.remove(2);
    let (code, out) = ask("row_2_dropped", join_lines(&dropped));
    assert_eq!(code, 1);
    assert!(out.starts_with("holds: false\ncause: plan\nrow: 2\n"), "{out}");
    assert!(out.contains("fault: A row was dropped: row 2 is the first row missing, and the row in its place is row 3. Rows read: 7; rows in the plan: 8.\n"), "{out}");

    // The last row repeated at the end.
    let mut repeated = lines.clone();
    repeated.push(lines[7].clone());
    let (code, out) = ask("last_repeated", join_lines(&repeated));
    assert_eq!(code, 1);
    assert!(out.starts_with("holds: false\ncause: plan\nrow: 8\n"), "{out}");
    assert!(out.contains("fault: A row is repeated: row 7 stands in its own place and again in place 8, after the plan's last row. Rows read: 9; rows in the plan: 8.\n"), "{out}");

    // Row 6 copied over row 4, the count unchanged.
    let mut copied = lines.clone();
    copied[4] = lines[6].clone();
    let (code, out) = ask("copied", join_lines(&copied));
    assert_eq!(code, 1);
    assert!(out.starts_with("holds: false\ncause: plan\nrow: 4\n"), "{out}");
    assert!(out.contains("fault: A row is repeated: row 6 stands in its own place and also in the place of row 4, so row 4 is missing.\n"), "{out}");

    // A decimal of a final state changed, with the head's rows_sha256 and
    // receipt worked out again: every seal holds, the decimal is named, and
    // the exit code says the bytes are not the engine's.
    let state = |row: &Value| row["final_state"].as_object().unwrap().clone();
    let first: Value = serde_json::from_slice(&lines[3]).unwrap();
    let key = state(&first)
        .iter()
        .find(|(key, value)| value.is_number() && !key.ends_with("_hex"))
        .map(|(key, _)| key.clone())
        .unwrap();
    let rows = edit_row(&files.rows, 3, |row| {
        let read = row["final_state"][&key].as_f64().unwrap();
        row["final_state"][&key] = json!(read * 2.0 + 1.0);
    });
    let (code, out) = ask("decimal", rows);
    assert_eq!(code, 3, "{out}");
    assert!(out.starts_with("holds: true\ncause: none\nrow: 3\n"), "{out}");
    assert!(out.contains("rows_file: matches the hash in the head\n"), "{out}");
    assert!(out.contains(&format!("fault: A decimal was changed in row 3: final_state.{key} reads ")), "{out}");
    // A decimal written as a string, or as null, is not its hex either.
    for (name, written) in [("string", json!("1.0")), ("null", Value::Null)] {
        let rows = edit_row(&files.rows, 5, |row| row["final_state"][&key] = written.clone());
        let (code, out) = ask(name, rows);
        if code == 2 {
            // The row reader refuses a decimal of the wrong type by name.
            continue;
        }
        assert_eq!(code, 3, "{name}: {out}");
        assert!(out.starts_with("holds: true\ncause: none\nrow: 5\n"), "{name}: {out}");
    }
}

#[test]
fn row_proof_writes_a_proof_that_verify_proof_holds() {
    needs_saved_runs!();
    let folder = folder("row_proof");
    let files = dataset_of(&eight_short_runs()[..7], "frames");
    let head = write(&folder, "head.json", &files.head);
    let rows = write(&folder, "rows.jsonl", &files.rows);
    let frames = write(&folder, "frames.bin", files.frames.as_deref().unwrap());
    let seal = &files.assembled.head.dataset_seal;

    for row in ["0", "4", "6"] {
        let (code, out, _) = run(&["row-proof", &head, &rows, row, &frames]);
        assert_eq!(code, 0);
        let proof = write(&folder, &format!("proof_{row}.json"), out.as_bytes());
        let (code, out, _) = run(&["verify-proof", &proof]);
        assert_eq!(code, 0, "{out}");
        // check-proof, the earlier name, gives the same answer.
        assert_eq!(run(&["check-proof", &proof]), (code, out.clone(), String::new()));
        assert_eq!(
            out,
            format!(
                "holds: true\ncause: none\nrow: {row}\nbroken_link: none\nchain: rebuilt from the frames\ndataset_seal: {seal}\n"
            )
        );
    }
    let (code, out, _) = run(&["row-proof", &head, &rows, "5"]);
    assert_eq!(code, 0);
    let proof = write(&folder, "proof_5_without_frames.json", out.as_bytes());
    let (code, out, _) = run(&["verify-proof", &proof]);
    assert_eq!(code, 0);
    assert!(out.contains("chain: not rebuilt: the proof carries no frames\n"), "{out}");

    // The proof of row 4 with the row of row 5 put in its place.
    let (_, four, _) = run(&["row-proof", &head, &rows, "4"]);
    let (_, five, _) = run(&["row-proof", &head, &rows, "5"]);
    let mut forged: Value = serde_json::from_str(&four).unwrap();
    forged["row"] = serde_json::from_str::<Value>(&five).unwrap()["row"].clone();
    let forged = write(&folder, "proof_forged.json", &serde_json::to_vec(&forged).unwrap());
    let (code, out, _) = run(&["verify-proof", &forged]);
    assert_eq!(code, 1);
    assert!(out.starts_with("holds: false\ncause: root\nrow: 5\n"), "{out}");

    let (code, _, error) = run(&["row-proof", &head, &rows, "7"]);
    assert_eq!(code, 2);
    assert_eq!(error, "refused: The dataset has 7 rows and no row 7.\n");
    let (code, _, error) = run(&["row-proof", &head, &rows, "-1"]);
    assert_eq!(code, 2);
    assert_eq!(error, "refused: '-1' is not a row number.\n");

    // No proof is written from a dataset that does not hold.
    let bad_rows = write(
        &folder,
        "rows_changed.jsonl",
        &edit_row(&files.rows, 2, |row| row["reject"] = json!("X")),
    );
    let (code, out, _) = run(&["row-proof", &head, &bad_rows, "0"]);
    assert_eq!(code, 1);
    assert!(out.starts_with("holds: false\ncause: run_seal\nrow: 2\n"), "{out}");
}

#[test]
fn expand_prints_one_row_of_a_plan() {
    needs_saved_runs!();
    let folder = folder("expand");
    let tapes = eight_short_runs();
    let listed = list_request(&tapes, "frames", "file");
    let request = write(&folder, "list.json", &serde_json::to_vec(listed.value()).unwrap());
    let (code, out, _) = run(&["expand", &request, "6"]);
    assert_eq!(code, 0);
    let row: Value = serde_json::from_str(&out).unwrap();
    assert_eq!(
        genesis_seal::sealed_form::sealed_form(&row),
        genesis_seal::sealed_form::sealed_form(&tapes[6]["inputs"])
    );
    assert!(out.starts_with("{\"card\":\"satellite_lageos1\",\"domain\":\"aerospace\",\"dt_s\":10.0,"), "{out}");
    let (code, _, error) = run(&["expand", &request, "8"]);
    assert_eq!(code, 2);
    assert_eq!(error, "refused: Physical Violation: the plan has 8 rows and no row 8.\n");

    let sample = br#"{"domain":"energy","card":"thermal","plan":{"kind":"sample","base":{"initial":{},"dt_s":0.1,"segments":[{"steps":10,"load":{"ambient_k":293.15}}],"record_every":1},"rows":100000,"generator":"splitmix64_counter_v1","seed_hex":"0000000000000000","draws":[{"path":"initial.temperature_k","uniform":{"low":0.0,"high":1.0}},{"path":"segments[0].load.heat_power_w","choice":[10,20,30]}]},"store":"none","return":"file"}"#;
    let request = write(&folder, "sample.json", sample);
    let (code, out, _) = run(&["expand", &request, "0"]);
    assert_eq!(code, 0);
    // The first published vector: seed 0, row 0, draw 0, over [0, 1].
    let row: Value = serde_json::from_str(&out).unwrap();
    assert_eq!(
        format!("{:016x}", row["initial"]["temperature_k"].as_f64().unwrap().to_bits()),
        "3fe4e0dba5e9a32f"
    );
    let (code, last, _) = run(&["expand", &request, "99999"]);
    assert_eq!(code, 0);
    assert_ne!(last, out);

    let generator = String::from_utf8(sample.to_vec()).unwrap().replace("splitmix64_counter_v1", "mt19937");
    let request = write(&folder, "generator.json", generator.as_bytes());
    let (code, _, error) = run(&["expand", &request, "0"]);
    assert_eq!(code, 2);
    assert_eq!(
        error,
        "refused: Physical Violation: 'mt19937' is not a generator. The generator is splitmix64_counter_v1.\n"
    );
}

#[test]
fn verify_dataset_holds_the_three_files_the_writer_wrote() {
    needs_saved_runs!();
    use genesis_seal::dataset::DatasetWriter;
    let folder = folder("writer_files");
    let tapes = eight_short_runs();
    let request = list_request(&tapes, "frames", "file");
    let engine_version = tapes[0]["engine_version"].as_str().unwrap().to_string();
    let frames_path = folder.join("frames.bin");
    let rows_path = folder.join("rows.jsonl");
    let frames_file = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(true)
        .open(&frames_path)
        .unwrap();
    let rows_file = std::fs::File::create(&rows_path).unwrap();
    let mut writer = DatasetWriter::new(&request, &engine_version, rows_file, Some(frames_file)).unwrap();
    for (index, tape) in tapes.iter().enumerate() {
        writer
            .push(index as u64, &genesis_seal::dataset::RowResult::Run(tape.clone()))
            .unwrap();
    }
    let done = writer.finish().unwrap();
    let head = write(&folder, "head.json", done.head.wire().as_bytes());
    let (code, out, _) = run(&[
        "verify-dataset",
        &head,
        &rows_path.to_string_lossy(),
        &frames_path.to_string_lossy(),
    ]);
    assert_eq!(code, 0, "{out}");
    assert!(out.contains("frames_file: matches the hash in the head\n"), "{out}");
}

/// An answer of `validate_series` made by the rules of the crate alone, with its receipt.
fn validation_answer() -> Value {
    use genesis_seal::validation::{SealParts, bits_hex, measurements_sha256, statistics, validation_seal};
    let measurements = json!([{"step": 1, "value": 3.9}, {"step": 2, "value": 3.8}]);
    let model = [3.9_f64, 3.7];
    let hash = measurements_sha256(&measurements);
    let stats = statistics(&[1, 2], &model, &[3.9, 3.8]).unwrap().unwrap();
    let hexes: Vec<String> = model.iter().map(|value| bits_hex(*value)).collect();
    let run_seal = "ab".repeat(32);
    let seal = validation_seal(&SealParts {
        run_seal: &run_seal,
        measurements_sha256: &hash,
        quantity: "terminal_v",
        component: None,
        outcome: "completed",
        steps_landed: 2,
        reject: None,
        compared: 2,
        not_reached: 0,
        statistics: Some(&stats),
        model_values_hex: &hexes,
        commitment: None,
    });
    json!({
        "inputs": {"run": {}, "quantity": "terminal_v", "measurements": measurements},
        "outcome": "completed",
        "steps_landed": 2,
        "reject": null,
        "run_seal": run_seal,
        "compared": 2,
        "not_reached": 0,
        "rms_error_hex": stats.rms_error_hex(),
        "mean_signed_error_hex": stats.mean_signed_error_hex(),
        "max_abs_error_hex": stats.max_abs_error_hex(),
        "max_abs_error_step": stats.max_abs_error_step,
        "model_values_hex": hexes,
        "measurements_sha256": hash,
        "validation_seal": seal,
    })
}

#[test]
fn verify_validation_holds_an_answer_and_names_each_change_to_it() {
    let folder = folder("verify_validation");
    let genuine = validation_answer();
    let wire = |value: &Value| {
        genesis_seal::receipt::with_receipt(&serde_json::to_string(value).unwrap())
    };
    let path = write(&folder, "genuine.json", wire(&genuine).as_bytes());
    let (code, out, _) = run(&["verify-validation", &path]);
    assert_eq!(code, 0, "{out}");
    assert!(out.starts_with("holds: true\ncause: none\ncompared: 2\nnot_reached: 0\n"), "{out}");
    assert!(out.ends_with("receipt: matches\n"), "{out}");

    // A changed statistic, with the receipt made again so that only the seal speaks.
    for (name, edit, cause) in [
        ("statistic", "mean_signed_error_hex", "statistics"),
        ("model_value", "model_values_hex", "model_values"),
        ("measurement_hash", "measurements_sha256", "measurements"),
    ] {
        let mut changed = genuine.clone();
        match edit {
            "model_values_hex" => changed[edit][1] = json!("4000000000000000"),
            "measurements_sha256" => changed[edit] = json!("cd".repeat(32)),
            _ => changed[edit] = json!("3fe0000000000000"),
        }
        let path = write(&folder, &format!("{name}.json"), wire(&changed).as_bytes());
        let (code, out, _) = run(&["verify-validation", &path]);
        assert_eq!(code, 1, "{name}: {out}");
        assert!(out.starts_with(&format!("holds: false\ncause: {cause}\n")), "{name}: {out}");
    }

    // The same answer with a byte changed after the receipt: the seals hold, the bytes differ.
    let mut text = wire(&genuine);
    text = text.replace("\"compared\":2", "\"compared\": 2");
    let path = write(&folder, "respaced.json", text.as_bytes());
    let (code, out, _) = run(&["verify-validation", &path]);
    assert_eq!(code, 3, "{out}");
    assert!(out.contains("holds: true\n") && out.ends_with("receipt: does not match\n"), "{out}");

    let path = write(&folder, "refused.json", b"{\"inputs\":{}}");
    let (code, _, err) = run(&["verify-validation", &path]);
    assert_eq!(code, 2);
    assert!(err.starts_with("refused: "), "{err}");
}

/// The answer of `validation_answer` with the commitment its measurements
/// open under `salt`, sealed with it, or, given `opened`, with other data in
/// place of the committed ones and every other part made again from them.
fn committed_answer(salt: &str, opened: Option<Value>) -> Value {
    use genesis_seal::commitment::{Commitment, commitment_sha256};
    use genesis_seal::validation::{SealParts, bits_hex, measurements_sha256, statistics, validation_seal};
    let committed = json!([{"step": 1, "value": 3.9}, {"step": 2, "value": 3.8}]);
    let sha256 = commitment_sha256("terminal_v", &Value::Null, &committed, salt);
    let commitment = Commitment { sha256, salt: salt.to_string() };
    let measurements = opened.unwrap_or(committed);
    let measured: Vec<f64> = measurements.as_array().unwrap().iter().map(|item| item["value"].as_f64().unwrap()).collect();
    let steps: Vec<u64> = measurements.as_array().unwrap().iter().map(|item| item["step"].as_u64().unwrap()).collect();
    let model = [3.9_f64, 3.7];
    let hash = measurements_sha256(&measurements);
    let stats = statistics(&steps, &model, &measured).unwrap().unwrap();
    let hexes: Vec<String> = model.iter().map(|value| bits_hex(*value)).collect();
    let mut answer = validation_answer();
    answer["inputs"]["measurements"] = measurements;
    answer["inputs"]["commitment"] = json!({"sha256": commitment.sha256, "salt": salt});
    answer["commitment_opened"] = json!(genesis_seal::commitment::OPENED);
    answer["measurements_sha256"] = json!(hash);
    answer["rms_error_hex"] = json!(stats.rms_error_hex());
    answer["mean_signed_error_hex"] = json!(stats.mean_signed_error_hex());
    answer["max_abs_error_hex"] = json!(stats.max_abs_error_hex());
    answer["max_abs_error_step"] = json!(stats.max_abs_error_step);
    answer["validation_seal"] = json!(validation_seal(&SealParts {
        run_seal: &"ab".repeat(32),
        measurements_sha256: &hash,
        quantity: "terminal_v",
        component: None,
        outcome: "completed",
        steps_landed: 2,
        reject: None,
        compared: 2,
        not_reached: 0,
        statistics: Some(&stats),
        model_values_hex: &hexes,
        commitment: Some(&commitment),
    }));
    answer
}

#[test]
fn verify_validation_opens_a_commitment_and_names_data_that_do_not_open_it() {
    let folder = folder("verify_validation_commitment");
    let salt = "000102030405060708090a0b0c0d0e0f";
    let wire = |value: &Value| genesis_seal::receipt::with_receipt(&serde_json::to_string(value).unwrap());
    let genuine = committed_answer(salt, None);
    let committed = genuine["inputs"]["commitment"]["sha256"].as_str().unwrap().to_string();
    let path = write(&folder, "genuine.json", wire(&genuine).as_bytes());
    let (code, out, _) = run(&["verify-validation", &path]);
    assert_eq!(code, 0, "{out}");
    assert!(out.starts_with("holds: true\ncause: none\n"), "{out}");
    assert!(out.contains(&format!("commitment_sha256: {committed}\ncommitment: opens\nreceipt: matches\n")), "{out}");

    // One digit of one measurement, every other part made again from it,
    // the receipt too: the commitment alone speaks.
    let one_digit = committed_answer(salt, Some(json!([{"step": 1, "value": 3.8}, {"step": 2, "value": 3.8}])));
    assert_eq!(one_digit["inputs"]["commitment"]["sha256"], committed.as_str());
    let path = write(&folder, "one_digit.json", wire(&one_digit).as_bytes());
    let (code, out, _) = run(&["verify-validation", &path]);
    assert_eq!(code, 1, "{out}");
    assert!(out.starts_with("holds: false\ncause: commitment\n"), "{out}");
    assert!(out.contains(&format!("commitment_sha256: {committed}\ncommitment: does not open: ")), "{out}");

    // Another salt under the sha256 committed.
    let mut wrong_salt = committed_answer("ffeeddccbbaa99887766554433221100", None);
    wrong_salt["inputs"]["commitment"]["sha256"] = json!(committed);
    let path = write(&folder, "wrong_salt.json", wire(&wrong_salt).as_bytes());
    let (code, out, _) = run(&["verify-validation", &path]);
    assert_eq!(code, 1, "{out}");
    assert!(out.starts_with("holds: false\ncause: commitment\n"), "{out}");

    // A salt of fewer than 16 bytes is not read.
    let mut short = genuine.clone();
    short["inputs"]["commitment"]["salt"] = json!("00010203");
    let path = write(&folder, "short.json", wire(&short).as_bytes());
    let (code, _, err) = run(&["verify-validation", &path]);
    assert_eq!(code, 2);
    assert!(err.ends_with("(CommitmentSaltTooShort).\n"), "{err}");

    // Without its receipt the note says the seal covers the commitment.
    let path = write(&folder, "bare.json", serde_json::to_string(&genuine).unwrap().as_bytes());
    let (code, out, _) = run(&["verify-validation", &path]);
    assert_eq!(code, 0, "{out}");
    assert!(out.contains("the component, the commitment, the outcome"), "{out}");

    // An answer without a commitment prints no commitment line.
    let path = write(&folder, "plain.json", wire(&validation_answer()).as_bytes());
    let (code, out, _) = run(&["verify-validation", &path]);
    assert_eq!(code, 0, "{out}");
    assert!(!out.contains("commitment"), "{out}");

    // The red team of 0.13.0: commitment_opened written into an answer that
    // opened no commitment, the receipt made again, held; and left out of
    // one that did. Each is named by cause, exit 1.
    let mut claimed = validation_answer();
    claimed["commitment_opened"] = json!(genesis_seal::commitment::OPENED);
    let path = write(&folder, "claimed.json", wire(&claimed).as_bytes());
    let (code, out, _) = run(&["verify-validation", &path]);
    assert_eq!(code, 1, "{out}");
    assert!(out.starts_with("holds: false\ncause: commitment_opened\n"), "{out}");
    assert!(out.contains("commitment_opened: said, and the inputs carry no commitment\n"), "{out}");
    let mut unsaid = genuine.clone();
    unsaid.as_object_mut().unwrap().remove("commitment_opened");
    let path = write(&folder, "unsaid.json", wire(&unsaid).as_bytes());
    let (code, out, _) = run(&["verify-validation", &path]);
    assert_eq!(code, 1, "{out}");
    assert!(out.starts_with("holds: false\ncause: commitment_opened\n"), "{out}");
    assert!(out.contains("commitment_opened: not said, and the inputs carry a commitment\n"), "{out}");
}

/// A genuine answer with every field the gateway writes: the decimal copies, the
/// engine version, the reject detail and the words that travel with it.
fn full_validation_answer() -> Value {
    let mut answer = validation_answer();
    let object = answer.as_object_mut().unwrap();
    object.insert("engine_version".into(), json!("0.2.1"));
    object.insert("reject_detail".into(), Value::Null);
    object.insert("rms_error".into(), json!(0.0707));
    object.insert("mean_signed_error".into(), json!(0.05));
    object.insert("max_abs_error".into(), json!(0.1));
    object.insert("does_not_show".into(), json!("The numbers say how far this run was."));
    answer
}

/// Every field of the answer, named by its path: a top-level key, or a key of `inputs`.
fn field_paths(answer: &Value) -> Vec<Vec<String>> {
    let mut paths = Vec::new();
    for (key, value) in answer.as_object().unwrap() {
        if key == "inputs" {
            for inner in value.as_object().unwrap().keys() {
                paths.push(vec!["inputs".to_string(), inner.clone()]);
            }
        } else {
            paths.push(vec![key.clone()]);
        }
    }
    paths
}

/// The value changed to another of the same kind, the smallest edit there is.
fn edited(value: &Value) -> Value {
    match value {
        Value::Null => json!("FirstLawBreach"),
        Value::Bool(flag) => json!(!flag),
        Value::Number(number) if number.is_u64() => json!(number.as_u64().unwrap() + 1),
        Value::Number(number) => json!(number.as_f64().unwrap() + 0.25),
        Value::String(text) if !text.is_empty() && text.bytes().all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase()) => {
            let mut bytes = text.clone().into_bytes();
            bytes[0] = if bytes[0] == b'0' { b'1' } else { b'0' };
            json!(String::from_utf8(bytes).unwrap())
        }
        Value::String(text) => json!(format!("{text}x")),
        Value::Array(items) => {
            let mut items = items.clone();
            match items.first_mut() {
                Some(first) => *first = edited(first),
                None => items.push(json!(1)),
            }
            Value::Array(items)
        }
        Value::Object(map) => {
            let mut map = map.clone();
            map.insert("edited".into(), json!(1));
            Value::Object(map)
        }
    }
}

/// The fields only the receipt covers: the decimal copies, and the words and
/// versions the seal does not take in.
fn only_the_receipt_covers(path: &[String]) -> bool {
    matches!(
        path.join(".").as_str(),
        "rms_error" | "mean_signed_error" | "max_abs_error" | "engine_version" | "reject_detail" | "does_not_show" | "inputs.run"
    )
}

#[test]
fn every_field_of_an_answer_changed_in_turn_is_named_with_the_receipt_and_without_it() {
    let folder = folder("validation_sweep");
    let genuine = full_validation_answer();
    let with_receipt = |value: &Value| genesis_seal::receipt::with_receipt(&serde_json::to_string(value).unwrap());
    let path = write(&folder, "genuine.json", with_receipt(&genuine).as_bytes());
    assert_eq!(run(&["verify-validation", &path]).0, 0);
    let mut swept = 0;
    for field in field_paths(&genuine) {
        let mut changed = genuine.clone();
        let slot = match field.as_slice() {
            [key] => &mut changed[key.as_str()],
            [outer, inner] => &mut changed[outer.as_str()][inner.as_str()],
            _ => unreachable!(),
        };
        *slot = edited(slot);
        let name = field.join(".");
        let bytes = serde_json::to_string(&changed).unwrap();
        // The receipt of the genuine answer kept on the edited bytes: the receipt names every edit.
        let genuine_text = with_receipt(&genuine);
        let tail = &genuine_text[genuine_text.rfind(",\"receipt_sha256\"").unwrap()..];
        let old_receipt = format!("{}{}", &bytes[..bytes.len() - 1], tail);
        let path = write(&folder, "kept_old.json", old_receipt.as_bytes());
        let (code, out, _) = run(&["verify-validation", &path]);
        assert_ne!(code, 0, "{name} edited, the old receipt kept: {out}");
        // The receipt removed: every field the seal covers is seen, and the program says what it cannot see.
        let path = write(&folder, "removed.json", bytes.as_bytes());
        let (code, out, _) = run(&["verify-validation", &path]);
        if only_the_receipt_covers(&field) {
            assert_eq!(code, 0, "{name}: {out}");
            assert!(out.contains("receipt: absent\n"), "{name}: {out}");
            assert!(out.contains("note: the receipt is absent"), "{name}: {out}");
        } else {
            assert_ne!(code, 0, "{name} edited, receipt removed: {out}");
        }
        swept += 1;
    }
    assert!(swept >= 17, "{swept}");
}

#[test]
fn an_answer_without_a_receipt_says_in_words_what_only_the_receipt_covers() {
    let folder = folder("validation_absent");
    let path = write(&folder, "bare.json", serde_json::to_string(&full_validation_answer()).unwrap().as_bytes());
    let (code, out, _) = run(&["verify-validation", &path]);
    assert_eq!(code, 0, "{out}");
    assert!(out.contains("receipt: absent\nnote: the receipt is absent, so the bytes of this answer are not covered."), "{out}");
    assert!(out.contains("decimal copies"), "{out}");
    let path = write(&folder, "with.json", genesis_seal::receipt::with_receipt(&serde_json::to_string(&full_validation_answer()).unwrap()).as_bytes());
    assert!(!run(&["verify-validation", &path]).1.contains("note:"));
}

/// The program, with a limit on how long it may run. A hang fails the test.
fn run_within(arguments: &[&str], seconds: u64) -> (i32, String, String) {
    let mut child = Command::new(PROGRAM)
        .args(arguments)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("the program runs");
    let start = std::time::Instant::now();
    loop {
        if child.try_wait().unwrap().is_some() {
            break;
        }
        if start.elapsed().as_secs() >= seconds {
            let _ = child.kill();
            panic!("the program did not end within {seconds} s: {arguments:?}");
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    let output = child.wait_with_output().unwrap();
    (
        output.status.code().unwrap(),
        String::from_utf8(output.stdout).unwrap(),
        String::from_utf8(output.stderr).unwrap(),
    )
}

#[test]
fn a_named_pipe_a_device_and_a_link_are_refused_by_name_with_exit_2_and_no_wait() {
    let folder = folder("not_regular");
    let head = write(&folder, "head.json", b"{}");
    let rows = write(&folder, "rows.jsonl", b"");
    let fifo = folder.join("frames.fifo");
    let _ = std::fs::remove_file(&fifo);
    let text = std::ffi::CString::new(fifo.to_str().unwrap()).unwrap();
    assert_eq!(unsafe { libc_mkfifo(text.as_ptr(), 0o600) }, 0);
    let fifo = fifo.to_string_lossy().into_owned();
    let (code, out, err) = run_within(&["verify-dataset", &head, &rows, &fifo], 10);
    assert_eq!((code, out.as_str()), (2, ""));
    assert!(err.contains("is a named pipe") && err.starts_with("refused: "), "{err}");
    // As the head, and as the answer to check.
    for command in ["verify-run", "verify-validation", "verify-proof", "check-proof"] {
        let (code, _, err) = run_within(&[command, &fifo], 10);
        assert_eq!(code, 2, "{command}");
        assert!(err.contains("a named pipe"), "{command}: {err}");
    }
    let link = folder.join("frames.link");
    let _ = std::fs::remove_file(&link);
    std::os::unix::fs::symlink("/dev/zero", &link).unwrap();
    let link = link.to_string_lossy().into_owned();
    let (code, _, err) = run_within(&["verify-dataset", &head, &rows, &link], 10);
    assert_eq!(code, 2);
    assert!(err.contains("symbolic link"), "{err}");
    let (code, _, err) = run_within(&["verify-dataset", &head, &rows, "/dev/zero"], 10);
    assert_eq!(code, 2);
    assert!(err.contains("a device"), "{err}");
}

unsafe extern "C" {
    #[link_name = "mkfifo"]
    fn libc_mkfifo(path: *const std::ffi::c_char, mode: u32) -> i32;
}

/// A chain of two runs of the cold plate, the second continued from the
/// first through `from`, as the engine 0.7.0 with chained proofs answered
/// them on its local connection (`tests/fixtures/chain_thermal.json`).
fn chain_fixture() -> Value {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/chain_thermal.json");
    serde_json::from_slice(&std::fs::read(path).expect("the chain fixture")).expect("JSON")
}

#[test]
fn verify_chain_holds_a_chain_and_names_each_break() {
    let folder = folder("verify_chain");
    let chain = chain_fixture();
    let whole = write(&folder, "chain.json", &serde_json::to_vec(&chain).unwrap());
    let (code, out, _) = run(&["verify-chain", &whole]);
    assert_eq!(code, 0, "{out}");
    assert_eq!(
        out,
        "holds: true\nlinks: 2\nbroken_link: none\ncause: none\nchain_seal: ebe580913f63594dbaa1cd6dd2e2da63774e4c1ca27bcca1b65e73b100f579df\n"
    );
    let flip = |text: &str| {
        let last = if text.ends_with('0') { '1' } else { '0' };
        format!("{}{last}", &text[..text.len() - 1])
    };
    let cases: Vec<(&str, Box<dyn Fn(&mut Value)>, &str)> = vec![
        ("frame", Box::new(|c: &mut Value| { let t = c["links"][0]["frames"][0]["frame_hex"].as_str().unwrap().to_string(); c["links"][0]["frames"][0]["frame_hex"] = json!(flip(&t)); }), "broken_link: 0\ncause: chain"),
        ("state", Box::new(|c: &mut Value| { let t = c["links"][0]["final_state"]["temperature_k_hex"].as_str().unwrap().to_string(); c["links"][0]["final_state"]["temperature_k_hex"] = json!(flip(&t)); }), "broken_link: 0\ncause: state"),
        ("upstream", Box::new(|c: &mut Value| { let t = c["links"][1]["chain"]["upstream_run_seal"].as_str().unwrap().to_string(); c["links"][1]["chain"]["upstream_run_seal"] = json!(flip(&t)); }), "broken_link: 1\ncause: upstream"),
        ("handed", Box::new(|c: &mut Value| { let t = c["links"][1]["chain"]["handed"][0]["value_hex"].as_str().unwrap().to_string(); c["links"][1]["chain"]["handed"][0]["value_hex"] = json!(flip(&t)); }), "broken_link: 1\ncause: handed"),
        ("start", Box::new(|c: &mut Value| { let x = c["links"][1]["inputs"]["initial"]["temperature_k"].as_f64().unwrap(); c["links"][1]["inputs"]["initial"]["temperature_k"] = json!(x.next_up()); }), "broken_link: 1\ncause: chain"),
        ("chain seal", Box::new(|c: &mut Value| { let t = c["links"][1]["chain"]["chain_seal"].as_str().unwrap().to_string(); c["links"][1]["chain"]["chain_seal"] = json!(flip(&t)); }), "broken_link: 1\ncause: chain_seal"),
        ("kind", Box::new(|c: &mut Value| { c["links"][1]["chain"]["kind"] = json!("coupling"); c["links"][1]["chain"]["coupling"] = json!("x"); }), "broken_link: 1\ncause: chain_seal"),
    ];
    for (what, change, said) in cases {
        let mut broken = chain.clone();
        change(&mut broken);
        let path = write(&folder, "broken.json", &serde_json::to_vec(&broken).unwrap());
        let (code, out, _) = run(&["verify-chain", &path]);
        assert_eq!(code, 1, "{what}: {out}");
        assert!(out.starts_with("holds: false\nlinks: 2\n") && out.contains(said), "{what}: {out}");
    }
    // One link is not a chain, and the first link carries no chain.
    let mut one = chain.clone();
    one["links"].as_array_mut().unwrap().pop();
    let path = write(&folder, "one.json", &serde_json::to_vec(&one).unwrap());
    assert_eq!(run(&["verify-chain", &path]).0, 2);
    let mut reversed = chain.clone();
    reversed["links"].as_array_mut().unwrap().reverse();
    let path = write(&folder, "reversed.json", &serde_json::to_vec(&reversed).unwrap());
    assert_eq!(run(&["verify-chain", &path]).0, 2);
}
