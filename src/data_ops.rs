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

fn s<'a>(input: &'a Value, key: &str) -> Option<&'a str> {
    input.get(key).and_then(|v| v.as_str()).filter(|s| !s.is_empty())
}

/// (tenant_id, owner_id) for a write. tenant_id arrives via `$auth:tenant_id`
/// (runtime-substituted, RLS-guarded); owner_id is the authenticated caller.
fn write_scope(ctx: &OpContext, input: &Value) -> Result<(String, String), OpError> {
    let tenant = s(input, "tenant_id")
        .ok_or_else(|| OpError::Forbidden)?
        .to_string();
    let owner = ctx.user_id.clone();
    Ok((tenant, owner))
}

fn one(rows: &[Value], id: &str) -> Value {
    rows.iter()
        .find(|r| r.get("id").and_then(|v| v.as_str()) == Some(id))
        .cloned()
        .unwrap_or(Value::Null)
}

// ── queries ────────────────────────────────────────────────────

pub fn dashboard_summary(_ctx: &OpContext, _input: &Value) -> Result<Value, OpError> {
    let open = rows(&query(json!({
        "from": "opportunities",
        "filter": { "column": "status", "op": "eq", "value": "open" },
        "select": ["id", "amount"],
        "limit": 1000
    }))?);
    let pipeline_value: f64 = open
        .iter()
        .filter_map(|o| o.get("amount").and_then(|a| a.as_f64()))
        .sum();
    let won = rows(&query(json!({
        "from": "opportunities",
        "filter": { "column": "status", "op": "eq", "value": "won" },
        "select": ["id"],
        "limit": 1000
    }))?);
    let contacts = rows(&query(json!({ "from": "contacts", "select": ["id"], "limit": 5000 }))?);
    let leads_open = rows(&query(json!({
        "from": "leads",
        "filter": { "column": "converted", "op": "eq", "value": false },
        "select": ["id"],
        "limit": 5000
    }))?);
    let recent = rows(&query(json!({
        "from": "activities",
        "sort": [{ "column": "created_at", "direction": "desc" }],
        "limit": 8
    }))?);

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
