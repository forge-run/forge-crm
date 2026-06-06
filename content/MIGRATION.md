# Migrating off Salesforce

Forge CRM's data model maps cleanly onto Salesforce's core objects, so a migration is mostly a field-mapping exercise — not a re-platforming project.

## Object map

| Salesforce | Forge CRM table | Notes |
|---|---|---|
| Account | `accounts` | 1:1. `Industry`, `Type`, `AnnualRevenue`, `NumberOfEmployees` → `industry`, `type`, `annual_revenue`, `employee_count`. |
| Contact | `contacts` | 1:1. `AccountId` → `account_id` (FK). |
| Lead | `leads` | 1:1. `Status`, `LeadSource`, `Rating` → `status`, `source`, `rating`. Lead conversion → `convert_lead` op. |
| Opportunity | `opportunities` | 1:1. `StageName` → `stage_id` (FK into your own `pipeline_stages`, see below). `Amount`, `CloseDate` map directly. |
| Opportunity Stage (picklist) | `pipeline_stages` | Salesforce stages are config buried in setup; here they're **rows you own**. Seed them from your SF stage picklist. |
| Task / Event / Activity | `activities` | Unified under one table with a `kind` enum (`call`/`email`/`meeting`/`note`/`task`) and a polymorphic `related_type` + `related_id`. |
| Note / ContentNote | `activities` (`kind: "note"`) | Folded into the activity timeline. |

## Fields Salesforce has that this starter doesn't

By design — this is the 80% most teams use, not feature-parity. Add what you need: drop a column into the table schema, re-apply, done. Common additions: `accounts.parent_account_id` (hierarchies), `opportunities.forecast_category`, custom `__c` fields → just regular columns. The whole point is that extending the model is a config edit, not a vendor request.

## The import (roadmap)

The intended migration path is an **import agent**, not a CSV slog:

1. Export your Salesforce objects (Data Loader / Reports → CSV, or the Bulk API).
2. Point the import agent at the export. It maps SF fields → Forge CRM columns (the table above), reconciles the stage picklist into `pipeline_stages` rows, rewrites `AccountId`/`WhoId`/`WhatId` references to the new ids, and writes through the CRM's own create ops (so RLS + audit apply).
3. Review the diff, deploy, point your domain at it.

Because every write goes through the same secured ops your UI uses, the import can't bypass row-level security or land malformed rows — the substrate enforces the contract on the migration exactly as it does on a user.

> Until the import agent ships, you can bulk-load via the schema-derived REST API (`POST /api/v1/accounts`, etc.) or `storage::mutate` from a one-off script. Keep that script out of this repo (it'll carry credentials).
