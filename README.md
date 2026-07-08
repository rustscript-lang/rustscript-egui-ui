# rustscript-egui-ui-policy

Standalone egui integration demo for `pd-vm` / RustScript.

## What it proves

A desktop/UI application can keep egui rendering compiled while moving responsive UI policy into RustScript:

- compiled UI type: `UiSpec { mode, title, accent: egui::Color32 }`
- scripted inputs: viewport width and error state
- scripted behavior: compact/wide mode and normal/error state selection

This does not fork or patch egui. It depends on upstream `egui` and local `pd-vm` path only.

## Run

```bash
cargo test --tests --jobs 4
cargo run --example panel
```

## Script

See `scripts/ui_policy.rss`.
