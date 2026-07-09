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
    let mut parts = raw.splitn(3, ':');
    let mode = parts
        .next()
        .ok_or_else(|| format!("invalid ui policy result '{raw}'"))?;
    let title = parts
        .next()
        .ok_or_else(|| format!("invalid ui policy result '{raw}'"))?;
    let accent = parts
        .next()
        .ok_or_else(|| format!("invalid ui policy result '{raw}'"))?;
    let mode = match mode {
        "compact" => UiMode::Compact,
        "wide" => UiMode::Wide,
        other => return Err(format!("unknown ui mode '{other}'")),
    };
    let accent = accent
        .parse::<u32>()
        .map_err(|err| format!("invalid ui accent '{accent}': {err}"))?;
    let red = ((accent >> 16) & 0xff) as u8;
    let green = ((accent >> 8) & 0xff) as u8;
    let blue = (accent & 0xff) as u8;
    Ok(UiSpec {
        mode,
        title: title.to_string(),
        accent: Color32::from_rgb(red, green, blue),
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
            [
                Value::String(mode),
                Value::String(title),
                Value::Int(accent),
            ] => Ok(CallOutcome::Return(CallReturn::one(Value::string(
                format!("{mode}:{title}:{accent}"),
            )))),
            _ => Err(VmError::TypeMismatch("ui mode, title and accent")),
        }
    }
}

struct EguiRgbHost;

impl HostFunction for EguiRgbHost {
    fn call(&mut self, _vm: &mut Vm, args: &[Value]) -> Result<CallOutcome, VmError> {
        match args {
            [Value::Int(red), Value::Int(green), Value::Int(blue)] => {
                let color = Color32::from_rgb(to_u8(*red), to_u8(*green), to_u8(*blue));
                let [red, green, blue, _alpha] = color.to_array();
                let packed = ((red as i64) << 16) | ((green as i64) << 8) | blue as i64;
                Ok(CallOutcome::Return(CallReturn::one(Value::Int(packed))))
            }
            _ => Err(VmError::TypeMismatch("rgb ints")),
        }
    }
}

fn run_value(source: &str) -> Result<Value, String> {
    let compiled = compile_source(source).map_err(|err| err.to_string())?;
    let mut vm = Vm::new(compiled.program);
    vm.bind_function("ui_spec", Box::new(UiSpecHost));
    vm.bind_function("egui_rgb", Box::new(EguiRgbHost));
    let status = vm.run().map_err(|err| err.to_string())?;
    if status != VmStatus::Halted {
        return Err(format!("script did not halt: {status:?}"));
    }
    vm.stack()
        .last()
        .cloned()
        .ok_or_else(|| "script returned an empty stack".to_string())
}

fn to_u8(value: i64) -> u8 {
    value.clamp(0, u8::MAX as i64) as u8
}
