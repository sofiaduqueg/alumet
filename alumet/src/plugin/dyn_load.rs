use std::{
    collections::HashMap,
    ffi::{c_char, CStr},
    path::Path,
};

// use alumet_api::{
//     AlumetStart,
//     config::{self, ConfigTable},
//     plugin::{ffi, Plugin, PluginError, PluginInfo},
// };
use crate::{config::ConfigTable, plugin::version::Version};
use anyhow::Context;
use libc::c_void;
use libloading::{Library, Symbol};

use super::{dyn_ffi, version, AlumetStart, Plugin, PluginInfo};

/// A plugin initialized from a dynamic library (aka. shared library).
struct DylibPlugin {
    name: String,
    version: String,
    start_fn: dyn_ffi::StartFn,
    stop_fn: dyn_ffi::StopFn,
    drop_fn: dyn_ffi::DropFn,
    // the library must stay loaded for the symbols to be valid
    _library: Library,
    instance: *mut c_void,
}

impl Plugin for DylibPlugin {
    fn name(&self) -> &str {
        &self.name
    }

    fn version(&self) -> &str {
        &self.version
    }

    fn start(&mut self, alumet: &mut AlumetStart) -> anyhow::Result<()> {
        (self.start_fn)(self.instance, alumet); // TODO error handling for ffi
        Ok(())
    }

    fn stop(&mut self) -> anyhow::Result<()> {
        (self.stop_fn)(self.instance); // TODO error handling for ffi
        Ok(())
    }
}

impl Drop for DylibPlugin {
    fn drop(&mut self) {
        // When the external plugin is dropped, call the external code that allocated the
        // `instance` struct, in order to de-allocate it. The external code should also free
        // the resources it has previously opened, if any.
        //
        // **Rule of thumb**: Rust allocations are deallocated by Rust code,
        // C allocations (malloc) are deallocated by C code (free).
        (self.drop_fn)(self.instance);
    }
}

#[derive(Debug)]
pub enum LoadError {
    /// Unable to load something the shared library.
    LibraryLoad(libloading::Error),
    /// A symbol loaded from the library contains an invalid value.
    InvalidSymbol(String, Box<dyn std::error::Error + Send + Sync>),
    /// `plugin_init` failed.
    PluginInit,
}

pub struct PluginRegistry {
    plugins: HashMap<String, Box<dyn Plugin>>,
}

/// Loads a dynamic plugin from a shared library file, and returns a [`PluginInfo`] that allows to initialize the plugin.
pub fn load_cdylib(file: &Path) -> Result<PluginInfo, LoadError> {
    log::debug!("loading dynamic library {}", file.display());

    // load the library and the symbols we need to initialize the plugin
    // BEWARE: to load a constant of type `T` from the shared library, a `Symbol<*const T>` or `Symbol<*mut T>` must be used.
    // However, to load a function of type `fn(A,B) -> R`, a `Symbol<extern fn(A,B) -> R>` must be used.
    let lib = unsafe { Library::new(file)? };
    log::debug!("library loaded");

    let sym_name: Symbol<*const *const c_char> = unsafe { lib.get(b"PLUGIN_NAME\0")? };
    let sym_plugin_version: Symbol<*const *const c_char> = unsafe { lib.get(b"PLUGIN_VERSION\0")? };
    let sym_alumet_version: Symbol<*const *const c_char> = unsafe { lib.get(b"ALUMET_VERSION\0")? };
    let sym_init: Symbol<dyn_ffi::InitFn> = unsafe { lib.get(b"plugin_init\0")? };
    let sym_start: Symbol<dyn_ffi::StartFn> = unsafe { lib.get(b"plugin_start\0")? };
    let sym_stop: Symbol<dyn_ffi::StopFn> = unsafe { lib.get(b"plugin_stop\0")? };
    let sym_drop: Symbol<dyn_ffi::DropFn> = unsafe { lib.get(b"plugin_drop\0")? };

    log::debug!("symbols loaded");

    // convert the C strings to Rust strings
    fn sym_to_string(sym: &Symbol<*const *const c_char>, name: &str) -> Result<String, LoadError> {
        unsafe { CStr::from_ptr(***sym) }
            .to_str()
            .map_err(|e| LoadError::InvalidSymbol(name.into(), e.into()))
            .map(|v| v.to_owned())
    }

    let name = sym_to_string(&sym_name, "PLUGIN_NAME")?;
    let version = sym_to_string(&sym_plugin_version, "PLUGIN_VERSION")?;
    let alumet_version = sym_to_string(&sym_alumet_version, "ALUMET_VERSION")?;
    log::debug!("plugin found: {name} v{version}  (requires ALUMET v{alumet_version})");

    // check the required ALUMET version
    let required_alumet_version = Version::parse(&alumet_version)?;
    if !Version::alumet().can_load(required_alumet_version) {
        todo!("invalid ALUMET version requirement");
    }

    // extract the function pointers from the Symbol, to get around lifetime constraints
    let init_fn = *sym_init;
    let start_fn = *sym_start;
    let stop_fn = *sym_stop;
    let drop_fn = *sym_drop;

    // wrap the plugin info in a Rust struct, to allow the plugin to be initialized later
    let initializable_info = PluginInfo {
        name: name.clone(),
        version: version.clone(),
        init: Box::new(move |config| {
            // initialize the plugin
            let external_plugin = init_fn(config);
            log::debug!("init called from Rust");

            if external_plugin.is_null() {
                return Err(LoadError::PluginInit.into());
            }

            // wrap the external plugin in a nice Rust struct
            let plugin = DylibPlugin {
                name,
                version,
                start_fn,
                stop_fn,
                drop_fn,
                _library: lib,
                instance: external_plugin,
            };
            Ok(Box::new(plugin))
        }),
    };

    Ok(initializable_info)
}

/// Initializes a plugin, using its [`PluginInfo`] and config table (not the global configuration).
pub fn initialize(plugin: PluginInfo, config: toml::Table) -> anyhow::Result<Box<dyn Plugin>> {
    let mut ffi_config = ConfigTable::new(config).context("conversion to ffi-safe configuration failed")?;
    let plugin_instance = (plugin.init)(&mut ffi_config)?;
    Ok(plugin_instance)
}

pub fn plugin_subconfig(plugin: &PluginInfo, global_config: &mut toml::Table) -> anyhow::Result<toml::Table> {
    let name = &plugin.name;
    let sub_config = global_config.remove(name);
    match sub_config {
        Some(toml::Value::Table(t)) => Ok(t),
        Some(bad_value) => Err(anyhow::anyhow!(
            "invalid plugin configuration for '{name}': the value must be a table, not a {}.",
            bad_value.type_str()
        )),
        None => Err(anyhow::anyhow!("missing plugin configuration for '{name}'")),
    }
}

impl LoadError {
    pub fn invalid_symbol(name: &str, source: Box<dyn std::error::Error + Send + Sync>) -> LoadError {
        LoadError::InvalidSymbol(name.to_owned(), source)
    }
}
impl std::error::Error for LoadError {}
impl std::fmt::Display for LoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LoadError::LibraryLoad(err) => write!(f, "failed to load shared library: {err}"),
            LoadError::InvalidSymbol(name, err) => write!(f, "invalid value for symbol {name}: {err}"),
            LoadError::PluginInit => write!(f, "plugin_init returned NULL"),
        }
    }
}
impl From<libloading::Error> for LoadError {
    fn from(value: libloading::Error) -> Self {
        LoadError::LibraryLoad(value)
    }
}
impl From<version::Error> for LoadError {
    fn from(value: version::Error) -> Self {
        LoadError::InvalidSymbol("ALUMET_VERSION".to_owned(), Box::new(value))
    }
}

impl PluginRegistry {
    pub fn register(&mut self, plugin: Box<dyn Plugin>) {
        self.plugins.insert(plugin.name().into(), plugin);
    }

    pub fn get_mut(&mut self, name: &str) -> Option<&mut dyn Plugin> {
        self.plugins.get_mut(name).map(|b| &mut **b as _)
        // the cast is necessary here to coerce the lifetime
        // `&mut dyn Plugin + 'static` to `&mut dyn Plugin + 'a`
    }
}
