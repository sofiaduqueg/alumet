//! Definition of Rust plugins.
//! 
//! See the [documentation of the plugin module](super#static-plugins).

use crate::{
    config::ConfigTable,
    plugin::{AlumetStart, Plugin},
};

use super::manage::PluginStartup;

/// Trait for Alumet plugins written in Rust.
///
/// Implement this trait to define your plugin.
/// See the [plugin module documentation](super#static-plugins).
pub trait AlumetPlugin {
    // Note: add `where Self: Sized` to make this trait "object safe", if necessary in the future.

    /// The name of the plugin. It must be unique: two plugins cannot have the same name.
    fn name() -> &'static str;

    /// The version of the plugin, for instance `"1.2.3"`. It should adhere to semantic versioning.
    fn version() -> &'static str;

    /// Initializes the plugin.
    ///
    /// Read more about the plugin lifecycle in the [module documentation](super).
    fn init(config: &mut ConfigTable) -> anyhow::Result<Box<Self>>;

    /// Starts the plugin, allowing it to register metrics, sources and outputs.
    ///
    /// ## Plugin restart
    /// A plugin can be started and stopped multiple times, for instance when ALUMET switches from monitoring to profiling mode.
    /// [`AlumetPlugin::stop`] is guaranteed to be called between two calls of [`AlumetPlugin::start`].
    fn start(&mut self, alumet: &mut AlumetStart) -> anyhow::Result<()>;

    /// Stops the plugin.
    ///
    /// This method is called _after_ all the metrics, sources and outputs previously registered
    /// by [`AlumetPlugin::start`] have been stopped and unregistered.
    fn stop(&mut self) -> anyhow::Result<()>;
    
    /// Function called after the plugin startup phase, i.e. after every plugin has started.
    ///
    /// It can be used, for instance, to examine the metrics that have been registered.
    fn post_startup(&mut self, startup: &PluginStartup) -> anyhow::Result<()> {
        Ok(())
    }
}

// Every AlumetPlugin is a Plugin :)
impl<P: AlumetPlugin> Plugin for P {
    fn name(&self) -> &str {
        P::name() as _
    }

    fn version(&self) -> &str {
        P::version() as _
    }

    fn start(&mut self, alumet: &mut AlumetStart) -> anyhow::Result<()> {
        AlumetPlugin::start(self, alumet)
    }

    fn stop(&mut self) -> anyhow::Result<()> {
        AlumetPlugin::stop(self)
    }
    
    fn post_startup(&mut self, startup: &PluginStartup) -> anyhow::Result<()> {
        AlumetPlugin::post_startup(self, startup)
    }
}