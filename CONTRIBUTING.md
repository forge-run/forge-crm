# Customizing Forge CRM

This isn't a product with a settings page — it's a repo. You change the CRM by changing config. Here's the map.

## Add a field

Edit the table in `schemas/`, add a column, re-apply:

```jsonc
// schemas/opportunity.json → columns
{ "name": "forecast_category", "type": "string", "required": false,
  "enum": ["pipeline", "best_case", "commit", "closed"], "label": "Forecast" }
```

```bash
forge schema apply schemas/opportunity.json
```

The derived REST/MCP/WebSocket API, the audit trail, and row-level security pick it up automatically. Show it in the UI by referencing it in the relevant `.page.html`.

## Add an object

Drop a new `schemas/<thing>.json` (copy an existing one as a template). On `forge schema apply` you get a secured CRUD API + MCP tools for it, tenant-isolated by the same `row_security` block. Add a page under `pages/app/` to surface it, and a read op in `service.json` + `src/data_ops.rs` (copy `list_contacts` — it's ~6 lines).

## Change who-sees-what

Every table's visibility lives in its `row_security` block. The starter isolates by tenant:

```json
"row_security": [
  { "name": "tenant_isolation",
    "filter": { "column": "tenant_id", "op": "eq", "value": "$claims.tenant_id" },
    "scopes": ["select", "insert", "update", "delete"] }
]
```

Want reps to see only their own accounts? Add an owner policy:

```json
{ "name": "own_records", "filter": { "column": "owner_id", "op": "eq", "value": "$user_id" },
  "scopes": ["select", "update", "delete"] }
```

It compiles into every query — there's no app code to keep in sync, and no path around it.

## Change a screen

Pages are server-rendered: a `.page.json` (route + auth + data bindings) and a `.page.html` (template). Edit the template; bind any data you need through an `@op:crm::<name>` operation declared in `service.json`.

## Let an agent do it

Because the whole application is declarative config, an AI agent can do all of the above — add a field, wire a page, tighten a policy — and redeploy. That's the intended maintenance model. The substrate guarantees the result runs; the agent owns the change; you review the diff.

## Project layout

```
schemas/      data model (one JSON per table)        ← the durable core
service.json  op contracts (what the pages can read/write)
src/          thin storage-wrapper ops (boilerplate)
pages/app/    the authenticated UI
components/   shared UI pieces (crm_shell = the frame)
app.json      branding, auth providers, hosts
```

## Build & ship

```bash
for f in schemas/*.json; do forge schema apply "$f"; done
forge build
forge ship --manifest service.json --wasm target/wasm32-wasip1/release/forge_crm.wasm --app-manifest app.json --static-dir static
```

Never commit secrets — see [`.gitignore`](.gitignore). Credentials live in your Forge session, not in this repo.
