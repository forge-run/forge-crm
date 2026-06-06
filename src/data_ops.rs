//! Forge CRM data operations — thin wrappers over `storage::query` /
//! `storage::mutate`. Two rules keep these honest and safe:
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
//!
//! Everything else is payload shaping. There is no secret here.

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

fn one(rows: &[Value], id: &str) -> Value {
    rows.iter()
        .find(|r| r.get("id").and_then(|v| v.as_str()) == Some(id))
        .cloned()
        .unwrap_or(Value::Null)
}

// ── queries ────────────────────────────────────────────────────

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
/// user. Everything is scoped to the caller's `tenant_id`. Each demo user
/// (each signup mints its own tenant) gets an isolated copy they can edit
/// freely. Fully resilient: every step continues on error, so one bad insert
/// never aborts the rest or blocks the dashboard render.
///
/// `tenant_id` is supplied by the caller (resolved from the trusted session,
/// not the browser) and stamped onto every row. It must be a real UUID: the
/// column is `format: uuid` + `required`, so an empty/blank value fails
/// validation and the insert commits nothing.
fn seed_demo_data(tenant: &str) -> String {
    let diag = std::cell::RefCell::new(Vec::<String>::new());
    let t = tenant;
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
            Err(_) => {
                // Best-effort: record and continue so one failed insert never
                // aborts the rest of the seed or blocks the dashboard render.
                diag.borrow_mut().push(format!("{label}:ERR"));
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

pub fn dashboard_summary(ctx: &OpContext, input: &Value) -> Result<Value, OpError> {
    // First-load seed: this is the demo workspace, so every user gets a
    // private, pre-populated CRM. Guarded on `pipeline_stages` (the first
    // thing seeded) so it runs exactly once per tenant even if a later step
    // fails. Best-effort — never blocks the dashboard render.
    //
    // This op is the dashboard's first-load data binding, so two things have to
    // line up for the seed write to land:
    //   1. the binding must address this op by its **service name** —
    //      `@op:workspace::dashboard_summary`, not the namespace
    //      (`@op:crm::…`). A wrong op-path silently resolves to the binding's
    //      `default`, so the op (and its seed) never runs at all.
    //   2. it must be a `mutation`-kind op — `query`-kind ops get a read-only
    //      storage handle, so the seed's inserts are silently dropped.
    //   3. every row needs a real `tenant_id` (the column is `format: uuid`,
    //      `required`). We take it from the trusted session: `ctx.tenant_id`
    //      first, falling back to the `$auth:tenant_id` binding param. If
    //      neither resolves (caller has no tenant yet) the seed safely no-ops
    //      rather than writing an empty string that fails uuid validation.
    let tenant = ctx
        .tenant_id
        .clone()
        .filter(|t| !t.is_empty())
        .or_else(|| s(input, "tenant_id").map(str::to_string));
    if let Some(tenant) = tenant {
        if qrows(json!({ "from": "pipeline_stages", "select": ["id"], "limit": 1 })).is_empty() {
            let _ = seed_demo_data(&tenant);
        }
    }
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
        "recent_activities": recent
    }))
}

pub fn list_accounts(_ctx: &OpContext, input: &Value) -> Result<Value, OpError> {
    let mut spec = json!({
        "from": "accounts",
        "sort": [{ "column": "created_at", "direction": "desc" }],
        "limit": 200
    });
    if let Some(term) = s(input, "search") {
        spec["filter"] = json!({ "column": "name", "op": "ilike", "value": format!("%{term}%") });
    }
    let r = rows(&query(spec)?);
    Ok(json!({ "total": r.len(), "rows": r }))
}

pub fn list_contacts(_ctx: &OpContext, input: &Value) -> Result<Value, OpError> {
    let mut filters: Vec<Value> = Vec::new();
    if let Some(acct) = s(input, "account_id") {
        filters.push(json!({ "column": "account_id", "op": "eq", "value": acct }));
    }
    if let Some(term) = s(input, "search") {
        filters.push(json!({ "column": "last_name", "op": "ilike", "value": format!("%{term}%") }));
    }
    let mut spec = json!({
        "from": "contacts",
        "sort": [{ "column": "created_at", "direction": "desc" }],
        "limit": 200,
        "follow": [{ "relationship": "account_id", "select": ["id", "name"] }]
    });
    match filters.len() {
        0 => {}
        1 => spec["filter"] = filters.remove(0),
        _ => spec["filter"] = json!({ "and": filters }),
    }
    let r = rows(&query(spec)?);
    Ok(json!({ "total": r.len(), "rows": r }))
}

pub fn get_contact(_ctx: &OpContext, input: &Value) -> Result<Value, OpError> {
    let id = s(input, "id").ok_or_else(|| OpError::BadRequest("id required".into()))?;
    let contact = rows(&query(json!({
        "from": "contacts",
        "filter": { "column": "id", "op": "eq", "value": id },
        "limit": 1
    }))?)
    .into_iter()
    .next()
    .ok_or(OpError::NotFound)?;

    let account = match contact.get("account_id").and_then(|v| v.as_str()) {
        Some(aid) if !aid.is_empty() => rows(&query(json!({
            "from": "accounts",
            "filter": { "column": "id", "op": "eq", "value": aid },
            "limit": 1
        }))?)
        .into_iter()
        .next()
        .unwrap_or(Value::Null),
        _ => Value::Null,
    };

    let activities = rows(&query(json!({
        "from": "activities",
        "filter": { "and": [
            { "column": "related_type", "op": "eq", "value": "contact" },
            { "column": "related_id", "op": "eq", "value": id }
        ]},
        "sort": [{ "column": "created_at", "direction": "desc" }],
        "limit": 50
    }))?);

    Ok(json!({ "contact": contact, "account": account, "activities": activities }))
}

pub fn list_leads(_ctx: &OpContext, input: &Value) -> Result<Value, OpError> {
    let mut spec = json!({
        "from": "leads",
        "filter": { "column": "converted", "op": "eq", "value": false },
        "sort": [{ "column": "created_at", "direction": "desc" }],
        "limit": 200
    });
    if let Some(status) = s(input, "status") {
        spec["filter"] = json!({ "and": [
            { "column": "converted", "op": "eq", "value": false },
            { "column": "status", "op": "eq", "value": status }
        ]});
    }
    let r = rows(&query(spec)?);
    Ok(json!({ "total": r.len(), "rows": r }))
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

pub fn create_account(ctx: &OpContext, input: &Value) -> Result<Value, OpError> {
    let name = s(input, "name").ok_or_else(|| OpError::BadRequest("name required".into()))?;
    let mut row = json!({ "name": name });
    for k in ["domain", "website", "industry", "type", "phone"] {
        if let Some(v) = s(input, k) { row[k] = json!(v); }
    }
    insert("accounts", row, ctx, input)
}

pub fn create_contact(ctx: &OpContext, input: &Value) -> Result<Value, OpError> {
    let first = s(input, "first_name").ok_or_else(|| OpError::BadRequest("first_name required".into()))?;
    let mut row = json!({ "first_name": first });
    for k in ["last_name", "email", "phone", "title", "account_id"] {
        if let Some(v) = s(input, k) { row[k] = json!(v); }
    }
    insert("contacts", row, ctx, input)
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

pub fn log_activity(ctx: &OpContext, input: &Value) -> Result<Value, OpError> {
    let subject = s(input, "subject").ok_or_else(|| OpError::BadRequest("subject required".into()))?;
    let mut row = json!({ "subject": subject, "kind": s(input, "kind").unwrap_or("note") });
    for k in ["body", "related_type", "related_id", "due_at"] {
        if let Some(v) = s(input, k) { row[k] = json!(v); }
    }
    insert("activities", row, ctx, input)
}

pub fn move_opportunity(_ctx: &OpContext, input: &Value) -> Result<Value, OpError> {
    let id = s(input, "id").ok_or_else(|| OpError::BadRequest("id required".into()))?;
    let stage_id = s(input, "stage_id").ok_or_else(|| OpError::BadRequest("stage_id required".into()))?;
    // The update is RLS-scoped to the caller's tenant — they can only move
    // their own opportunities.
    let res = mutate(json!({
        "table": "opportunities",
        "op": "update",
        "filter": { "column_equals": { "column": "id", "value": id } },
        "patch": { "stage_id": stage_id }
    }))?;
    Ok(json!({ "committed": res.get("committed").and_then(|v| v.as_i64()).unwrap_or(0) }))
}

/// Convert a qualified lead into a contact (and optionally an opportunity),
/// then mark the lead converted. A small multi-step write — the kind of thing
/// that justifies an op over the raw auto-CRUD API.
pub fn convert_lead(ctx: &OpContext, input: &Value) -> Result<Value, OpError> {
    let id = s(input, "id").ok_or_else(|| OpError::BadRequest("id required".into()))?;
    let lead = rows(&query(json!({
        "from": "leads",
        "filter": { "column": "id", "op": "eq", "value": id },
        "limit": 1
    }))?)
    .into_iter()
    .next()
    .ok_or(OpError::NotFound)?;

    let get = |k: &str| lead.get(k).and_then(|v| v.as_str()).unwrap_or("").to_string();

    // 1. contact from the lead's person fields
    let contact_row = json!({
        "first_name": get("first_name"),
        "last_name": get("last_name"),
        "email": get("email"),
        "phone": get("phone"),
        "title": get("title")
    });
    let contact = insert("contacts", contact_row, ctx, input)?;
    let contact_id = contact.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string();

    // 2. optional opportunity
    let opportunity_id = if input.get("create_opportunity").and_then(|v| v.as_bool()).unwrap_or(false) {
        let company = get("company");
        let name = if company.is_empty() { format!("{} opportunity", get("first_name")) } else { format!("{company} opportunity") };
        let opp = insert("opportunities", json!({ "name": name, "status": "open" }), ctx, input)?;
        opp.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string()
    } else {
        String::new()
    };

    // 3. mark the lead converted
    let _ = mutate(json!({
        "table": "leads",
        "op": "update",
        "filter": { "column_equals": { "column": "id", "value": id } },
        "patch": { "converted": true, "converted_contact_id": contact_id, "status": "qualified" }
    }))?;

    let _ = &one; // (helper kept for future joins)
    Ok(json!({ "contact_id": contact_id, "account_id": "", "opportunity_id": opportunity_id }))
}

// ── operational reads (workbench / lists / worklist / forecast) ──

fn by_id(table: &str, id: &str) -> Result<Value, OpError> {
    Ok(rows(&query(json!({
        "from": table,
        "filter": { "column": "id", "op": "eq", "value": id },
        "limit": 1
    }))?)
    .into_iter()
    .next()
    .unwrap_or(Value::Null))
}

fn related(table: &str, col: &str, id: &str, limit: i64) -> Result<Vec<Value>, OpError> {
    Ok(rows(&query(json!({
        "from": table,
        "filter": { "column": col, "op": "eq", "value": id },
        "sort": [{ "column": "created_at", "direction": "desc" }],
        "limit": limit
    }))?))
}

fn timeline(related_type: &str, related_id: &str) -> Result<Vec<Value>, OpError> {
    Ok(rows(&query(json!({
        "from": "activities",
        "filter": { "and": [
            { "column": "related_type", "op": "eq", "value": related_type },
            { "column": "related_id", "op": "eq", "value": related_id }
        ]},
        "sort": [{ "column": "created_at", "direction": "desc" }],
        "limit": 50
    }))?))
}

/// Account 360° — the workbench. Account + its contacts + its opportunities +
/// its activity timeline, in one payload.
pub fn get_account(_ctx: &OpContext, input: &Value) -> Result<Value, OpError> {
    let id = s(input, "id").ok_or_else(|| OpError::BadRequest("id required".into()))?;
    let account = by_id("accounts", id)?;
    if account.is_null() { return Err(OpError::NotFound); }
    Ok(json!({
        "account": account,
        "contacts": related("contacts", "account_id", id, 200)?,
        "opportunities": related("opportunities", "account_id", id, 200)?,
        "activities": timeline("account", id)?
    }))
}

/// Opportunity 360° — the deal workbench. Opp + its account + the full stage
/// list (for the move/close control) + its activity timeline.
pub fn get_opportunity(_ctx: &OpContext, input: &Value) -> Result<Value, OpError> {
    let id = s(input, "id").ok_or_else(|| OpError::BadRequest("id required".into()))?;
    let opp = by_id("opportunities", id)?;
    if opp.is_null() { return Err(OpError::NotFound); }
    let account = match opp.get("account_id").and_then(|v| v.as_str()) {
        Some(a) if !a.is_empty() => by_id("accounts", a)?,
        _ => Value::Null,
    };
    let stages = rows(&query(json!({
        "from": "pipeline_stages",
        "sort": [{ "column": "position", "direction": "asc" }],
        "limit": 50
    }))?);
    Ok(json!({ "opportunity": opp, "account": account, "stages": stages, "activities": timeline("opportunity", id)? }))
}

pub fn list_opportunities(_ctx: &OpContext, input: &Value) -> Result<Value, OpError> {
    let mut spec = json!({
        "from": "opportunities",
        "sort": [{ "column": "close_date", "direction": "asc" }],
        "follow": [{ "relationship": "account_id", "select": ["id", "name"] }],
        "limit": 300
    });
    if let Some(status) = s(input, "status") {
        spec["filter"] = json!({ "column": "status", "op": "eq", "value": status });
    }
    let r = rows(&query(spec)?);
    Ok(json!({ "total": r.len(), "rows": r }))
}

/// The caller's open work — incomplete activities they own, soonest first.
/// (No server clock dependency; the page can bucket by `due_at` client-side.)
pub fn my_work(ctx: &OpContext, _input: &Value) -> Result<Value, OpError> {
    let owner = ctx.user_id.clone();
    let r = rows(&query(json!({
        "from": "activities",
        "filter": { "and": [
            { "column": "completed", "op": "eq", "value": false },
            { "column": "owner_id", "op": "eq", "value": owner }
        ]},
        "sort": [{ "column": "due_at", "direction": "asc" }],
        "limit": 200
    }))?);
    Ok(json!({ "rows": r, "total": r.len() }))
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

// ── operational mutations (edit forms / quick actions) ──────────

/// Apply a client `patch` to one row by id, RLS-scoped to the caller's tenant.
/// Protected columns (tenant_id, id, audit) are stripped so a patch can never
/// re-tenant a row or forge its identity.
fn patch_table(table: &str, input: &Value) -> Result<Value, OpError> {
    let id = s(input, "id").ok_or_else(|| OpError::BadRequest("id required".into()))?;
    let mut patch = input.get("patch").cloned().unwrap_or(json!({}));
    if let Some(obj) = patch.as_object_mut() {
        for protected in ["id", "tenant_id", "created_at", "created_by", "last_modified_at", "last_modified_by"] {
            obj.remove(protected);
        }
        if obj.is_empty() {
            return Err(OpError::BadRequest("empty patch".into()));
        }
    }
    let res = mutate(json!({
        "table": table,
        "op": "update",
        "filter": { "column_equals": { "column": "id", "value": id } },
        "patch": patch
    }))?;
    Ok(json!({ "committed": res.get("committed").and_then(|v| v.as_i64()).unwrap_or(0) }))
}

pub fn update_account(_ctx: &OpContext, input: &Value) -> Result<Value, OpError> { patch_table("accounts", input) }
pub fn update_contact(_ctx: &OpContext, input: &Value) -> Result<Value, OpError> { patch_table("contacts", input) }
pub fn update_opportunity(_ctx: &OpContext, input: &Value) -> Result<Value, OpError> { patch_table("opportunities", input) }
pub fn update_lead(_ctx: &OpContext, input: &Value) -> Result<Value, OpError> { patch_table("leads", input) }

/// Close a deal won or lost — a one-click operational action.
pub fn close_opportunity(_ctx: &OpContext, input: &Value) -> Result<Value, OpError> {
    let id = s(input, "id").ok_or_else(|| OpError::BadRequest("id required".into()))?;
    let outcome = s(input, "outcome").unwrap_or("won");
    let outcome = if outcome == "lost" { "lost" } else { "won" };
    let res = mutate(json!({
        "table": "opportunities",
        "op": "update",
        "filter": { "column_equals": { "column": "id", "value": id } },
        "patch": { "status": outcome }
    }))?;
    Ok(json!({ "committed": res.get("committed").and_then(|v| v.as_i64()).unwrap_or(0) }))
}
