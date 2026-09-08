//! The demo-workspace bootstrap — the one piece of `data_ops.rs` that is not
//! an op and did not port.
//!
//! HS-9 moved forge-crm's last four op bodies into `ops/*.java`, which is
//! what deleted `data_ops.rs`. This is what was underneath them: a
//! first-load seeder for the demo catalog's instant-temp-user flow. It is
//! not an op — nothing dispatches it by name — and three things about it are
//! outside the dialect on purpose rather than by omission:
//!
//!   * it swallows an error per insert and keeps going, so one bad row never
//!     blocks a dashboard render. The dialect has no `try` (`FL2034`);
//!   * it builds a diagnostic string by mutation across twenty-odd steps;
//!   * it reads `ctx.tenant_id`, which the dialect's context surface does
//!     not carry (it has `user_id` and `is_platform_admin`).
//!
//! So it stays Rust, in the HOST crate, and `lib.rs` runs it before
//! dispatching `dashboard_summary` — exactly where the Rust op ran it — and
//! hands the diagnostic to the dialect op as an input field. The op echoes
//! it, so the value and its position on the wire are unchanged.
//!
//! It is NOT a raw-Rust op body and should not be counted as one by HS-7's
//! gate; if that gate counts by file rather than by dispatched op, this is
//! the file to teach it about.

#![cfg(target_arch = "wasm32")]

use forge_sdk_v2::{OpContext, OpError};
use serde_json::{json, Value};

fn query(spec: Value) -> Result<Value, OpError> {
    forge_sdk_v2::storage::query(&spec).map_err(|e| OpError::Internal(format!("query: {e:?}")))
}

fn mutate(spec: Value) -> Result<Value, OpError> {
    forge_sdk_v2::storage::mutate(&spec).map_err(|e| OpError::Internal(format!("mutate: {e:?}")))
}

fn rows(v: &Value) -> Vec<Value> {
    v.get("rows").and_then(|r| r.as_array()).cloned().unwrap_or_default()
}

/// Best-effort query -> rows, swallowing errors to an empty Vec.
fn qrows(spec: Value) -> Vec<Value> {
    query(spec).map(|v| rows(&v)).unwrap_or_default()
}

/// The guard the Rust `dashboard_summary` opened with, lifted verbatim:
/// seed only when `pipeline_stages` (the first thing seeded) is empty, so
/// the bootstrap runs exactly once per tenant even if a later step failed.
pub fn seed_if_empty(ctx: &OpContext) -> String {
    if qrows(json!({ "from": "pipeline_stages", "select": ["id"], "limit": 1 })).is_empty() {
        seed_demo_data(ctx)
    } else {
        "skipped".to_string()
    }
}

// ── the seed (dashboard first-load) ─────────────────────────────
/// Insert a batch of rows and return their ids in order.
fn insert_batch(table: &str, rows_in: Vec<Value>) -> Result<Vec<String>, OpError> {
    let res = mutate(json!({ "table": table, "op": "insert", "rows": rows_in }))?;
    Ok(res
        .get("ids")
        .and_then(|v| v.as_array())
        .map(|a| a.iter().filter_map(|v| v.as_str().map(str::to_string)).collect())
        .unwrap_or_default())
}

/// Seed a private, pre-populated sample CRM for a freshly-created demo
/// user. Everything is scoped to the caller's `tenant_id` (+ `owner_id`
/// when the caller resolves to a real user). Each demo user (each signup
/// mints its own tenant) gets an isolated copy they can edit freely.
/// Fully resilient: every step logs and continues, so one bad insert
/// never aborts the rest or blocks the dashboard render.
fn seed_demo_data(ctx: &OpContext) -> String {
    let tenant = match ctx.tenant_id.clone().filter(|t| !t.is_empty()) {
        Some(t) => t,
        None => return "no-tenant".to_string(),
    };
    let diag = std::cell::RefCell::new(vec![format!("uid={:?}", ctx.user_id)]);
    let t = tenant.as_str();
    // owner_id is optional metadata (RLS scopes records by tenant_id, not
    // owner), so leave it unset in the seed — the session subject's exact
    // shape can then never trip the column's format:uuid check.
    let owned = |row: Value| -> Value { row };
    let ins = |label: &str, table: &str, rows_in: Vec<Value>| -> Vec<String> {
        match insert_batch(table, rows_in) {
            Ok(ids) => {
                diag.borrow_mut().push(format!("{label}:{}", ids.len()));
                ids
            }
            Err(e) => {
                diag.borrow_mut().push(format!("{label}:ERR({e:?})"));
                Vec::new()
            }
        }
    };

    let stages = ins("stages", "pipeline_stages", vec![
        json!({"tenant_id":t,"name":"Prospecting","position":1,"probability":10,"is_won":false,"is_lost":false}),
        json!({"tenant_id":t,"name":"Qualification","position":2,"probability":25,"is_won":false,"is_lost":false}),
        json!({"tenant_id":t,"name":"Proposal","position":3,"probability":50,"is_won":false,"is_lost":false}),
        json!({"tenant_id":t,"name":"Negotiation","position":4,"probability":75,"is_won":false,"is_lost":false}),
        json!({"tenant_id":t,"name":"Closed Won","position":5,"probability":100,"is_won":true,"is_lost":false}),
        json!({"tenant_id":t,"name":"Closed Lost","position":6,"probability":0,"is_won":false,"is_lost":true}),
    ]);
    let stage = |i: usize| stages.get(i).cloned().unwrap_or_default();

    let accounts = ins("accounts", "accounts", vec![
        owned(json!({"tenant_id":t,"name":"Northwind Trading","industry":"retail","type":"customer","employee_count":1200,"annual_revenue":48_000_000.0,"billing_city":"Chicago","billing_country":"USA","website":"northwind.example","description":"Long-time customer; multi-region rollout underway."})),
        owned(json!({"tenant_id":t,"name":"Helios Robotics","industry":"manufacturing","type":"prospect","employee_count":340,"annual_revenue":9_500_000.0,"billing_city":"Austin","billing_country":"USA","website":"helios.example","description":"Evaluating a migration off a legacy CRM."})),
        owned(json!({"tenant_id":t,"name":"Meridian Health","industry":"healthcare","type":"prospect","employee_count":5400,"annual_revenue":210_000_000.0,"billing_city":"Boston","billing_country":"USA","website":"meridian.example","description":"Enterprise opportunity; security review in progress."})),
        owned(json!({"tenant_id":t,"name":"Cobalt Logistics","industry":"technology","type":"customer","employee_count":80,"annual_revenue":6_200_000.0,"billing_city":"Denver","billing_country":"USA","website":"cobalt.example","description":"Expanding seats next quarter."})),
    ]);
    let acct = |i: usize| accounts.get(i).cloned().unwrap_or_default();

    let _ = ins("contacts", "contacts", vec![
        owned(json!({"tenant_id":t,"account_id":acct(0),"first_name":"Dana","last_name":"Whitfield","title":"VP Operations","department":"Operations","phone":"+1-312-555-0142","status":"active"})),
        owned(json!({"tenant_id":t,"account_id":acct(0),"first_name":"Marcus","last_name":"Lee","title":"Procurement Lead","department":"Finance","phone":"+1-312-555-0188","status":"active"})),
        owned(json!({"tenant_id":t,"account_id":acct(1),"first_name":"Priya","last_name":"Nair","title":"CTO","department":"Engineering","phone":"+1-512-555-0110","status":"active"})),
        owned(json!({"tenant_id":t,"account_id":acct(2),"first_name":"Sam","last_name":"Okafor","title":"Director of IT","department":"IT","phone":"+1-617-555-0173","status":"active"})),
        owned(json!({"tenant_id":t,"account_id":acct(3),"first_name":"Lena","last_name":"Hartman","title":"Head of Sales","department":"Sales","phone":"+1-303-555-0129","status":"active"})),
    ]);

    let _ = ins("opportunities", "opportunities", vec![
        owned(json!({"tenant_id":t,"name":"Northwind — multi-region expansion","account_id":acct(0),"stage_id":stage(3),"amount":120_000.0,"currency":"USD","status":"open","probability":75,"description":"Expansion across three regions."})),
        owned(json!({"tenant_id":t,"name":"Helios — platform migration","account_id":acct(1),"stage_id":stage(1),"amount":64_000.0,"currency":"USD","status":"open","probability":25,"description":"Replacing a legacy CRM."})),
        owned(json!({"tenant_id":t,"name":"Meridian — enterprise rollout","account_id":acct(2),"stage_id":stage(2),"amount":480_000.0,"currency":"USD","status":"open","probability":50,"description":"Enterprise-wide deployment."})),
        owned(json!({"tenant_id":t,"name":"Cobalt — seat expansion","account_id":acct(3),"stage_id":stage(4),"amount":28_000.0,"currency":"USD","status":"won","probability":100,"description":"Closed last week."})),
        owned(json!({"tenant_id":t,"name":"Northwind — support renewal","account_id":acct(0),"stage_id":stage(0),"amount":42_000.0,"currency":"USD","status":"open","probability":10,"description":"Annual renewal conversation."})),
    ]);

    let _ = ins("activities", "activities", vec![
        owned(json!({"tenant_id":t,"kind":"call","subject":"Discovery call with Helios","body":"Walked through the migration plan.","related_type":"account","related_id":acct(1),"completed":true})),
        owned(json!({"tenant_id":t,"kind":"email","subject":"Sent proposal to Meridian","body":"Enterprise pricing + security docs.","related_type":"account","related_id":acct(2),"completed":true})),
        owned(json!({"tenant_id":t,"kind":"meeting","subject":"QBR with Northwind","body":"Reviewed the expansion roadmap.","related_type":"account","related_id":acct(0),"completed":false})),
        owned(json!({"tenant_id":t,"kind":"note","subject":"Cobalt expansion signal","body":"Lena mentioned ~20 new seats in Q3.","related_type":"account","related_id":acct(3),"completed":false})),
        owned(json!({"tenant_id":t,"kind":"call","subject":"Follow up with Meridian IT","body":"Confirm the security-review timeline.","related_type":"account","related_id":acct(2),"completed":false})),
    ]);

    diag.into_inner().join(" | ")
}
