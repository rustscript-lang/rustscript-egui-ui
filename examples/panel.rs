use rustscript_egui_ui_policy::ScriptedUiPolicy;

fn main() {
    let policy = ScriptedUiPolicy::from_source(include_str!("../scripts/ui_policy.rss"))
        .expect("policy should compile");
    let spec = policy.evaluate(320, true).expect("policy should evaluate");
    println!(
        "mode={:?}, title={}, accent={:?}",
        spec.mode, spec.title, spec.accent
    );
}
