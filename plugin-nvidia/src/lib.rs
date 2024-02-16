use anyhow::anyhow;
use nvml::{NvmlDevices, NvmlSource};

mod jetson;
mod nvml;

struct NvidiaPlugin;

impl alumet::plugin::Plugin for NvidiaPlugin {
    fn name(&self) -> &str {
        "nvidia"
    }

    fn version(&self) -> &str {
        "0.1.0"
    }

    fn start(&mut self, alumet: &mut alumet::plugin::AlumetStart) -> anyhow::Result<()> {
        let nvml = NvmlDevices::detect(true)?;
        let stats = nvml.detection_stats();
        if stats.found_devices == 0 {
            return Err(anyhow!("No NVML-compatible GPU found."))
        }
        if stats.working_devices == 0 {
            return Err(anyhow!("{} NVML-compatible devices found but none of them is working (see previous warnings).", stats.found_devices));
        }

        let metrics = nvml::Metrics::new(alumet)?;
        
        for maybe_device in nvml.devices {
            if let Some(device) = maybe_device {
                let source = NvmlSource::new(device, metrics.clone())?;
                alumet.add_source(Box::new(source));
            }
        }
        Ok(())
    }

    fn stop(&mut self) -> anyhow::Result<()> {
        Ok(())
    }
}