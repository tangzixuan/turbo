use std::collections::HashMap;

#[derive(PartialEq, Debug, Clone)]
pub enum CssModuleReference {
    /// A local reference.
    Local {
        /// The local (compiled) name for the reference.
        name: String,
    },
    /// A global reference.
    Global {
        /// The referenced global name.
        name: String,
    },
    /// A reference to an export in a different file.
    Dependency {
        /// The name to reference within the dependency.
        name: String,
        /// The dependency specifier for the referenced file.
        specifier: String,
    },
}

pub type CssModuleExports = HashMap<String, CssModuleExport>;

/// An exported value from a CSS module.
#[derive(PartialEq, Debug, Clone)]

pub struct CssModuleExport {
    /// The local (compiled) name for this export.
    pub name: String,
    /// Other names that are composed by this export.
    pub composes: Vec<CssModuleReference>,
    /// Whether the export is referenced in this file.
    pub is_referenced: bool,
}
