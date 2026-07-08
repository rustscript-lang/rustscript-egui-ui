use egui::Color32;
use pretty_assertions::assert_eq;
use rustscript_egui_ui_policy::{ScriptedUiPolicy, UiMode};

#[test]
fn rustscript_selects_egui_compact_error_style() {
    let policy = ScriptedUiPolicy::from_source(include_str!("../scripts/ui_policy.rss"))
        .expect("policy should compile");

    let spec = policy.evaluate(320, true).expect("policy should evaluate");

    assert_eq!(spec.mode, UiMode::Compact);
    assert_eq!(spec.title, "Fix errors");
    assert_eq!(spec.accent, Color32::from_rgb(220, 40, 40));
}

#[test]
fn rustscript_selects_egui_wide_normal_style() {
    let policy = ScriptedUiPolicy::from_source(include_str!("../scripts/ui_policy.rss"))
        .expect("policy should compile");

    let spec = policy.evaluate(900, false).expect("policy should evaluate");

    assert_eq!(spec.mode, UiMode::Wide);
    assert_eq!(spec.title, "Ready");
    assert_eq!(spec.accent, Color32::from_rgb(60, 160, 90));
}
