use std::{borrow::Cow, fmt};


/// Hardware or software entity for which metrics can be gathered.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
#[repr(C)]
pub enum ResourceId {
    /// The whole local machine, for instance the whole physical server.
    LocalMachine,
    /// A process at the OS level.
    Process { pid: u32 },
    /// A control group, often abbreviated cgroup.
    ControlGroup { path: StrCow },
    /// A physical CPU package (which is not the same as a NUMA node).
    CpuPackage { id: u32 },
    /// A CPU core.
    CpuCore { id: u32 },
    /// The RAM attached to a CPU package.
    Dram { pkg_id: u32 },
    /// A dedicated GPU.
    Gpu { bus_id: StrCow },
    /// A custom resource
    Custom { kind: StrCow, id: StrCow },
}

/// Alias to a static cow. It helps avoiding allocations of Strings.
pub type StrCow = Cow<'static, str>;

impl ResourceId {
    /// Creates a new [`ResourceId::Custom`] with the given kind and id.
    /// You can pass `&'static str` as kind, id, or both in order to avoid allocating memory.
    /// Strings are also accepted and will be moved into the ResourceId.
    pub fn custom(kind: impl Into<StrCow>, id: impl Into<StrCow>) -> ResourceId {
        ResourceId::Custom {
            kind: kind.into(),
            id: id.into(),
        }
    }

    pub fn kind(&self) -> &str {
        match self {
            ResourceId::LocalMachine => "local_machine",
            ResourceId::Process { .. } => "process",
            ResourceId::ControlGroup { .. } => "cgroup",
            ResourceId::CpuPackage { .. } => "cpu_package",
            ResourceId::CpuCore { .. } => "cpu_core",
            ResourceId::Dram { .. } => "dram",
            ResourceId::Gpu { .. } => "gpu",
            ResourceId::Custom { kind, id: _ } => &kind,
        }
    }

    pub fn id_str(&self) -> impl fmt::Display + '_ {
        match self {
            ResourceId::LocalMachine => LazyDisplayable::Str(""),
            ResourceId::Process { pid } => LazyDisplayable::U32(*pid),
            ResourceId::ControlGroup { path } => LazyDisplayable::Str(&path),
            ResourceId::CpuPackage { id } => LazyDisplayable::U32(*id),
            ResourceId::CpuCore { id } => LazyDisplayable::U32(*id),
            ResourceId::Dram { pkg_id } => LazyDisplayable::U32(*pkg_id),
            ResourceId::Gpu { bus_id } => LazyDisplayable::Str(&bus_id),
            ResourceId::Custom { kind: _, id } => LazyDisplayable::Str(&id),
        }
    }
}

enum LazyDisplayable<'a> {
    U32(u32),
    Str(&'a str),
}

impl<'a> fmt::Display for LazyDisplayable<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LazyDisplayable::U32(id) => write!(f, "{id}"),
            LazyDisplayable::Str(id) => write!(f, "{id}"),
        }
    }
}