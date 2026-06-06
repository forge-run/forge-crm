# Forge CRM

**An open-source CRM you actually own.** A complete sales CRM — accounts, contacts, leads, a deal pipeline, and activity tracking — shipped as **pure [Forge](https://forge.run) configuration**. Fork it, run it, rip out what you don't like, and build whatever your business actually needs. You are never locked in, because the whole application is config you hold in your own repo.

> Throw the expensive seat-licensed CRM in the garbage. Keep your data, your schema, and your roadmap.

---

## Why this exists

Every CRM you can buy is a rental. The vendor owns the data model, meters you per seat, charges a consultancy to bend it to your business, and makes leaving expensive on purpose. The lock-in isn't a side effect — it's the product.

Forge CRM inverts that:

- **You own the application.** It's declarative config — table schemas, pages, a thin operations layer — sitting in a repo you control. There is no proprietary app to be trapped inside.
- **It's infinitely customizable.** Add a field, a table, a page, a workflow. It's your schema. An agent can reshape it to your business in an afternoon (see [Customizing](#customizing-it-is-the-point)).
- **It's always supported — at the layer that matters.** Forge guarantees the *substrate* (storage, the secured multi-protocol API, row-level security, atomic deploys). You own the *app* on top. Clean boundary, no walled garden.
- **Security is compiled in.** Row-level security lives in the schema and runs on every read and write. There is no unguarded path, because there is no second path around the one Forge generates.
- **Agent-native by construction.** The same schema that serves the UI exposes an MCP tool surface — so an AI agent is a first-class operator on your pipeline, with the *same* row-level security enforced on every action.

The catch — the honest one — is that it runs on Forge. That's the deal: the app is yours and free; the engine underneath is the product. The mod is yours, the engine is ours.

---

## What's in the box

A working sales CRM data model and UI:

| Object | What it is |
|---|---|
| **Account** | A company — the organization you sell to. |
| **Contact** | A person at an account. |
| **Lead** | An unqualified prospect, convertible into a Contact + Opportunity. |
| **Opportunity** | A deal in flight, moving through pipeline stages with an amount and close date. |
| **Pipeline stage** | Your sales stages — *configurable as data*, not hard-coded (rename them, reorder them, add your own). |
| **Activity** | A logged call, email, meeting, note, or task, attached to any record. |

Every object is multi-tenant-isolated, owner-aware, audited (`created_at` / `created_by` / `last_modified_*` come free with the `Record` archetype), and reachable over REST, an MCP tool surface, and WebSocket subscriptions — all derived from the schema, none hand-written.

```
schemas/        ← the data model: one JSON file per table (the crown jewel — pure declarative config)
pages/          ← the UI: server-rendered pages (.page.json route + .page.html template + .page.css)
components/     ← reusable UI pieces (record table, pipeline column, stat tiles, app shell)
src/            ← a THIN operations layer: storage-query wrappers that pages bind to (boilerplate, no business secrets)
content/        ← optional seed data + docs
app.json        ← branding, auth providers, hosts
service.json    ← the service manifest: which tables the ops touch + the op contracts
```

---

## Quickstart

You need the [`forge` CLI](https://forge.run/docs/reference/cli) and a Forge workspace (`forge login`).

```bash
# 1. Apply the data model (creates the tables + RLS + the derived API)
for f in schemas/*.json; do forge schema apply "$f"; done

# 2. Build the thin operations layer to WASM (validates pages + components too)
forge build

# 3. Ship it — static assets upload before the deploy (edge-cache safe)
forge ship \
  --manifest service.json \
  --wasm target/wasm32-wasip1/release/forge_crm.wasm \
  --app-manifest app.json \
  --static-dir static
```

Open your workspace host and you have a CRM. Point a custom domain at it in `app.json` when you're ready.

> **Migrating off another CRM?** The data model maps cleanly onto Salesforce's core objects (Account / Contact / Lead / Opportunity / Activity). An import agent that reads a Salesforce export and lands it here is on the roadmap — see [`content/MIGRATION.md`](content/MIGRATION.md).

---

## Customizing — it *is* the point

This isn't a product with a settings page. It's a repo. To change the CRM, change the config:

- **Add a field** → add a column to the table in `schemas/`, re-apply. The API, the audit trail, and RLS pick it up automatically.
- **Add an object** → drop a new `schemas/<thing>.json`. You now have a secured CRUD API + MCP tools for it.
- **Change a screen** → edit the `.page.html` template. It's server-rendered HTML with components.
- **Change who-sees-what** → edit the `row_security` block in the schema. It compiles into every query.

Because the whole app is declarative config, **an AI agent can do all of the above for you** and redeploy — that's the intended maintenance model, not a support ticket.

---

## The security boundary (and why this repo is safe to be public)

This repo contains **only declarative configuration and boilerplate** — no secrets, ever:

- ✅ Table schemas (including `row_security` — that's the security *model*, which is meant to be readable; it's enforced server-side and contains no credentials).
- ✅ Page/component templates and CSS.
- ✅ Thin storage-wrapper operations (no API keys, no business logic worth hiding).
- ✅ App + service manifests (branding, auth provider *names*, table grants).

It contains **none** of:

- ❌ Tokens, API keys, passwords, or `.env` files.
- ❌ Your `~/.forge/config.toml` or any session/refresh credentials.
- ❌ Customer data.

Authentication, secrets, and identity are provided by the Forge substrate at runtime — not by anything in this repo. See [`.gitignore`](.gitignore) for the enforced exclusions.

---

## License

Apache License 2.0 — see [`LICENSE`](LICENSE). Use it, fork it, sell what you build on it.

---

<sub>Built on [Forge](https://forge.run) — the agent-era app platform. One schema becomes secured, multi-protocol software.</sub>
