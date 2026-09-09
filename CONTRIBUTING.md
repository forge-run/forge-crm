# Customizing Forge CRM

This isn't a product with a settings page — it's a repo. You change the CRM by changing config. Here's the map.

## Add a field

Edit the table in `domains/crm/schemas/`, add a column, redeploy:

```jsonc
// domains/crm/schemas/opportunities.table.json → columns
{ "name": "forecast_category", "type": "string", "required": false,
  "enum": ["pipeline", "best_case", "commit", "closed"], "label": "Forecast" }
```

```bash
forge build && forge ship
```

The data model is repo-owned: the deploy applies it. The derived REST/MCP/WebSocket API, the audit trail, and row-level security pick the column up automatically. Show it in the UI by referencing it in the relevant `.page.html`.

## Add an object

Drop a new `domains/crm/schemas/<thing>.table.json` (copy an existing one as a template). On the next deploy you get a secured CRUD API + MCP tools for it, tenant-isolated by the same `row_security` block. Add a page under `apps/workbench/pages/app/` to surface it, and a read op in `domains/crm/service.json` + `domains/crm/services/<Thing>.java` (copy `ListContacts.java` — it's a dozen lines).

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

Pages are server-rendered: a `.page.json` (route + auth + data bindings) and a `.page.html` (template). Edit the template; bind any data you need through an `@op:crm::<name>` operation declared in `domains/crm/service.json`.

## Let an agent do it

Because the whole application is declarative config, an AI agent can do all of the above — add a field, wire a page, tighten a policy — and redeploy. That's the intended maintenance model. The substrate guarantees the result runs; the agent owns the change; you review the diff.

## Project layout

```
workspace.json                 which app answers on which host
domains/crm/schemas/           data model (one <table>.table.json per table)   ← the durable core
domains/crm/service.json       op contracts (what the pages can read/write)
domains/crm/services/          thin storage-wrapper ops, in the Forge dialect
apps/workbench/pages/app/      the authenticated UI
apps/workbench/components/     shared UI pieces (crm_shell = the frame)
apps/workbench/app.json        branding, auth providers, hosts
```

The domain is the data and the behaviour; the app is the screens. The app
is pure configuration and compiles to no code, so a screen change never
rebuilds an operation.

## Build & ship

```bash
forge build   # compiles the workspace: schemas, ops, pages, components
forge ship    # uploads the module, pushes, waits for the converge
```

Never commit secrets — see [`.gitignore`](.gitignore). Credentials live in your Forge session, not in this repo.
