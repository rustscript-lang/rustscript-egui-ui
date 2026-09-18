use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use rustscript_egui_ui_policy::{
    FROZEN_RUSTSCRIPT_REV, ScriptedUiPolicy, egui_host_catalog, egui_host_modules,
    install_egui_host_modules,
};
use vm::{
    CompileSourceFileOptions, HostAdapterDescriptor, HostApiCatalog, HostFunctionRegistry,
    HostFunctionSchema, HostTypeSchema, SourceFlavor, Vm, compile_source_with_flavor_and_options,
};

fn schema_is_dynamic(schema: &HostTypeSchema) -> bool {
    match schema {
        HostTypeSchema::Unknown | HostTypeSchema::Map(_) => true,
        HostTypeSchema::Array(inner) | HostTypeSchema::Optional(inner) => schema_is_dynamic(inner),
        HostTypeSchema::Callable { params, result } => {
            params.iter().any(schema_is_dynamic) || schema_is_dynamic(result)
        }
        HostTypeSchema::Named { fields, .. } => {
            fields.iter().any(|field| schema_is_dynamic(&field.ty))
        }
        _ => false,
    }
}

fn dynamic_slots(catalog: &HostApiCatalog) -> Vec<String> {
    let mut dynamic = Vec::new();
    for function in catalog.functions() {
        if schema_is_dynamic(&function.return_type) {
            dynamic.push(format!("{} return", function.name));
        }
        for param in &function.params {
            if schema_is_dynamic(&param.ty) {
                dynamic.push(format!("{} param {}", function.name, param.name));
            }
        }
    }
    dynamic.sort();
    dynamic
}

fn rss_paths(dir: &str) -> Vec<PathBuf> {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(dir);
    let mut paths: Vec<PathBuf> = fs::read_dir(&dir)
        .unwrap_or_else(|err| panic!("read {}: {err}", dir.display()))
        .map(|entry| entry.expect("entry").path())
        .filter(|path| path.extension().and_then(|ext| ext.to_str()) == Some("rss"))
        .collect();
    paths.sort();
    paths
}

fn compile_against(catalog: Arc<HostApiCatalog>, source: &str) -> vm::CompiledProgram {
    compile_source_with_flavor_and_options(
        source,
        SourceFlavor::RustScript,
        CompileSourceFileOptions::default().with_host_api_catalog(catalog),
    )
    .expect("script compiles")
}

fn compile_rss(path: &Path) {
    let source = fs::read_to_string(path)
        .unwrap_or_else(|err| panic!("{} failed to read: {err}", path.display()));
    ScriptedUiPolicy::from_source(source)
        .unwrap_or_else(|err| panic!("{} failed to compile: {err}", path.display()));
}

#[test]
fn composition_catalog_and_adapter_names_are_the_same_surface() {
    let modules = egui_host_modules();
    let composition_names: Vec<String> = modules
        .iter()
        .flat_map(|module| module.descriptors())
        .map(|descriptor| descriptor.schema.name.clone())
        .collect();
    let adapter_names: Vec<String> = modules
        .iter()
        .flat_map(|module| module.descriptors())
        .map(|descriptor| match descriptor.adapter {
            HostAdapterDescriptor::StaticArgs(_)
            | HostAdapterDescriptor::StaticNonYieldingArgs(_)
            | HostAdapterDescriptor::Static(_)
            | HostAdapterDescriptor::StaticStack(_)
            | HostAdapterDescriptor::StaticStackRuntimeOwned(_)
            | HostAdapterDescriptor::Owned(_) => descriptor.schema.name.clone(),
        })
        .collect();
    let catalog_names: Vec<String> = egui_host_catalog()
        .functions()
        .iter()
        .map(|function| function.name.clone())
        .collect();

    assert_eq!(composition_names.len(), 2);
    assert_eq!(composition_names, adapter_names);
    assert_eq!(adapter_names, catalog_names);

    let composition_set: BTreeSet<_> = composition_names.iter().cloned().collect();
    assert_eq!(composition_set.len(), composition_names.len());
    assert_eq!(
        composition_names,
        ["egui::rgb".to_string(), "egui::ui_spec".to_string()]
    );
}

#[test]
fn ui_spec_return_is_named() {
    let catalog = egui_host_catalog();
    let ui_spec = catalog
        .functions()
        .iter()
        .find(|function| function.name == "egui::ui_spec")
        .expect("egui::ui_spec is in the catalog");
    match &ui_spec.return_type {
        HostTypeSchema::Named { name, fields } => {
            assert_eq!(name, "UiSpec");
            assert_eq!(fields.len(), 3);
            assert_eq!(fields[0].name, "mode");
            assert_eq!(fields[1].name, "title");
            assert_eq!(fields[2].name, "accent");
        }
        other => panic!("expected Named UiSpec, got {other:?}"),
    }
}

#[test]
fn dynamic_map_any_unknown_slots_match_allowlist() {
    assert!(
        dynamic_slots(egui_host_catalog().as_ref()).is_empty(),
        "panel host surface has no dynamic Map/Unknown slots"
    );
}

#[test]
fn deny_before_install_then_allow_after() {
    let catalog = egui_host_catalog();
    let compiled = compile_against(
        catalog.clone(),
        "use egui;\nlet _color = egui::rgb(1, 2, 3);\n",
    );

    let mut denied = Vm::new(compiled.program.clone());
    let empty = HostFunctionRegistry::restricted();
    let error = empty
        .bind_vm_cached(&mut denied)
        .expect_err("restricted bind must deny before install");
    let message = error.to_string();
    assert!(
        message.contains("egui::rgb") || message.contains("not authorized"),
        "unexpected deny-before error: {message}"
    );

    let mut allowed = Vm::new(compiled.program);
    let mut registry = HostFunctionRegistry::restricted();
    install_egui_host_modules(&mut registry, catalog.as_ref()).expect("install composition");
    registry
        .bind_vm_cached(&mut allowed)
        .expect("allow after exact install");
}

#[test]
fn unlisted_host_import_is_denied_at_bind() {
    let production = egui_host_catalog();
    let mut builder = HostApiCatalog::builder();
    for schema in production.structs() {
        builder.named_struct(schema.clone());
    }
    for function in production.functions() {
        builder.function(function.clone());
    }
    builder.function(HostFunctionSchema::with_return(
        "egui::not_exported",
        vec![],
        HostTypeSchema::Bool,
    ));
    let expanded = Arc::new(builder.build().expect("expanded catalog builds"));
    let compiled = compile_against(expanded, "use egui;\nlet _ok = egui::not_exported();\n");

    let mut registry = HostFunctionRegistry::restricted();
    install_egui_host_modules(&mut registry, production.as_ref()).expect("install composition");
    let mut vm = Vm::new(compiled.program);
    let error = registry
        .bind_vm_cached(&mut vm)
        .expect_err("unlisted import must be denied at bind");
    let message = error.to_string();
    assert!(
        message.contains("egui::not_exported"),
        "bind denial must name the unlisted import: {message}"
    );
}

#[test]
fn rustscript_crates_are_pinned_to_the_frozen_full_sha() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let cargo_toml = fs::read_to_string(manifest_dir.join("Cargo.toml")).expect("Cargo.toml");
    let cargo_lock = fs::read_to_string(manifest_dir.join("Cargo.lock")).expect("Cargo.lock");
    assert!(
        cargo_toml.contains(FROZEN_RUSTSCRIPT_REV),
        "Cargo.toml must pin the frozen full SHA"
    );
    assert!(
        !cargo_toml.contains("path = \"../rustscript"),
        "production RustScript crates must not use sibling path deps"
    );
    assert!(
        !cargo_toml.contains("/home/"),
        "production RustScript crates must not use machine-specific paths"
    );
    let expected_source = format!(
        "git+https://github.com/rustscript-lang/rustscript?rev={FROZEN_RUSTSCRIPT_REV}#{FROZEN_RUSTSCRIPT_REV}"
    );
    for crate_name in ["pd-vm", "pd-host-function", "pd-host-schema"] {
        assert!(
            cargo_lock.contains(&format!("name = \"{crate_name}\"")),
            "{crate_name} must appear in Cargo.lock"
        );
        assert!(
            cargo_lock.contains(&expected_source),
            "{crate_name} must resolve to the frozen git SHA"
        );
    }
}

#[test]
fn bundled_rss_scripts_compile_through_the_production_catalog() {
    let scripts = rss_paths("scripts");
    assert_eq!(
        scripts.len(),
        1,
        "plan expected one bundled RSS example, found {scripts:?}"
    );
    for path in scripts {
        compile_rss(&path);
    }
}
