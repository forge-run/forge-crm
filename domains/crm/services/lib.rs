//! The CRM bounded context — the ops the app's pages bind to.
//!
//! The application is declarative: tables in `schemas/`, UI in
//! `apps/crm/pages` + `apps/crm/components`. This crate exists only because
//! page-data bindings resolve through `@op:crm::<name>` operations — so each
//! page-read and each mutation is a small handler that wraps
//! `storage::query` / `storage::mutate`. There is no business logic worth
//! hiding: row-level security (declared in the schemas) does the enforcement;
//! these handlers just shape payloads.
//!
//! **The handlers are written in the forge-lang dialect** (`services/*.java`),
//! not in Rust. `services/jgen` is their emitted Rust, committed so a plain
//! `cargo build` needs no transpiler; regenerate it in the SAME commit as any
//! `services/` change with
//!
//! ```text
//! forge-lang build domains/crm/services/*.java \
//!   --module-out domains/crm/services/jgen --source-root .
//! ```
//!
//! ALL twenty-two ops are dialect sources as of HS-9, and `src/data_ops.rs`
//! is gone. The four that HS-6 could not port — `dashboard_summary`,
//! `pipeline_board`, `forecast`, `create_opportunity` — were blocked on one
//! missing band entry, a float read out of a JSON value, and each of them
//! sums or scales a `number` column. `GetKind::Float` landed in forge-lang
//! and they moved.
//!
//! What did NOT move is [`seed`], the demo-workspace bootstrap that lived
//! under `dashboard_summary`. It is not an op — nothing dispatches it by
//! name — and it swallows an error per insert, builds a diagnostic by
//! mutation, and reads `ctx.tenant_id`; the dialect has no `try` (`FL2034`)
//! and no `tenant_id` on its context. So the HOST crate runs it, from this
//! file, immediately before dispatching `dashboard_summary` — exactly where
//! the Rust op ran it — and hands the diagnostic to the dialect op as an
//! input field, which the op echoes. Its value and its position on the wire
//! are unchanged. It is also why this domain keeps a hand-written
//! `services/lib.rs` (the MIXED shape) rather than a wholly emitted crate.
//!
//! **This module is HEADLESS**, like the portal's domain crates: the runtime
//! re-invokes this wasm with `ctx.op_name` set to the bound op, the op
//! returns raw JSON, and the substrate lifts it into `props.data.<binding>`.
//! Until HS-10 this crate ALSO rendered the app's routes — it carried
//! `forge_web::dispatch_with` and an `include!` of the declarative-page
//! registration a root `build.rs` emitted into `OUT_DIR`. In the domain/app
//! layout the pages are walked by forge-web's workspace compiler and shipped
//! as manifest rows the substrate renders, so the page half of this file, its
//! `forge-web` / `forge-schema` dependencies and the `build.rs` that fed it
//! are all gone. An op name this module does not own is now a refusal, not a
//! fallthrough to a page.

mod jgen;

#[cfg(target_arch = "wasm32")]
mod seed;

// The emitted module's ops spell `crate::fault::...` exactly as they do in
// an all-dialect crate; this re-export is what resolves it here.
pub(crate) use jgen::fault;

#[cfg(target_arch = "wasm32")]
mod component_export {
    use forge_sdk_v2::{Guest, OpError, OpInput, OpOutput};
    use serde_json::Value;

    pub struct Crm;

    impl Guest for Crm {
        fn handle(input: OpInput) -> Result<OpOutput, OpError> {
            let _parsed: Value = serde_json::from_str(&input.json)
                .map_err(|e| OpError::BadRequest(format!("input is not valid JSON: {e}")))?;
            let ctx = forge_sdk_v2::op_context()
                .map_err(|e| OpError::Internal(format!("op_context unavailable: {e}")))?;

            // `dashboard_summary` is the one op with a side effect that is
            // not its own: the first-load bootstrap. The Rust op ran it
            // before its reads and reported the result as `seed_diag`; the
            // host runs it here, in the same place, and hands the answer to
            // the dialect op on its input. The op read no other field —
            // its Rust signature took `_input` — so a fresh object carrying
            // only `seed_diag` is what it took before, spelled exactly.
            let input_json = if ctx.op_name == "dashboard_summary" {
                let diag = crate::seed::seed_if_empty(&ctx);
                serde_json::to_string(&serde_json::json!({ "seed_diag": diag }))
                    .map_err(|e| OpError::Internal(format!("encode: {e}")))?
            } else {
                input.json.clone()
            };

            // Every op is a dialect op. A name this domain does not own is
            // a refusal: routes are the substrate's job, not this wasm's.
            let json = match crate::jgen::dispatch(
                ctx.op_name.as_str(),
                &input_json,
                ctx.user_id.clone(),
            ) {
                Some(r) => r?,
                None => {
                    return Err(OpError::BadRequest(format!(
                        "unknown op `{}` — the crm domain serves only the ops in its service.json",
                        ctx.op_name
                    )));
                }
            };
            Ok(OpOutput { json })
        }
    }

    forge_sdk_v2::export!(Crm with_types_in forge_sdk_v2);
}
