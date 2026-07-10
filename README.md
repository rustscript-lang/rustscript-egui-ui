# rustscript-egui-ui

Example of using RustScript in an egui application.

## What it proves

A desktop/UI application can keep egui rendering compiled while moving responsive UI policy into RustScript:

- compiled UI type: `UiSpec { mode, title, accent: egui::Color32 }`
- scripted inputs: viewport width and error state
- RustScript calls `egui_rgb(r, g, b) -> int`, a host function backed by `egui::Color32::from_rgb`
- scripted behavior: compact/wide mode, title, and accent color selection

This does not fork or patch egui. It depends on upstream `egui` and local `pd-vm` path only.

## Run

```bash
cargo test --tests --jobs 4
cargo run --example panel
```

## Script

See `scripts/ui_policy.rss`.
