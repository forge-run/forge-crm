//! Forge CRM data operations — the FOUR ops the dialect cannot yet spell.
//!
//! The other eighteen live in `ops/*.java` and serve from the emitted
//! `src/jgen` module. These four are blocked on ONE missing band entry: the
//! dialect has no float read out of a JSON value. `GetKind` is
//! Str/Int/Bool/IntOrNone/List/Dict on every band, and a `.get(k, 0.0)` is
//! refused (`FL2033`: "a `double` `.get` default — not yet in the Java
//! subset"). Each of the four sums or scales a `number` column:
//!
//!   * `dashboard_summary` — sums `opportunities.amount`
//!   * `pipeline_board`    — sums `opportunities.amount` per stage
//!   * `forecast`          — `amount` x `pipeline_stages.probability`
//!   * `create_opportunity`— reads `amount` off the input as a float
//!
//! Filed as `HS-9` in the host-surface roadmap. Nothing else here is a
//! judgement about raw Rust: the moment a float read lands, these port the
//! same way the other eighteen did.
//!
//! The two rules that keep them honest and safe are unchanged:
//!
//!  1. **Reads need no tenant filter.** Each table's `row_security`
//!     (`tenant_id == $claims.tenant_id`) is compiled into every query by the
//!     runtime, so a `query` here returns only the caller's tenant's rows.
//!
//!  2. **Writes stamp `tenant_id` + `owner_id` from the trusted session.**
//!     Pages pass them via the `$auth:tenant_id` / `$auth:user_id` binding
//!     placeholders, which the runtime substitutes server-side (never from the
//!     browser). The schema's `insert` row-security policy then rejects any row
//!     whose `tenant_id` doesn't match the caller's claim — so isolation holds
//!     even if an op is hit directly. `owner_id` is also taken from `ctx`.

#![cfg(target_arch = "wasm32")]

use forge_sdk_v2::{OpContext, OpError};
use serde_json::{json, Value};

// ── helpers ────────────────────────────────────────────────────

fn query(spec: Value) -> Result<Value, OpError> {
    forge_sdk_v2::storage::query(&spec).map_err(|e| OpError::Internal(format!("query: {e:?}")))
}

fn mutate(spec: Value) -> Result<Value, OpError> {
    forge_sdk_v2::storage::mutate(&spec).map_err(|e| OpError::Internal(format!("mutate: {e:?}")))
}

fn rows(v: &Value) -> Vec<Value> {
    v.get("rows").and_then(|r| r.as_array()).cloned().unwrap_or_default()
}

/// Best-effort query → rows, swallowing errors to an empty Vec. Used on the
/// dashboard so one failing aggregate never errors the whole summary (which
/// would drop us to the page's static default).
fn qrows(spec: Value) -> Vec<Value> {
    query(spec).map(|v| rows(&v)).unwrap_or_default()
}

fn s<'a>(input: &'a Value, key: &str) -> Option<&'a str> {
    input.get(key).and_then(|v| v.as_str()).filter(|s| !s.is_empty())
}

/// (tenant_id, owner_id) for a write. tenant_id arrives via `$auth:tenant_id`
/// (runtime-substituted, RLS-guarded); owner_id is the authenticated caller.
fn write_scope(ctx: &OpContext, input: &Value) -> Result<(String, String), OpError> {
    let tenant = s(input, "tenant_id")
        .ok_or_else(|| OpError::Forbidden)?
        .to_string();
    let owner = ctx.user_id.clone().ok_or(OpError::Forbidden)?;
    Ok((tenant, owner))
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

pub fn dashboard_summary(ctx: &OpContext, _input: &Value) -> Result<Value, OpError> {
    // First-load seed: this is the demo workspace, so every user gets a
    // private, pre-populated CRM. Guarded on `pipeline_stages` (the first
    // thing seeded) so it runs exactly once per tenant even if a later
    // step fails. Best-effort — never blocks the dashboard render.
    let seed_diag = if qrows(json!({ "from": "pipeline_stages", "select": ["id"], "limit": 1 })).is_empty() {
        seed_demo_data(ctx)
    } else {
        "skipped".to_string()
    };
    let open = qrows(json!({
        "from": "opportunities",
        "filter": { "column": "status", "op": "eq", "value": "open" },
        "select": ["id", "amount"],
        "limit": 1000
    }));
    let pipeline_value: f64 = open
        .iter()
        .filter_map(|o| o.get("amount").and_then(|a| a.as_f64()))
        .sum();
    let won = qrows(json!({
        "from": "opportunities",
        "filter": { "column": "status", "op": "eq", "value": "won" },
        "select": ["id"],
        "limit": 1000
    }));
    let contacts = qrows(json!({ "from": "contacts", "select": ["id"], "limit": 5000 }));
    let leads_open = qrows(json!({
        "from": "leads",
        "filter": { "column": "converted", "op": "eq", "value": false },
        "select": ["id"],
        "limit": 5000
    }));
    let recent = qrows(json!({
        "from": "activities",
        "sort": [{ "column": "created_at", "direction": "desc" }],
        "limit": 8
    }));

    Ok(json!({
        "open_opportunities": open.len(),
        "open_pipeline_value": pipeline_value,
        "won_this_period": won.len(),
        "contacts_total": contacts.len(),
        "leads_open": leads_open.len(),
        "recent_activities": recent,
        "seed_diag": seed_diag
    }))
}

pub fn pipeline_board(_ctx: &OpContext, _input: &Value) -> Result<Value, OpError> {
    let stages = rows(&query(json!({
        "from": "pipeline_stages",
        "sort": [{ "column": "position", "direction": "asc" }],
        "limit": 50
    }))?);
    let opps = rows(&query(json!({
        "from": "opportunities",
        "filter": { "column": "status", "op": "eq", "value": "open" },
        "sort": [{ "column": "close_date", "direction": "asc" }],
        "follow": [{ "relationship": "account_id", "select": ["id", "name"] }],
        "limit": 1000
    }))?);

    let columns: Vec<Value> = stages
        .iter()
        .map(|stage| {
            let sid = stage.get("id").and_then(|v| v.as_str()).unwrap_or("");
            let in_stage: Vec<Value> = opps
                .iter()
                .filter(|o| o.get("stage_id").and_then(|v| v.as_str()) == Some(sid))
                .cloned()
                .collect();
            let total: f64 = in_stage
                .iter()
                .filter_map(|o| o.get("amount").and_then(|a| a.as_f64()))
                .sum();
            json!({ "stage": stage, "opportunities": in_stage, "total_value": total, "count": in_stage.len() })
        })
        .collect();

    Ok(json!({ "columns": columns }))
}

// ── mutations ──────────────────────────────────────────────────

fn insert(table: &str, mut row: Value, ctx: &OpContext, input: &Value) -> Result<Value, OpError> {
    let (tenant, owner) = write_scope(ctx, input)?;
    if let Some(obj) = row.as_object_mut() {
        obj.insert("tenant_id".into(), json!(tenant));
        obj.insert("owner_id".into(), json!(owner));
    }
    let res = mutate(json!({ "table": table, "op": "insert", "rows": [row] }))?;
    let id = res
        .get("ids")
        .and_then(|v| v.as_array())
        .and_then(|a| a.first())
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let committed = res.get("committed").and_then(|v| v.as_i64()).unwrap_or(0);
    Ok(json!({ "id": id, "committed": committed }))
}

pub fn create_opportunity(ctx: &OpContext, input: &Value) -> Result<Value, OpError> {
    let name = s(input, "name").ok_or_else(|| OpError::BadRequest("name required".into()))?;
    let mut row = json!({ "name": name });
    for k in ["account_id", "stage_id", "close_date", "currency"] {
        if let Some(v) = s(input, k) { row[k] = json!(v); }
    }
    if let Some(amt) = input.get("amount").and_then(|v| v.as_f64()) { row["amount"] = json!(amt); }
    insert("opportunities", row, ctx, input)
}

/// Manager forecast — weighted open pipeline (amount × stage probability),
/// per-stage rollup, win rate, and the lead→won funnel.
pub fn forecast(_ctx: &OpContext, _input: &Value) -> Result<Value, OpError> {
    let stages = rows(&query(json!({
        "from": "pipeline_stages",
        "sort": [{ "column": "position", "direction": "asc" }],
        "limit": 50
    }))?);
    let open = rows(&query(json!({
        "from": "opportunities",
        "filter": { "column": "status", "op": "eq", "value": "open" },
        "select": ["id", "amount", "stage_id"],
        "limit": 2000
    }))?);

    let mut weighted_total = 0.0_f64;
    let by_stage: Vec<Value> = stages.iter().map(|st| {
        let sid = st.get("id").and_then(|v| v.as_str()).unwrap_or("");
        let prob = st.get("probability").and_then(|v| v.as_f64()).unwrap_or(0.0) / 100.0;
        let mut raw = 0.0_f64;
        let mut n = 0i64;
        for o in &open {
            if o.get("stage_id").and_then(|v| v.as_str()) == Some(sid) {
                raw += o.get("amount").and_then(|a| a.as_f64()).unwrap_or(0.0);
                n += 1;
            }
        }
        let weighted = raw * prob;
        weighted_total += weighted;
        json!({ "stage": st.get("name").cloned().unwrap_or(json!("")), "count": n, "raw_value": raw, "weighted_value": weighted })
    }).collect();

    let won = rows(&query(json!({ "from": "opportunities", "filter": { "column": "status", "op": "eq", "value": "won" }, "select": ["id"], "limit": 5000 }))?).len() as i64;
    let lost = rows(&query(json!({ "from": "opportunities", "filter": { "column": "status", "op": "eq", "value": "lost" }, "select": ["id"], "limit": 5000 }))?).len() as i64;
    let win_rate = if won + lost > 0 { (won as f64) / ((won + lost) as f64) } else { 0.0 };

    let leads_total = rows(&query(json!({ "from": "leads", "select": ["id"], "limit": 10000 }))?).len() as i64;
    let leads_qualified = rows(&query(json!({ "from": "leads", "filter": { "column": "status", "op": "eq", "value": "qualified" }, "select": ["id"], "limit": 10000 }))?).len() as i64;

    Ok(json!({
        "weighted_total": weighted_total,
        "by_stage": by_stage,
        "won": won,
        "lost": lost,
        "win_rate": win_rate,
        "funnel": { "leads": leads_total, "qualified": leads_qualified, "open_opportunities": open.len(), "won": won }
    }))
}
