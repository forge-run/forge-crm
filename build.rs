//! Forge CRM build pipeline — mirrors the forge-portal-web walker setup,
//! trimmed to what this app needs:
//!   1. validate app.json (a broken manifest fails the build, not first request)
//!   2. walk components/ → scoped CSS records + the resolved name set
//!   3. walk pages/ → validate manifests, cross-check component() calls,
//!      emit declarative_pages.rs (included by src/lib.rs)
//!   4. concatenate + minify the page/component CSS into static/app.<hash>.css
//!
//! service.json op schemas are hand-authored here (unlike the portal, which
//! derives them from typed Rust structs) — keep them in sync with src/data_ops.rs by hand.

use std::path::PathBuf;

fn main() {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let src_root = manifest.join("src");
    let static_dir = manifest.join("static");
    let out_env = PathBuf::from(std::env::var_os("OUT_DIR").expect("OUT_DIR unset"));

    println!("cargo:rerun-if-changed=src");
    println!("cargo:rerun-if-changed=pages");
    println!("cargo:rerun-if-changed=components");
    println!("cargo:rerun-if-changed=static");
    println!("cargo:rerun-if-changed=app.json");
    println!("cargo:rerun-if-changed=service.json");
    println!("cargo:rerun-if-changed=build.rs");

    // ── app.json validation ────────────────────────────────────
    let app_json = manifest.join("app.json");
    let app_bytes = std::fs::read(&app_json)
        .unwrap_or_else(|e| panic!("read app.json {}: {e}", app_json.display()));
    let app_manifest: forge_schema::AppManifest =
        serde_json::from_slice(&app_bytes).unwrap_or_else(|e| panic!("parse app.json: {e}"));
    let errors = forge_schema::validate_app_manifest(&app_manifest);
    if !errors.is_empty() {
        panic!("app.json validation failed: {errors:?}");
    }
    println!(
        "cargo:warning=forge-crm app.json validated: {} (v{})",
        app_manifest.app.name, app_manifest.app.version,
    );

    // ── components walk ─────────────────────────────────────────
    let component_records =
        match forge_web_build::collect_and_bundle_components(&manifest, &out_env) {
            Ok(r) => r,
            Err(e) => panic!("forge-web-build components: {e}"),
        };
    println!("cargo:warning=forge-crm components: {} record(s)", component_records.len());

    // ── pages walk + declarative-page codegen ───────────────────
    let component_names: std::collections::HashSet<&str> =
        component_records.iter().map(|r| r.name.as_str()).collect();
    let page_records =
        match forge_web_build::collect_and_bundle_pages(&manifest, &out_env, &component_names) {
            Ok(r) => r,
            Err(e) => panic!("forge-web-build pages: {e}"),
        };
    println!("cargo:warning=forge-crm pages: {} record(s)", page_records.len());

    if let Err(e) = forge_web_build::emit_declarative_pages_register(&page_records, &out_env) {
        panic!("forge-web-build emit_declarative_pages_register: {e}");
    }

    // ── scoped + minified CSS bundle → static/app.<hash>.css ────
    let substrate_css = forge_web_build::substrate_css_string(&component_records, &page_records);
    match forge_web_build::collect_and_minify_css_with_substrate(
        &src_root, &static_dir, &out_env, &substrate_css,
    ) {
        Ok(r) => println!(
            "cargo:warning=forge-crm CSS: {} blocks, {} → {} bytes, href={}",
            r.blocks, r.bytes_in, r.bytes_out, r.href
        ),
        Err(e) => panic!("forge-web-build CSS: {e}"),
    }
}
