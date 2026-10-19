//! Concurrency spike against the `physics_permit_count()` semaphore.
//!
//! The compiled step returns before the next HTTP request is parsed, so this
//! probe holds each taken permit after the step. The hold is `#[cfg(test)]`
//! and is not in the production binary. `app()` holds for zero.
//! Overflow must be HTTP 429 with no seal, and the runtime must still answer
//! `/health` and one more sealed step after the permits are released.

use std::time::Duration;

use genesis_kernel::grid_node;
use reqwest::Client;
use serde_json::Value;
use tokio::net::TcpListener;
use tokio::task::JoinSet;

use super::{app_with_permit_hold, physics_permit_count};

const HOLD: Duration = Duration::from_millis(1500);
const ACQUIRE_WAIT: Duration = Duration::from_millis(300);

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn permit_spike_returns_429_without_killing_the_runtime() {
    let permits = physics_permit_count();
    assert!(
        permits >= 1 && permits <= 32,
        "permit count {permits} left the cap"
    );
    let base_url = spawn_held_app().await;
    let client = Client::builder()
        .pool_max_idle_per_host(1024)
        .tcp_nodelay(true)
        .timeout(Duration::from_secs(30))
        .build()
        .unwrap();
    wait_until_healthy(&client, &base_url).await;

    let body = grid_body();
    let holders = fanout(&client, &base_url, &body, permits);
    tokio::time::sleep(ACQUIRE_WAIT).await;
    let overflow = permits.saturating_mul(8);
    let rejected = fanout(&client, &base_url, &body, overflow);

    let held = join_all(holders).await;
    let spilled = join_all(rejected).await;
    assert_eq!(held.len(), permits, "dropped holder tasks");
    assert_eq!(spilled.len(), overflow, "dropped overflow tasks");

    let mut sealed = 0usize;
    let mut too_many = 0usize;
    for hit in &held {
        assert_eq!(hit.status, 200, "holder missed the pool: {}", hit.detail);
        assert_sealed(&hit.detail);
        sealed += 1;
    }
    for hit in &spilled {
        assert_eq!(
            hit.status, 429,
            "overflow was not rejected by the semaphore: {}",
            hit.detail
        );
        assert!(
            hit.detail.contains("Too many concurrent physics tasks"),
            "{}",
            hit.detail
        );
        assert!(
            !hit.detail.contains("sha384_seal"),
            "429 wrote a frame seal: {}",
            hit.detail
        );
        assert!(
            !hit.detail.contains("receipt_sha384"),
            "429 wrote a receipt: {}",
            hit.detail
        );
        too_many += 1;
    }
    assert_eq!(sealed, permits);
    assert_eq!(too_many, overflow);

    let released = post(&client, &base_url, &body).await;
    assert_eq!(
        released.status, 200,
        "pool did not release: {}",
        released.detail
    );
    assert_sealed(&released.detail);

    let health = client
        .get(format!("{base_url}/health"))
        .send()
        .await
        .expect("listener died after the spike");
    assert_eq!(
        health.status().as_u16(),
        200,
        "listener died after the spike"
    );
}

struct Hit {
    status: u16,
    detail: String,
}

fn assert_sealed(body: &str) {
    let parsed: Value = serde_json::from_str(body).unwrap_or_else(|err| panic!("{err}: {body}"));
    let seal = parsed["sha384_seal"].as_str().unwrap_or("");
    assert_eq!(seal.len(), 96, "{body}");
    assert!(seal.chars().all(|c| c.is_ascii_hexdigit()), "{seal}");
    let receipt = parsed["receipt_sha384"].as_str().unwrap_or("");
    assert_eq!(receipt.len(), 96, "{body}");
    assert!(receipt.chars().all(|c| c.is_ascii_hexdigit()), "{receipt}");
    assert_ne!(receipt, seal, "{body}");
    assert!(
        parsed["state"]["temperature_k"].as_f64().unwrap() > 0.0,
        "{body}"
    );
    assert_eq!(
        parsed["state"]["temperature_k_hex"].as_str().unwrap().len(),
        16,
        "{body}"
    );
    assert!(parsed["state"]["current_a"].is_number(), "{body}");
    assert!(parsed["state"]["voltage_v"].is_number(), "{body}");
    assert!(parsed["residual_conservation_error"].is_number(), "{body}");
}

fn fanout(client: &Client, base_url: &str, body: &str, count: usize) -> JoinSet<Hit> {
    let mut join_set = JoinSet::new();
    for _ in 0..count {
        let client = client.clone();
        let base_url = base_url.to_string();
        let body = body.to_string();
        join_set.spawn(async move { post(&client, &base_url, &body).await });
    }
    join_set
}

async fn join_all(mut join_set: JoinSet<Hit>) -> Vec<Hit> {
    let mut hits = Vec::new();
    while let Some(joined) = join_set.join_next().await {
        hits.push(joined.expect("physics task panicked"));
    }
    hits
}

async fn post(client: &Client, base_url: &str, body: &str) -> Hit {
    let response = match client
        .post(format!("{base_url}/verify_physics"))
        .header("content-type", "application/json")
        .body(body.to_string())
        .send()
        .await
    {
        Ok(response) => response,
        Err(err) => {
            return Hit {
                status: 0,
                detail: format!("transport: {err}"),
            };
        }
    };
    let status = response.status().as_u16();
    let detail = response.text().await.unwrap_or_else(|err| err.to_string());
    Hit { status, detail }
}

fn grid_body() -> String {
    let node = grid_node::GridNode::copper_fuse_link();
    let temperature = serde_json::to_string(&grid_node::REFERENCE_TEMPERATURE_K).unwrap();
    let dt = serde_json::to_string(&node.gate_dt_s().unwrap()).unwrap();
    format!(
        r#"{{"domain":"energy","card":"grid_node","parameters":{{"current_a":0.0,"voltage_v":0.0,"temperature_k":{temperature},"upstream_voltage_v":25.0,"load_current_a":0.0,"ambient_k":{temperature},"dt_s":{dt}}}}}"#
    )
}

async fn spawn_held_app() -> String {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    tokio::spawn(async move {
        axum::serve(listener, app_with_permit_hold(HOLD))
            .await
            .unwrap();
    });
    tokio::task::yield_now().await;
    format!("http://127.0.0.1:{port}")
}

async fn wait_until_healthy(client: &Client, base_url: &str) {
    let mut last = String::new();
    for _ in 0..40 {
        match client.get(format!("{base_url}/health")).send().await {
            Ok(response) if response.status().as_u16() == 200 => return,
            Ok(response) => last = format!("status {}", response.status()),
            Err(err) => last = err.to_string(),
        }
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
    panic!("gateway did not become healthy: {last}");
}
