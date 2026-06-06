# Deploying Forge CRM to `demos.forge.run`

Standing up the `demos` workspace on the branded `demos.forge.run` host is a
**control-plane + edge** task, not a plain `forge deploy`. Steps marked
**[operator]** need control-plane admin / DNS / edge access and are run by a
platform operator — they're listed here so the sequence is one place.

> ⚠ Never commit the CP admin token, DNS API keys, or edge SSH config. They
> live in the operator's environment, not in this repo.

## 0. Re-authenticate the CLI

```bash
forge login            # browser device flow against app.forge.run
forge whoami           # confirm subject + role
```

## 1. Create the `demos` workspace  (self-serve)

In the portal: `app.forge.run` → your tenant → **Workspaces → + Create** →
name it `demos`. Note its workspace id (`ws-…`) and default host. Then point
the CLI at it:

```bash
forge tenant use <tenant_id>
forge ws use <demos_workspace_id>
```

## 2. Claim the hostname + issue the cert  **[operator]**

```bash
export FORGE_CP_URL=...            # control-plane URL (operator env)
export FORGE_ADMIN_TOKEN=...       # operator admin token (NEVER commit)

forge domain add demos.forge.run           # → prints a DNS TXT record to add
# add the TXT record in the forge.run DNS zone (Cloudflare/registrar) [operator]
forge domain validate demos.forge.run      # polls ACME; issues + serves the cert
forge domain status demos.forge.run        # confirm: validated + cert active
```

## 3. DNS for the host  **[operator]**

In the `forge.run` zone, add `demos` → the edge (same target as
`app.forge.run` — a CNAME/A to the edge, proxied). Propagation + the
routing-table poll make the host live at the edge.

## 4. Edge routing (Caddy)  **[operator]**

Add a reverse_proxy for `demos.forge.run` → the `demos` workspace's
forge-agent, mirroring the existing `app.forge.run` block. Roll it
**surgically** (scp the edited Caddyfile + `caddy validate` + reload) — do
**not** run a full `just sync-config`, which sweeps drift and bounces the
agent fleet.

```caddyfile
demos.forge.run {
    reverse_proxy <demos-workspace-agent-upstream>
    # mirror the headers/XFH handling from the app.forge.run block
}
```

## 5. Apply the schema + ship the app

```bash
for f in schemas/*.json; do forge schema apply "$f"; done
forge build
forge ship \
  --manifest service.json \
  --wasm target/wasm32-wasip1/release/forge_crm.wasm \
  --app-manifest app.json \
  --static-dir static
```

`app.json` already claims `demos.forge.run` with the `app` + `auth` surfaces.

## 6. Verify

```bash
curl -s -o /dev/null -w "%{http_code}\n" https://demos.forge.run/app
```

Then open `https://demos.forge.run/app` and walk the workbench / pipeline /
lead inbox.

---

### Known gap before this is demo-ready

The CRM currently ships **authenticated app pages only** — there's no login
UI yet, so an unauthenticated hit to `/app` has nowhere to land. Before the
demo is usable end-to-end, add an `auth`-surface login page (the
forge-portal-web `pages/anon/login_form.*` is the reference) or front it with
the platform's shared auth. Tracked as the next build item.
