use std::sync::{Arc, OnceLock};

use pd_host_function::pd_host_function;
use vm::{
    HostApiCatalog, HostFunctionDescriptor, HostFunctionRegistry, HostFunctionSchema,
    HostModuleDescriptor, HostNamedStruct, HostParamSchema, HostStructField, HostTypeSchema, Value,
    VmError, VmMap, VmResult, borrow_arg,
};

use egui::Color32;

use crate::to_u8;

pub const FROZEN_RUSTSCRIPT_REV: &str = "b1d6cffede77f49410bf63525f30b9a46b02dc01";

/// Single ordered composition for the panel policy host surface.
///
/// This list is the only compile-catalog and restricted-runtime authority.
/// Remaining dynamic Map/Unknown slots: none. `ui_spec` returns Named `UiSpec`;
/// `egui_rgb` returns a packed RGB `int`.
pub fn egui_host_modules() -> [HostModuleDescriptor; 1] {
    [panel_host_module()]
}

fn panel_host_module() -> HostModuleDescriptor {
    HostModuleDescriptor {
        name: "egui.panel",
        functions: &[
            functions::egui_rgb_descriptor,
            functions::ui_spec_descriptor,
        ],
        resources: &[],
    }
}

fn compose_egui_host_catalog() -> Result<HostApiCatalog, vm::HostApiCatalogError> {
    let descriptors: Vec<_> = egui_host_modules()
        .iter()
        .flat_map(|module| module.descriptors())
        .collect();
    HostFunctionDescriptor::collect_catalog(&descriptors)
}

/// Guest catalog derived from [`egui_host_modules`] in declaration order.
pub fn egui_host_catalog() -> Arc<HostApiCatalog> {
    static CATALOG: OnceLock<Arc<HostApiCatalog>> = OnceLock::new();
    CATALOG
        .get_or_init(|| {
            Arc::new(
                compose_egui_host_catalog().expect("egui host composition must build a catalog"),
            )
        })
        .clone()
}

/// Installs the panel module into `registry` against `catalog`.
///
/// Installation is transactional: a later failure leaves `registry` unchanged.
pub fn install_egui_host_modules(
    registry: &mut HostFunctionRegistry,
    catalog: &HostApiCatalog,
) -> VmResult<HostApiCatalog> {
    registry.transactionally(|registry| {
        let mut installed = None;
        for module in egui_host_modules() {
            installed = Some(module.install_from_catalog(registry, catalog)?);
        }
        installed.ok_or_else(|| VmError::HostError("egui host composition is empty".to_string()))
    })
}

mod functions {
    use super::*;

    struct HostUiSpec;

    impl HostNamedStruct for HostUiSpec {
        const NAME: &'static str = "UiSpec";

        fn host_struct_fields() -> Vec<HostStructField> {
            vec![
                HostStructField::new("mode", HostTypeSchema::String),
                HostStructField::new("title", HostTypeSchema::String),
                HostStructField::new("accent", HostTypeSchema::Int),
            ]
        }
    }

    fn ui_spec_contract() -> HostFunctionSchema {
        HostFunctionSchema::with_return(
            "egui::ui_spec",
            vec![
                HostParamSchema::value("mode", HostTypeSchema::String),
                HostParamSchema::value("title", HostTypeSchema::String),
                HostParamSchema::value("accent", HostTypeSchema::Int),
            ],
            HostUiSpec::host_type_schema(),
        )
    }

    /// Packs an egui RGB triplet into the panel accent integer.
    #[pd_host_function(name = "egui::rgb")]
    fn egui_rgb(red: i64, green: i64, blue: i64) -> i64 {
        let color = Color32::from_rgb(to_u8(red), to_u8(green), to_u8(blue));
        let [red, green, blue, _alpha] = color.to_array();
        ((red as i64) << 16) | ((green as i64) << 8) | i64::from(blue)
    }

    /// Builds the fixed-shape panel spec the UI host consumes.
    #[pd_host_function(name = "egui::ui_spec", contract = ui_spec_contract)]
    fn ui_spec(mode: String, title: String, accent: i64) -> VmResult<VmMap> {
        Ok(VmMap::from_entries(vec![
            (Value::string("mode"), Value::string(mode)),
            (Value::string("title"), Value::string(title)),
            (Value::string("accent"), Value::Int(accent)),
        ]))
    }
}
