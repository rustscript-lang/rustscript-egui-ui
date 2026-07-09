use egui::Color32;
use pretty_assertions::assert_eq;
use rustscript_egui_ui_policy::{ScriptedUiPolicy, UiMode};

#[test]
fn rustscript_calls_egui_color_function_for_compact_error_style() {
    let policy = ScriptedUiPolicy::from_source(include_str!("../scripts/ui_policy.rss"))
        .expect("policy should compile");

    let spec = policy.evaluate(320, true).expect("policy should evaluate");

    assert_eq!(spec.mode, UiMode::Compact);
    assert_eq!(spec.title, "Fix errors");
    assert_eq!(spec.accent, Color32::from_rgb(220, 40, 40));
}

#[test]
fn rustscript_calls_egui_color_function_for_wide_normal_style() {
    let policy = ScriptedUiPolicy::from_source(include_str!("../scripts/ui_policy.rss"))
        .expect("policy should compile");

    let spec = policy.evaluate(900, false).expect("policy should evaluate");

    assert_eq!(spec.mode, UiMode::Wide);
    assert_eq!(spec.title, "Ready");
    assert_eq!(spec.accent, Color32::from_rgb(60, 160, 90));
}

#[test]
fn rustscript_can_call_egui_rgb_from_inline_policy() {
    let policy = ScriptedUiPolicy::from_source(
        r#"
fn egui_rgb(r, g, b) -> int;
fn ui_spec(mode, title, accent) -> string;

let compact = width < 480;
let mode = if compact => { "compact" } else => { "wide" };
let title = if has_errors => { "Fix errors" } else => { "Ready" };
let accent = if has_errors => {
    egui_rgb(220, 40, 40)
} else => {
    egui_rgb(60, 160, 90)
};
ui_spec(mode, title, accent);
"#,
    )
    .expect("policy should compile");

    let spec = policy.evaluate(320, true).expect("policy should evaluate");

    assert_eq!(spec.mode, UiMode::Compact);
    assert_eq!(spec.title, "Fix errors");
    assert_eq!(spec.accent, Color32::from_rgb(220, 40, 40));
}
