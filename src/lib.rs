//! Forge CRM — the thin operations layer.
//!
//! The application is declarative: tables in `schemas/`, UI in `pages/` +
//! `components/`. This crate exists only because page-data bindings resolve
//! through `@op:crm::<name>` operations — so each page-read and each mutation
//! is a small handler here that wraps `storage::query` / `storage::mutate`.
//! There is no business logic worth hiding: row-level security (declared in
//! the schemas) does the enforcement; these handlers just shape payloads.
//!
//! The Guest contract mirrors forge-portal-web: the runtime re-invokes this
//! wasm with `ctx.op_name` set to the bound op; we branch on it and return a
//! raw JSON value (the data-binding resolver lifts it into
//! `props.data.<binding>`), falling through to the page renderer for routes.

#[cfg(target_arch = "wasm32")]
use forge_sdk_v2::{Guest, OpContext, OpError, OpInput, OpOutput, Response};

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
                // queries (page data bindings) — reads scope to the caller's
                // tenant automatically via the schema's row_security.
                "dashboard_summary" => data_op!(crate::data_ops::dashboard_summary),
                "list_accounts"     => data_op!(crate::data_ops::list_accounts),
                "list_contacts"     => data_op!(crate::data_ops::list_contacts),
                "get_contact"       => data_op!(crate::data_ops::get_contact),
                "get_account"       => data_op!(crate::data_ops::get_account),
                "get_opportunity"   => data_op!(crate::data_ops::get_opportunity),
                "list_leads"        => data_op!(crate::data_ops::list_leads),
                "list_opportunities"=> data_op!(crate::data_ops::list_opportunities),
                "pipeline_board"    => data_op!(crate::data_ops::pipeline_board),
                "my_work"           => data_op!(crate::data_ops::my_work),
                "forecast"          => data_op!(crate::data_ops::forecast),
                // mutations (form / htmx POSTs)
                "create_account"     => data_op!(crate::data_ops::create_account),
                "create_contact"     => data_op!(crate::data_ops::create_contact),
                "create_opportunity" => data_op!(crate::data_ops::create_opportunity),
                "update_account"     => data_op!(crate::data_ops::update_account),
                "update_contact"     => data_op!(crate::data_ops::update_contact),
                "update_opportunity" => data_op!(crate::data_ops::update_opportunity),
                "update_lead"        => data_op!(crate::data_ops::update_lead),
                "move_opportunity"   => data_op!(crate::data_ops::move_opportunity),
                "close_opportunity"  => data_op!(crate::data_ops::close_opportunity),
                "log_activity"       => data_op!(crate::data_ops::log_activity),
                "convert_lead"       => data_op!(crate::data_ops::convert_lead),
                // anything else is a page route → the SSR page dispatcher.
                _ => {
                    let response = dispatch(&ctx, &parsed)
                        .map_err(|e| OpError::Internal(format!("dispatch: {e}")))?;
                    serde_json::to_string(&response)
                        .map_err(|e| OpError::Internal(format!("encode response: {e}")))?
                }
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
