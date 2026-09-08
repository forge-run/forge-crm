//! Forge CRM — the thin operations layer.
//!
//! The application is declarative: tables in `schemas/`, UI in `pages/` +
//! `components/`. This crate exists only because page-data bindings resolve
//! through `@op:crm::<name>` operations — so each page-read and each mutation
//! is a small handler that wraps `storage::query` / `storage::mutate`.
//! There is no business logic worth hiding: row-level security (declared in
//! the schemas) does the enforcement; these handlers just shape payloads.
//!
//! **The handlers are written in the forge-lang dialect** (`ops/*.java`), not
//! in Rust. `src/jgen` is their emitted Rust, committed so a plain
//! `cargo build` needs no transpiler; regenerate it in the SAME commit as any
//! `ops/` change with
//!
//! ```text
//! forge-lang build ops/*.java --module-out src/jgen --source-root .
//! ```
//!
//! Four ops are still raw Rust in [`data_ops`], blocked on ONE missing band
//! entry — the dialect has no float read out of a JSON value (`GetKind` is
//! Str/Int/Bool/IntOrNone/List/Dict, and a `.get(k, 0.0)` is `FL2033`). Every
//! one of the four sums or scales a `number` column: `amount`,
//! `annual_revenue`, `probability`. See `HS-9` in the host-surface roadmap.
//!
//! The Guest contract mirrors forge-portal-web: the runtime re-invokes this
//! wasm with `ctx.op_name` set to the bound op; we branch on it and return a
//! raw JSON value (the data-binding resolver lifts it into
//! `props.data.<binding>`), falling through to the page renderer for routes.

#[cfg(target_arch = "wasm32")]
use forge_sdk_v2::{Guest, OpContext, OpError, OpInput, OpOutput, Response};

mod jgen;

// The emitted module's ops spell `crate::fault::...` exactly as they do in
// an all-dialect crate; this re-export is what resolves it here.
pub(crate) use jgen::fault;

// Declarative pages (pages/**/*.page.{json,html,css}) register at link time
// via the codegen build.rs emits into OUT_DIR.
include!(concat!(env!("OUT_DIR"), "/declarative_pages.rs"));

#[cfg(target_arch = "wasm32")]
mod component_export {
    use super::*;
    use serde_json::Value;

    pub struct Crm;

    impl Guest for Crm {
        fn handle(input: OpInput) -> Result<OpOutput, OpError> {
            let parsed: Value = serde_json::from_str(&input.json)
                .map_err(|e| OpError::BadRequest(format!("input is not valid JSON: {e}")))?;
            let ctx = forge_sdk_v2::op_context()
                .map_err(|e| OpError::Internal(format!("op_context unavailable: {e}")))?;

            macro_rules! data_op {
                ($f:path) => {{
                    let v = $f(&ctx, &parsed)?;
                    serde_json::to_string(&v).map_err(|e| OpError::Internal(format!("encode: {e}")))?
                }};
            }

            let json = match ctx.op_name.as_str() {
                // The four ops the dialect cannot yet spell (a float read out
                // of a JSON value). Everything else is `ops/*.java`, served
                // from the emitted `jgen` module below.
                "dashboard_summary" => data_op!(crate::data_ops::dashboard_summary),
                "pipeline_board"    => data_op!(crate::data_ops::pipeline_board),
                "forecast"          => data_op!(crate::data_ops::forecast),
                "create_opportunity"=> data_op!(crate::data_ops::create_opportunity),
                // The dialect ops, then the SSR page dispatcher for routes.
                other => match crate::jgen::dispatch(other, &input.json, ctx.user_id.clone()) {
                    Some(r) => r?,
                    None => {
                        let response = dispatch(&ctx, &parsed)
                            .map_err(|e| OpError::Internal(format!("dispatch: {e}")))?;
                        serde_json::to_string(&response)
                            .map_err(|e| OpError::Internal(format!("encode response: {e}")))?
                    }
                },
            };
            Ok(OpOutput { json })
        }
    }

    // Unauthenticated / forbidden requests land on the sign-in page (NOT /app,
    // which would loop for an anon caller).
    const DISPATCH_CONFIG: forge_web::DispatchConfig =
        forge_web::DispatchConfig::new().with_forbidden_redirect("/login");

    fn dispatch(ctx: &OpContext, input: &Value) -> Result<Response, String> {
        forge_web::dispatch_with(ctx, input, DISPATCH_CONFIG)
    }

    forge_sdk_v2::export!(Crm with_types_in forge_sdk_v2);
}

#[cfg(target_arch = "wasm32")]
mod data_ops;
