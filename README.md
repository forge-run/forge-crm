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

### The screens (operational UI)

Not a dashboard bolted onto a database — work surfaces tied to the sales lifecycle:

| Surface | Route | What you do there |
|---|---|---|
| **Dashboard** | `/app` | Open pipeline value, deal/contact/lead counts, recent activity. |
| **My work** | `/app/work` | Your open tasks & follow-ups, soonest first — the daily driver. |
| **Pipeline board** | `/app/pipeline` | Open deals grouped by your stages; move + close. |
| **Record workbench** | `/app/accounts/:id`, `/app/contacts/:id`, `/app/opportunities/:id` | The 360° view — related records, activity timeline, log-a-call composer, quick actions. |
| **Lead inbox** | `/app/leads` | Triage, qualify, and convert leads → contact + opportunity. |
| **Worklists** | `/app/{accounts,contacts,opportunities}` | Searchable, filterable lists that link into the workbench. |
| **Forecast** | `/app/forecast` | Weighted pipeline (amount × stage probability), win rate, lead→won funnel. |

Every action a human takes here is a `@op:crm::*` operation — which is **also** the MCP tool an agent calls. The human workbench and the agent operate the same surface, under the same row-level security.

```
workspace.json          ← which app answers on which host
domains/crm/
  schemas/              ← the data model: one `<table>.table.json` per table (the crown jewel — pure declarative config)
  services/             ← a THIN operations layer: storage-query wrappers the pages bind to, written in the Forge dialect (`*.java`)
  service.json          ← the op contracts: which tables the ops touch, what each one takes and returns
  domain.json           ← the bounded context's name and what it may call
apps/workbench/
  pages/                ← the UI: server-rendered pages (.page.json route + .page.html template + .page.css)
  components/           ← reusable UI pieces (record table, pipeline column, stat tiles, app shell)
  app.json              ← branding, auth providers, hosts
content/                ← optional seed data + docs
```

The **domain** owns data and behaviour; the **app** owns screens. They are
separate directories because they deploy separately: the app is pure
configuration and compiles to no code at all, so changing a screen never
rebuilds an operation.

---

## Quickstart

You need the [`forge` CLI](https://forge.run/docs/reference/cli) and a Forge workspace (`forge login`).

```bash
# 1. Compile the workspace — the data model, the ops, the pages and the
#    components, all checked before anything is uploaded
forge build

# 2. Ship it — uploads the built module, pushes, and waits for the converge.
#    The tables come with it: the data model is repo-owned and applied on
#    deploy, so there is no separate schema step.
forge ship
```

Open your workspace host and you have a CRM. Point a custom domain at it in `app.json` when you're ready.

> **Migrating off another CRM?** The data model maps cleanly onto Salesforce's core objects (Account / Contact / Lead / Opportunity / Activity). An import agent that reads a Salesforce export and lands it here is on the roadmap — see [`content/MIGRATION.md`](content/MIGRATION.md).

---

## Customizing — it *is* the point

This isn't a product with a settings page. It's a repo. To change the CRM, change the config:

- **Add a field** → add a column to the table in `domains/crm/schemas/`, redeploy. The API, the audit trail, and RLS pick it up automatically.
- **Add an object** → drop a new `domains/crm/schemas/<thing>.table.json`. You now have a secured CRUD API + MCP tools for it.
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
