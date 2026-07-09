use egui::Color32;
use vm::{CallOutcome, CallReturn, HostFunction, Value, Vm, VmError, VmStatus, compile_source};

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
        evaluate_raw(&source, 800, false)?;
        Ok(Self { source })
    }

    pub fn evaluate(&self, width: i64, has_errors: bool) -> Result<UiSpec, String> {
        let raw = evaluate_raw(&self.source, width, has_errors)?;
        parse_spec(&raw)
    }
}

fn parse_spec(raw: &str) -> Result<UiSpec, String> {
    let (mode, state) = raw
        .split_once(':')
        .ok_or_else(|| format!("invalid ui policy result '{raw}'"))?;
    let mode = match mode {
        "compact" => UiMode::Compact,
        "wide" => UiMode::Wide,
        other => return Err(format!("unknown ui mode '{other}'")),
    };
    let (title, accent) = match state {
        "error" => ("Fix errors".to_string(), Color32::from_rgb(220, 40, 40)),
        "normal" => ("Ready".to_string(), Color32::from_rgb(60, 160, 90)),
        other => return Err(format!("unknown ui state '{other}'")),
    };
    Ok(UiSpec {
        mode,
        title,
        accent,
    })
}

fn evaluate_raw(source: &str, width: i64, has_errors: bool) -> Result<String, String> {
    let wrapped = format!(
        "let width = {width};\nlet has_errors = {};\n{source}",
        if has_errors { "true" } else { "false" }
    );
    match run_value(&wrapped)? {
        Value::String(value) => Ok(value.as_str().to_string()),
        other => Err(format!("script returned {other:?}; expected string")),
    }
}

struct UiSpecHost;

impl HostFunction for UiSpecHost {
    fn call(&mut self, _vm: &mut Vm, args: &[Value]) -> Result<CallOutcome, VmError> {
        match args {
            [Value::String(mode), Value::String(state)] => Ok(CallOutcome::Return(
                CallReturn::one(Value::string(format!("{mode}:{state}"))),
            )),
            _ => Err(VmError::TypeMismatch("ui mode and state strings")),
        }
    }
}

fn run_value(source: &str) -> Result<Value, String> {
    let compiled = compile_source(source).map_err(|err| err.to_string())?;
    let mut vm = Vm::new(compiled.program);
    vm.bind_function("ui_spec", Box::new(UiSpecHost));
    let status = vm.run().map_err(|err| err.to_string())?;
    if status != VmStatus::Halted {
        return Err(format!("script did not halt: {status:?}"));
    }
    vm.stack()
        .last()
        .cloned()
        .ok_or_else(|| "script returned an empty stack".to_string())
}
