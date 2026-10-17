//! Runs the engine made and saved, checked here with no engine, and then
//! changed one thing at a time.

mod support;

use genesis_seal::receipt::{ReceiptCheck, check_receipt};
use genesis_seal::refusal::Refusal;
use genesis_seal::run::{TapeCause, TapeReport, check_tape, check_tape_text, frames_recorded};
use serde_json::{Value, json};
use support::*;

/// A saved run of the company's carries no signed load, so a run that holds
/// lists none.
fn holds() -> TapeReport {
    TapeReport::holds(Vec::new())
}

fn broken(cause: TapeCause, link: Option<u64>) -> TapeReport {
    TapeReport::broken(cause, link)
}

fn check(tape: &Value) -> TapeReport {
    check_tape(tape).unwrap_or_else(|refusal| panic!("the tape was refused: {refusal}"))
}

fn frame_count(tape: &Value) -> u64 {
    tape["frames"].as_array().unwrap().len() as u64
}

#[test]
fn every_saved_run_holds_and_its_receipt_matches_its_bytes() {
    needs_saved_runs!();
    let mut checked = 0;
    for (folder, expected) in [(CAPSULE, 3), (GENERATOR, 2), (SATELLITE, 56)] {
        let tapes = tapes_in(folder);
        assert_eq!(tapes.len(), expected, "{folder}");
        for (name, bytes, tape) in tapes {
            assert_eq!(check_tape_text(&bytes).unwrap(), holds(), "{name}");
            assert_eq!(check(&tape), holds(), "{name}");
            assert_eq!(check_receipt(&bytes), ReceiptCheck::Matches, "{name}");
            checked += 1;
        }
    }
    assert_eq!(checked, 61);
}

#[test]
fn every_saved_run_recorded_the_count_of_frames_the_rule_gives() {
    needs_saved_runs!();
    for folder in [CAPSULE, GENERATOR, SATELLITE] {
        for (name, _, tape) in tapes_in(folder) {
            let landed = tape["steps_landed"].as_u64().unwrap();
            let every = tape["inputs"]["record_every"].as_u64().unwrap();
            assert_eq!(frames_recorded(landed, every), Some(frame_count(&tape)), "{name}");
        }
    }
}

#[test]
fn the_first_link_of_the_named_run_is_in_its_first_frame() {
    needs_saved_runs!();
    let tape = read_tape(CAPSULE_2800);
    let link_0 = genesis_seal::run::link_0(&tape["inputs"]);
    let first = tape["frames"][0]["frame_hex"].as_str().unwrap();
    assert_eq!(&first[96..], hex::encode(&link_0[..16]));
    assert_eq!(
        tape["run_seal"],
        json!("44f398155039dcd1175f98bb6fc6c09c34cf94a261fa977a97a54c1653d2b93e")
    );
}

#[test]
fn a_changed_frame_byte_is_a_chain_failure_at_that_frame() {
    needs_saved_runs!();
    let tape = read_tape(CAPSULE_2800);
    for (frame, digit) in [(0, 0), (1, 17), (1400, 95), (2799, 127)] {
        let mut changed = tape.clone();
        let text = changed["frames"][frame]["frame_hex"].as_str().unwrap().to_string();
        changed["frames"][frame]["frame_hex"] = json!(flip_hex_digit(&text, digit));
        assert_eq!(
            check(&changed),
            broken(TapeCause::Chain, Some(frame as u64)),
            "frame {frame} digit {digit}"
        );
    }
    // A stored link that is not the hash of its frame fails at that frame too.
    let mut changed = tape.clone();
    let text = changed["frames"][7]["link"].as_str().unwrap().to_string();
    changed["frames"][7]["link"] = json!(flip_hex_digit(&text, 3));
    assert_eq!(check(&changed), broken(TapeCause::Chain, Some(7)));
    // A frame that is not 128 hex characters is that index, not a refusal.
    let mut changed = tape;
    changed["frames"][3]["frame_hex"] = json!("zz");
    assert_eq!(check(&changed), broken(TapeCause::Chain, Some(3)));
}

#[test]
fn a_removed_frame_is_a_chain_failure_where_the_chain_no_longer_joins() {
    needs_saved_runs!();
    let tape = read_tape(CAPSULE_2800);
    for removed in [0, 5, 2798] {
        let mut changed = tape.clone();
        changed["frames"].as_array_mut().unwrap().remove(removed);
        assert_eq!(
            check(&changed),
            broken(TapeCause::Chain, Some(removed as u64)),
            "frame {removed}"
        );
    }
    // With the last frame gone every frame left still joins, and the run
    // seal no longer follows from the last link.
    let mut changed = tape;
    changed["frames"].as_array_mut().unwrap().pop();
    assert_eq!(check(&changed), broken(TapeCause::RunSeal, Some(2799)));
}

#[test]
fn two_swapped_frames_are_a_chain_failure_at_the_first_of_them() {
    needs_saved_runs!();
    let tape = read_tape(CAPSULE_2800);
    for (first, second) in [(0, 1), (10, 2000), (2798, 2799)] {
        let mut changed = tape.clone();
        changed["frames"].as_array_mut().unwrap().swap(first, second);
        assert_eq!(
            check(&changed),
            broken(TapeCause::Chain, Some(first as u64)),
            "{first} and {second}"
        );
    }
}

#[test]
fn a_changed_outcome_is_a_run_seal_failure() {
    needs_saved_runs!();
    let mut tape = read_tape(CAPSULE_2800);
    let frames = frame_count(&tape);
    tape["outcome"] = json!("refused");
    assert_eq!(check(&tape), broken(TapeCause::RunSeal, Some(frames)));
    let mut tape = read_tape(CAPSULE_REFUSED);
    let frames = frame_count(&tape);
    tape["outcome"] = json!("completed");
    assert_eq!(check(&tape), broken(TapeCause::RunSeal, Some(frames)));
}

#[test]
fn a_changed_step_count_is_a_run_seal_failure_and_a_zero_count_is_a_state_failure() {
    needs_saved_runs!();
    let tape = read_tape(CAPSULE_2800);
    let frames = frame_count(&tape);
    for landed in [2799, 2801, 1] {
        let mut changed = tape.clone();
        changed["steps_landed"] = json!(landed);
        assert_eq!(check(&changed), broken(TapeCause::RunSeal, Some(frames)), "{landed}");
    }
    // No step landed and a state is present: the state cannot belong to the tape.
    let mut changed = tape;
    changed["steps_landed"] = json!(0);
    assert_eq!(check(&changed), broken(TapeCause::State, None));
}

#[test]
fn a_changed_refusal_name_is_a_run_seal_failure() {
    needs_saved_runs!();
    let tape = read_tape(CAPSULE_REFUSED);
    assert_eq!(tape["reject"], json!("AltitudeOutsideCard"));
    let frames = frame_count(&tape);
    for reject in [json!("MachOutsideCard"), json!(null), json!("")] {
        let mut changed = tape.clone();
        changed["reject"] = reject.clone();
        assert_eq!(check(&changed), broken(TapeCause::RunSeal, Some(frames)), "{reject}");
    }
    let mut completed = read_tape(CAPSULE_2800);
    let frames = frame_count(&completed);
    completed["reject"] = json!("AltitudeOutsideCard");
    assert_eq!(check(&completed), broken(TapeCause::RunSeal, Some(frames)));
}

#[test]
fn a_changed_state_hex_digit_is_a_state_failure() {
    needs_saved_runs!();
    let tape = read_tape(CAPSULE_2800);
    for key in ["altitude_m_hex", "mach_hex", "t_s_hex", "residual_conservation_error_hex"] {
        let mut changed = tape.clone();
        let text = changed["final_state"][key].as_str().unwrap().to_string();
        changed["final_state"][key] = json!(flip_hex_digit(&text, 15));
        assert_eq!(check(&changed), broken(TapeCause::State, None), "{key}");
    }
    // A flag is sealed with the hex values.
    let mut changed = tape.clone();
    changed["final_state"]["past_flight_deployment"] = json!(false);
    assert_eq!(check(&changed), broken(TapeCause::State, None));
    // One component of a 3-vector.
    let mut satellite = eight_short_runs().remove(0);
    let text = satellite["final_state"]["position_m_hex"][1].as_str().unwrap().to_string();
    satellite["final_state"]["position_m_hex"][1] = json!(flip_hex_digit(&text, 0));
    assert_eq!(check(&satellite), broken(TapeCause::State, None));
    // The stored digest itself.
    let mut changed = tape;
    let text = changed["state_digest"].as_str().unwrap().to_string();
    changed["state_digest"] = json!(flip_hex_digit(&text, 40));
    assert_eq!(check(&changed), broken(TapeCause::State, None));
}

#[test]
fn a_changed_residual_hex_digit_is_a_state_failure() {
    needs_saved_runs!();
    let tape = read_tape(CAPSULE_2800);
    for key in [
        "final_residual_conservation_error_hex",
        "max_residual_conservation_error_hex",
    ] {
        let mut changed = tape.clone();
        let text = changed[key].as_str().unwrap().to_string();
        changed[key] = json!(flip_hex_digit(&text, 9));
        assert_eq!(check(&changed), broken(TapeCause::State, None), "{key}");
    }
}

#[test]
fn a_changed_request_is_a_chain_failure_at_the_first_frame() {
    needs_saved_runs!();
    let tape = read_tape(CAPSULE_2800);
    // The load of the request: this card's load is empty, and a field put
    // into it changes the request.
    let mut changed = tape.clone();
    changed["inputs"]["segments"][0]["load"] = json!({ "heat_power_w": 1.0 });
    assert_eq!(check(&changed), broken(TapeCause::Chain, Some(0)));
    let mut changed = tape.clone();
    changed["inputs"]["initial"]["radius_m"] = json!(6503140.000000001);
    assert_eq!(check(&changed), broken(TapeCause::Chain, Some(0)));
    let mut changed = tape.clone();
    changed["inputs"]["dt_s"] = json!(0.025);
    assert_eq!(check(&changed), broken(TapeCause::Chain, Some(0)));
    let mut changed = tape.clone();
    changed["inputs"]["card"] = json!("satellite_lageos1");
    assert_eq!(check(&changed), broken(TapeCause::Chain, Some(0)));
    // The same request written another way is the same request.
    let mut same = tape;
    same["inputs"]["dt_s"] = json!(5e-2);
    same["inputs"]["record_every"] = json!(1.0);
    same["inputs"]["segments"][0]["steps"] = json!(2800.0);
    assert_eq!(check(&same), holds());
}

#[test]
fn a_changed_run_seal_is_a_run_seal_failure() {
    needs_saved_runs!();
    let mut tape = read_tape(CAPSULE_2800);
    let frames = frame_count(&tape);
    let text = tape["run_seal"].as_str().unwrap().to_string();
    tape["run_seal"] = json!(flip_hex_digit(&text, 63));
    assert_eq!(check(&tape), broken(TapeCause::RunSeal, Some(frames)));
}

#[test]
fn the_chain_is_named_before_the_state_and_the_state_before_the_run_seal() {
    needs_saved_runs!();
    let tape = read_tape(CAPSULE_2800);
    let mut changed = tape.clone();
    changed["outcome"] = json!("refused");
    let text = changed["final_state"]["mach_hex"].as_str().unwrap().to_string();
    changed["final_state"]["mach_hex"] = json!(flip_hex_digit(&text, 2));
    assert_eq!(check(&changed), broken(TapeCause::State, None));
    let text = changed["frames"][40]["frame_hex"].as_str().unwrap().to_string();
    changed["frames"][40]["frame_hex"] = json!(flip_hex_digit(&text, 2));
    assert_eq!(check(&changed), broken(TapeCause::Chain, Some(40)));
}

#[test]
fn a_changed_decimal_is_not_sealed_and_the_receipt_catches_it() {
    needs_saved_runs!();
    // The seals cover the hex values. A decimal beside its hex is a reading
    // aid, and only the receipt over the response bytes covers it.
    let bytes = read_bytes(CAPSULE_2800);
    let text = String::from_utf8(bytes).unwrap();
    assert!(text.contains("\"mach\":1.1446497465469e0") || text.contains("\"mach\":1.1446497465469"));
    let changed = text.replacen("\"mach\":1.1", "\"mach\":9.9", 1);
    assert_ne!(changed, text);
    assert_eq!(check_tape_text(changed.as_bytes()).unwrap(), holds());
    assert_eq!(check_receipt(changed.as_bytes()), ReceiptCheck::DoesNotMatch);
    assert_eq!(check_receipt(text.as_bytes()), ReceiptCheck::Matches);
}

#[test]
fn a_state_name_no_card_publishes_is_caught_another_way_than_the_engine_catches_it() {
    needs_saved_runs!();
    // The engine refuses a state name its card does not publish. This crate
    // knows no card. A name that is not sealed and has no hex sibling is
    // refused here. A name that is sealed changes the digest.
    let tape = read_tape(CAPSULE_2800);
    let mut changed = tape.clone();
    changed["final_state"]["not_a_field"] = json!("4072526666666666");
    assert_eq!(
        check_tape(&changed).unwrap_err(),
        Refusal::NotSealed("final_state.not_a_field".to_string())
    );
    let mut changed = tape.clone();
    changed["final_state"]["drogue_mach_window"] = json!("false");
    assert_eq!(
        check_tape(&changed).unwrap_err().to_string(),
        "Physical Violation: 'final_state.drogue_mach_window' is not a JSON boolean and has no '_hex' sibling."
    );
    let mut changed = tape.clone();
    changed["final_state"]["not_a_field"] = json!(1.0);
    changed["final_state"]["not_a_field_hex"] = json!("3ff0000000000000");
    assert_eq!(check(&changed), broken(TapeCause::State, None));
    let mut changed = tape;
    changed["final_state"]["not_a_flag"] = json!(true);
    assert_eq!(check(&changed), broken(TapeCause::State, None));
}

#[test]
fn a_run_made_with_no_engine_holds() {
    // The seal is not keyed. Anyone who follows the published rules can
    // seal a run the engine never made, and the check here holds it. That
    // is the stated limit of this check: it shows a record was not altered
    // after it was sealed. That the engine made it is shown only by sending
    // the request to the engine and comparing run seals.
    let request = json!({
        "domain": "energy", "card": "thermal",
        "initial": { "temperature_k": 293.15 }, "dt_s": 0.1,
        "segments": [{ "steps": 25, "load": { "heat_power_w": 1.0, "ambient_k": 293.15 } }],
        "record_every": 4
    });
    let tape = fabricated_run(request, 25, 4, 0x5a);
    assert_eq!(frame_count(&tape), 7);
    assert_eq!(check(&tape), holds());
    // No plate reaches a million kelvin. Nothing here can know that.
    assert_eq!(tape["final_state"]["temperature_k"], json!(1000000.0));
}
