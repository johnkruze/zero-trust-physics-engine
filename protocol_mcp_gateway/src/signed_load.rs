//! A signed load at the door: an instrument's signature over a reading,
//! verified before a physics permit is taken.
//!
//! The rule is `genesis_seal::attest`, the one implementation: this module
//! reads where a request may carry `signed`, turns a reading that does not
//! verify into the door's refusal, writes the label every run answer carries
//! (`load_source`), and refuses a plan or a search that would set a value of
//! a signed reading (`SignedLoadReplaced`). The engine never signs: a
//! signature shows that the holder of a key stated a reading, and which keys
//! to trust is the verifier's list.

use genesis_seal::attest::{self, Place, Verified};
use serde_json::{Value, json};

use crate::DoorError;

/// The key of a signed reading.
pub(crate) const SIGNED: &str = attest::SIGNED;

/// The name of a plan or a search refused for setting a signed value. The
/// names a reading itself meets are `genesis_seal::attest`'s:
/// LoadSignatureDoesNotVerify, LoadSignatureMalformed and
/// LoadSignatureAlgorithmNotServed.
pub(crate) const REPLACED: &str = attest::REPLACED;

/// The sentence of a reading that is not taken.
fn refused(fault: attest::AttestFault) -> DoorError {
    DoorError::Unprocessable(fault.to_string())
}

/// Every signed reading of a run request, verified, in place order: the
/// door's reading of a run, before `Live::open` and before a permit.
pub(crate) fn verify_run(request: &Value) -> Result<Vec<Verified>, DoorError> {
    attest::verify_run(request).map_err(refused)
}

/// The signed reading of a solve's `parameters`, verified, when it has one.
pub(crate) fn verify_solve(request: &Value) -> Result<Option<Verified>, DoorError> {
    Ok(attest::verify_all(&attest::solve_places(request)).map_err(refused)?.into_iter().next())
}

/// Each signed measurement of a comparison, verified, in order.
pub(crate) fn verify_measurements(measurements: &Value) -> Result<Vec<Verified>, DoorError> {
    attest::verify_all(&attest::measurement_places(measurements)).map_err(refused)
}

/// `load_source` of a run request whose readings have verified.
pub(crate) fn load_source(request: &Value, readings: &[Verified]) -> Value {
    let segments = request.get("segments").and_then(Value::as_array).map_or(0, Vec::len);
    attest::load_source(segments, readings)
}

/// `load_source` of a solve request, present only when its parameters carry
/// a signature: an answer without it is the caller's number.
pub(crate) fn solve_source_of(request: &Value) -> Option<Value> {
    let places = attest::solve_places(request);
    let reading = attest::verify_all(&places).ok()?.into_iter().next()?;
    Some(attest::parameters_source(&reading))
}

/// `measurement_source` of a comparison's measurements.
pub(crate) fn measurement_source(measurements: &Value, readings: &[Verified]) -> Value {
    attest::measurement_source(measurements.as_array().map_or(0, Vec::len), readings)
}

/// The refusal of a plan or a search that would set a value of a signed
/// reading: the value signed would no longer be the value run.
fn replaced(path: &str, place: Place) -> DoorError {
    DoorError::Unprocessable(format!(
        "Physical Violation: '{path}' would set a value of the reading signed at '{}.{SIGNED}', so the value run would not be the value its instrument signed. A signed reading is run as it was signed; vary a reading the caller states, or send each signed reading as its own run ({REPLACED}).",
        place.label()
    ))
}

/// The signed place a path of a run or a solve request falls in, if any. A
/// path is `initial.<name>`, `segments[i].load.<name>` or
/// `segments[i].signed...`, `parameters.<name>`, or `segments[*]...` for
/// every segment; `dt_s`, a segment's `steps` and `record_every` are not
/// signed.
fn signed_place_of(request: &Value, path: &str) -> Option<Place> {
    let holds_signed = |holder: Option<&Value>| holder.and_then(Value::as_object).is_some_and(|object| object.contains_key(SIGNED));
    if path.starts_with("initial.") || path == "initial" {
        return holds_signed(request.get("initial")).then_some(Place::Initial);
    }
    if path.starts_with("parameters.") || path == "parameters" {
        return holds_signed(request.get("parameters")).then_some(Place::Parameters);
    }
    let rest = path.strip_prefix("segments[")?;
    let (index, after) = rest.split_once(']')?;
    let signed_part = after.starts_with(".load") || after.starts_with(&format!(".{SIGNED}"));
    if !signed_part {
        return None;
    }
    let segments = request.get("segments")?.as_array()?;
    let indices: Vec<usize> = if index == "*" { (0..segments.len()).collect() } else { vec![index.parse().ok()?] };
    indices.into_iter().find(|index| holds_signed(segments.get(*index))).map(Place::Segment)
}

/// A `run_dataset` request whose grid axes or sample draws set a value of a
/// signed reading of its base is refused before any row runs.
pub(crate) fn refuse_replaced_in_plan(request: &Value) -> Result<(), DoorError> {
    let Some(base) = request.get("plan").and_then(|plan| plan.get("base")) else {
        return Ok(());
    };
    // A component of a vector, `name[c]`, is in the place its field is in.
    for path in crate::dataset::varied_paths(request) {
        if let Some(place) = signed_place_of(base, &path) {
            return Err(replaced(&path, place));
        }
    }
    Ok(())
}

/// A `find_flip` search whose varied place falls in a signed reading of its
/// request is refused before any run.
pub(crate) fn refuse_replaced_in_search(request: &Value, vary: &str) -> Result<(), DoorError> {
    match signed_place_of(request, vary) {
        Some(place) => Err(replaced(vary, place)),
        None => Ok(()),
    }
}

/// The schema of `signed`, where a request may carry it: the fields and
/// their forms. What they mean is said once, at run_trajectory's segments
/// (`SIGNED_SAYS`).
pub(crate) fn schema() -> Value {
    json!({
        "type": "object",
        "description": "An instrument's Ed25519 signature over this reading, by the rule run_trajectory's input schema gives at segments.",
        "properties": {
            "alg": { "type": "string", "enum": [attest::ALG_ED25519] },
            "key": { "type": "string", "pattern": "^[0-9a-f]{64}$" },
            "instrument": { "type": "string", "minLength": 1, "maxLength": attest::MOST_INSTRUMENT_CHARS },
            "time": { "type": "string" },
            "seq": { "type": "integer", "minimum": 0, "maximum": attest::SEQ_BELOW - 1 },
            "prev": { "type": ["string", "null"] },
            "challenge": { "type": ["string", "null"] },
            "sig": { "type": "string", "pattern": "^[0-9a-f]{128}$" }
        },
        "required": attest::FIELDS,
        "additionalProperties": false
    })
}

/// What `signed` is, said in the input schema where a request carries it.
pub(crate) const SIGNED_SAYS: &str = "A segment may carry signed beside its load: an instrument's signature over the load. signed holds alg \"ed25519\"; key, the instrument's public key as 64 hex digits, its identity; instrument, the name it gives itself, a label of at most 200 characters; time, its clock, RFC 3339 in UTC ending in Z; seq, its reading counter, below 2^53; prev, the SHA-256 of its previous reading's signed text, or null; challenge, hex the relying party chose, or null; and sig, the signature as 128 hex digits. The instrument signs the bytes \"zero-trust-physics/signed-reading/v1\" and a line break, then the sealed form (the text a request's seal is taken over: keys sorted by their bytes, each number x and the 16 hex digits of its 64-bit float, no spaces) of the object of alg, key, instrument, time, seq, prev and challenge with the reading under its place's key: \"load\" for a segment's load, \"initial\", \"parameters\" or \"measurement\" for the object it sits in, without signed. Ed25519 as RFC 8032 section 5.1.7, strict: the key and R canonical points of more than small order, S below the group order, the cofactorless equation. It is verified before computing, every field required and none filled in: a signature that does not verify is refused as LoadSignatureDoesNotVerify, a field of the wrong form as LoadSignatureMalformed, an alg other than \"ed25519\" as LoadSignatureAlgorithmNotServed, and a plan or search that would set a signed value as SignedLoadReplaced. The seal covers the signature, since it covers the request. A signature shows that the holder of the key stated the reading; whether the key is a real instrument's is the verifier's list.";

/// What the answer's `load_source` says, in the input schema of a run.
pub(crate) const LOAD_SOURCE_SAYS: &str = "Every run answer carries load_source: for a run with no signed load, the count of segments, none signed, and a sentence that each load is the number the request states; for a signed run, how many loads are signed, whether initial is, whether each key's readings form one unbroken stream (prev and seq), and for each key its fingerprint, the name it gives itself, its places, times, seq and challenges. It is outside the run seal, follows from inputs, which the seal covers, and is covered by the receipt.";

/// The sentence a run's description gains.
pub(crate) const RUN_SAYS: &str = "A load may be signed; load_source says by whom.";

/// What a solve's `parameters` say of a signature, in its input schema.
pub(crate) const SOLVE_SAYS: &str = "parameters may also carry signed, an instrument's signature over the other parameters under the key \"parameters\", by the rule run_trajectory's input schema gives at segments; it is verified before solving, the solver takes the other parameters, and the answer then carries load_source, naming the key. A solve answer without load_source holds no signed parameter.";

#[cfg(test)]
mod tests {
    use super::*;
    use crate::AppState;
    use std::time::Duration;

    /// A signed battery run of three one-second loads by a test key whose
    /// secret is the SHA-256 of a sentence that says it is one.
    fn signed_run() -> Value {
        use ed25519_dalek::{Signer, SigningKey};
        use sha2::Digest;
        let key = SigningKey::from_bytes(&sha2::Sha256::digest(b"a door test key; not an instrument's").into());
        let mut segments = Vec::new();
        for index in 0..3_u64 {
            let load = json!({ "current_a": 1.0 + index as f64 });
            let mut signed = serde_json::Map::new();
            signed.insert("alg".into(), json!("ed25519"));
            signed.insert("key".into(), json!(hex::encode(key.verifying_key().to_bytes())));
            signed.insert("instrument".into(), json!("door test"));
            signed.insert("time".into(), json!(format!("2026-10-21T14:02:0{index}Z")));
            signed.insert("seq".into(), json!(index));
            signed.insert("prev".into(), Value::Null);
            signed.insert("challenge".into(), Value::Null);
            let text = attest::signed_text(Place::Segment(index as usize), &signed, &load);
            signed.insert("sig".into(), json!(hex::encode(key.sign(&text).to_bytes())));
            segments.push(json!({ "steps": 1, "load": load, "signed": signed }));
        }
        json!({ "domain": "energy", "card": "battery_sag", "initial": { "soc": 1.0 }, "dt_s": 1.0, "segments": segments, "record_every": 1 })
    }

    /// With the one permit held, a signature that does not verify is refused
    /// by name, not answered as a full pool: it is read before a permit is
    /// asked for. The same run with its signatures whole waits for no permit
    /// and is told the pool is full.
    #[tokio::test]
    async fn a_signature_is_verified_before_a_permit_is_taken() {
        let state = AppState::with_hold(1, Duration::from_millis(600));
        let holder = {
            let state = state.clone();
            tokio::spawn(async move { crate::trajectory::run(&state, signed_run()).await.map(|answer| answer.run_seal) })
        };
        tokio::time::sleep(Duration::from_millis(200)).await;
        let mut edited = signed_run();
        edited["segments"][1]["load"]["current_a"] = json!(2.5);
        match crate::trajectory::run(&state, edited).await {
            Err(DoorError::Unprocessable(said)) => assert!(said.ends_with("(LoadSignatureDoesNotVerify)."), "{said}"),
            _ => panic!("an edited reading was not refused before the permit"),
        }
        match crate::trajectory::run(&state, signed_run()).await {
            Err(DoorError::TooMany(said)) => assert_eq!(said, "Too many concurrent physics tasks."),
            _ => panic!("a full pool is 429"),
        }
        assert!(holder.await.unwrap().is_ok());
    }

    /// A solve answer whose signed parameter was changed and whose seal was
    /// made again, by the engine's own sealing, over the changed request:
    /// every part of the seal holds, and the long form names the signature.
    #[tokio::test]
    async fn the_long_form_of_a_solve_names_a_signature_that_does_not_verify_under_a_seal_that_holds() {
        use ed25519_dalek::{Signer, SigningKey};
        use sha2::Digest;
        let key = SigningKey::from_bytes(&sha2::Sha256::digest(b"a door test key; not an instrument's").into());
        let reading = json!({ "atomic_number": 10 });
        let mut signed = serde_json::Map::new();
        for (field, value) in [
            ("alg", json!("ed25519")),
            ("key", json!(hex::encode(key.verifying_key().to_bytes()))),
            ("instrument", json!("door test")),
            ("time", json!("2026-10-21T14:02:00Z")),
            ("seq", json!(7)),
            ("prev", Value::Null),
            ("challenge", Value::Null),
        ] {
            signed.insert(field.into(), value);
        }
        let text = attest::signed_text(Place::Parameters, &signed, &reading);
        signed.insert("sig".into(), json!(hex::encode(key.sign(&text).to_bytes())));
        let request = json!({ "solver": "atom_dft", "parameters": { "atomic_number": 10, "signed": signed } });
        let state = crate::new_gateway_state();
        let honest_text = serde_json::to_string(&crate::solution::run(&state, request.clone()).await.unwrap_or_else(|error| panic!("{}", error.message()))).unwrap();
        let honest: Value = serde_json::from_str(&honest_text).unwrap();
        assert_eq!(honest["load_source"]["parameters"], "signed");
        let held: Value = serde_json::from_str(&crate::solution::verify(&state, &honest, Some(&honest_text)).await.unwrap_or_else(|error| panic!("{}", error.message()))).unwrap();
        assert_eq!(held["holds"], true, "{held}");
        // The forger changes the reading's seq and seals the changed request
        // by the published rule: the solver's answer is the same.
        let mut forged_request = request.clone();
        forged_request["parameters"]["signed"]["seq"] = json!(8);
        let solver = crate::solvers::find("atom_dft").unwrap();
        let given = crate::solution::read_parameters(solver, reading.as_object().unwrap()).unwrap_or_else(|error| panic!("{}", error.message()));
        let solved = (solver.solve)(&given).unwrap();
        let forged = serde_json::to_value(crate::solution::seal(solver, forged_request, solved).unwrap()).unwrap();
        assert_eq!(forged["result"], honest["result"]);
        let report: Value = serde_json::from_str(&crate::solution::verify(&state, &forged, None).await.unwrap_or_else(|error| panic!("{}", error.message()))).unwrap();
        assert_eq!(report["holds"], false, "{report}");
        assert_eq!(report["cause"], "load_signature", "{report}");
        assert_eq!(report["differs_at"], "inputs.parameters.signed");
        assert!(report["difference"].as_str().unwrap().ends_with("(LoadSignatureDoesNotVerify)."), "{report}");
    }

    #[test]
    fn a_path_into_a_signed_reading_is_found_and_one_beside_it_is_not() {
        let request = signed_run();
        for (path, place) in [
            ("segments[1].load.current_a", Some(Place::Segment(1))),
            ("segments[*].load.current_a", Some(Place::Segment(0))),
            ("segments[2].signed.seq", Some(Place::Segment(2))),
            ("segments[1].steps", None),
            ("dt_s", None),
            ("initial.soc", None),
            ("segments[7].load.current_a", None),
        ] {
            assert_eq!(signed_place_of(&request, path), place, "{path}");
        }
        let mut start = request.clone();
        start["initial"]["signed"] = json!({});
        assert_eq!(signed_place_of(&start, "initial.soc"), Some(Place::Initial));
    }
}
