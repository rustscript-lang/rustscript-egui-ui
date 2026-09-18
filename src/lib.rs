use egui::Color32;
use vm::{
    CompileSourceFileOptions, HostFunctionRegistry, SourceFlavor, Value, Vm, VmMap, VmStatus,
    compile_source_with_flavor_and_options,
};

pub(crate) type VmResult<T> = vm::VmResult<T>;

mod host;

pub use host::{
    FROZEN_RUSTSCRIPT_REV, egui_host_catalog, egui_host_modules, install_egui_host_modules,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UiMode {
    Compact,
    Wide,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UiSpec {
    pub mode: UiMode,
    pub title: String,
    pub accent: Color32,
}

#[derive(Debug, Clone)]
pub struct ScriptedUiPolicy {
    source: String,
}

impl ScriptedUiPolicy {
    pub fn from_source(source: impl Into<String>) -> Result<Self, String> {
        let source = source.into();
        evaluate_value(&source, 800, false)?;
        Ok(Self { source })
    }

    pub fn evaluate(&self, width: i64, has_errors: bool) -> Result<UiSpec, String> {
        let value = evaluate_value(&self.source, width, has_errors)?;
        parse_spec(&value)
    }
}

fn parse_spec(value: &Value) -> Result<UiSpec, String> {
    let Value::Map(map) = value else {
        return Err(format!("script returned {value:?}; expected UiSpec map"));
    };
    let mode = map_string(map, "mode")?;
    let title = map_string(map, "title")?;
    let accent = map_int(map, "accent")?;
    let mode = match mode.as_str() {
        "compact" => UiMode::Compact,
        "wide" => UiMode::Wide,
        other => return Err(format!("unknown ui mode '{other}'")),
    };
    let red = ((accent >> 16) & 0xff) as u8;
    let green = ((accent >> 8) & 0xff) as u8;
    let blue = (accent & 0xff) as u8;
    Ok(UiSpec {
        mode,
        title,
        accent: Color32::from_rgb(red, green, blue),
    })
}

fn map_string(map: &VmMap, key: &str) -> Result<String, String> {
    match map.get(&Value::string(key)) {
        Some(Value::String(value)) => Ok(value.as_str().to_string()),
        Some(other) => Err(format!(
            "ui spec field '{key}' was {other:?}; expected string"
        )),
        None => Err(format!("ui spec missing field '{key}'")),
    }
}

fn map_int(map: &VmMap, key: &str) -> Result<i64, String> {
    match map.get(&Value::string(key)) {
        Some(Value::Int(value)) => Ok(*value),
        Some(other) => Err(format!("ui spec field '{key}' was {other:?}; expected int")),
        None => Err(format!("ui spec missing field '{key}'")),
    }
}

fn evaluate_value(source: &str, width: i64, has_errors: bool) -> Result<Value, String> {
    run_value(&inject_policy_inputs(source, width, has_errors))
}

fn inject_policy_inputs(source: &str, width: i64, has_errors: bool) -> String {
    let inputs = format!(
        "let width = {width};\nlet has_errors = {};\n",
        if has_errors { "true" } else { "false" }
    );
    let mut uses = String::new();
    let mut rest = String::new();
    let mut in_uses = true;
    for line in source.lines() {
        let trimmed = line.trim();
        if in_uses && (trimmed.is_empty() || trimmed.starts_with("use ")) {
            uses.push_str(line);
            uses.push('\n');
        } else {
            in_uses = false;
            rest.push_str(line);
            rest.push('\n');
        }
    }
    format!("{uses}{inputs}{rest}")
}

fn compile_options() -> CompileSourceFileOptions {
    CompileSourceFileOptions::default().with_host_api_catalog(egui_host_catalog())
}

fn run_value(source: &str) -> Result<Value, String> {
    let catalog = egui_host_catalog();
    let compiled =
        compile_source_with_flavor_and_options(source, SourceFlavor::RustScript, compile_options())
            .map_err(|err| err.to_string())?;
    let mut registry = HostFunctionRegistry::restricted();
    install_egui_host_modules(&mut registry, catalog.as_ref()).map_err(|err| err.to_string())?;
    let mut vm = Vm::new(compiled.program);
    registry
        .bind_vm_cached(&mut vm)
        .map_err(|err| err.to_string())?;
    let status = vm.run().map_err(|err| err.to_string())?;
    if status != VmStatus::Halted {
        return Err(format!("script did not halt: {status:?}"));
    }
    vm.stack()
        .last()
        .cloned()
        .ok_or_else(|| "script returned an empty stack".to_string())
}

pub(crate) fn to_u8(value: i64) -> u8 {
    value.clamp(0, i64::from(u8::MAX)) as u8
}
