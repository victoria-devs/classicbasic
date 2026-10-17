//! Sandboxed Windows Registry Emulation for COM Component Resolution.
//!
//! Visual Basic 6 COM discovery relies on the Windows Registry (specifically `HKEY_CLASSES_ROOT`).
//! In this Linux headless runtime, the registry is virtualized and persisted as a sandboxed
//! JSON configuration file (`registry.json`).
//!
//! Applications and runtime queries interact with this virtual registry seamlessly, allowing
//! ProgID -> CLSID mapping and InprocServer32 discovery without requiring a native Windows environment.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{OnceLock, RwLock};

use crate::types::{
    Clsid, CLSID_Dictionary, CLSID_FileSystemObject, CLSID_Recordset, CLSID_WshNetwork,
    CLSID_WshShell,
};
use serde::{Deserialize, Serialize};

/// Descriptor for a registered COM In-Process Server (`InprocServer32`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct InprocServerInfo {
    /// DLL module filename or path (e.g. `"scrrun.dll"`).
    #[serde(default)]
    pub server_path: String,
    /// COM Threading Model (e.g. `"Apartment"`, `"Both"`, `"Free"`, `"Neutral"`).
    #[serde(default)]
    pub threading_model: String,
}

/// Metadata associated with a COM coclass in the registry.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ClassEntry {
    /// Human-readable friendly name of the class (default value of the CLSID key).
    #[serde(default)]
    pub friendly_name: String,
    /// Programmatic Identifier (`ProgID`).
    #[serde(default)]
    pub prog_id: String,
    /// In-Process 32-bit server metadata.
    #[serde(default)]
    pub inproc_server: Option<InprocServerInfo>,
}

/// Virtualized `HKEY_CLASSES_ROOT` Registry Hive.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Registry {
    /// Mapping from ProgID (case-insensitive) to CLSID string.
    #[serde(default)]
    pub prog_ids: HashMap<String, String>,
    /// Mapping from CLSID string (case-insensitive, canonical `{...}`) to ClassEntry.
    #[serde(default)]
    pub classes: HashMap<String, ClassEntry>,
    /// Generic key-value store for registry values (e.g. HKCU\Software\...).
    #[serde(default)]
    pub values: HashMap<String, String>,
}

impl Registry {
    /// Creates a new Registry instance pre-populated with standard default COM classes.
    pub fn with_defaults() -> Self {
        let mut reg = Self::default();

        reg.register_class(
            "Scripting.Dictionary",
            CLSID_Dictionary,
            "Microsoft Scripting Dictionary",
            "scrrun.dll",
            "Both",
        );
        reg.register_class(
            "Scripting.FileSystemObject",
            CLSID_FileSystemObject,
            "Microsoft Scripting FileSystemObject",
            "scrrun.dll",
            "Both",
        );
        reg.register_class(
            "WScript.Shell",
            CLSID_WshShell,
            "Windows Script Host Shell Object",
            "wshom.ocx",
            "Apartment",
        );
        reg.register_class(
            "WScript.Network",
            CLSID_WshNetwork,
            "Windows Script Host Network Object",
            "wshom.ocx",
            "Apartment",
        );
        reg.register_class(
            "ADODB.Recordset",
            CLSID_Recordset,
            "ActiveX Data Objects Recordset",
            "msado15.dll",
            "Both",
        );

        reg
    }

    /// Registers a COM coclass into the virtual registry.
    pub fn register_class(
        &mut self,
        prog_id: &str,
        clsid: Clsid,
        friendly_name: &str,
        server_path: &str,
        threading_model: &str,
    ) {
        let clsid_str = clsid.to_string_with_braces();
        self.prog_ids
            .insert(prog_id.to_ascii_lowercase(), clsid_str.clone());

        self.classes.insert(
            clsid_str,
            ClassEntry {
                friendly_name: friendly_name.to_string(),
                prog_id: prog_id.to_string(),
                inproc_server: Some(InprocServerInfo {
                    server_path: server_path.to_string(),
                    threading_model: threading_model.to_string(),
                }),
            },
        );
    }

    /// Resolves a ProgID string (e.g. `"Scripting.Dictionary"`) to its corresponding `Clsid`.
    pub fn resolve_progid(&self, prog_id: &str) -> Option<Clsid> {
        let key = prog_id.trim().to_ascii_lowercase();
        if let Some(clsid_str) = self.prog_ids.get(&key) {
            Clsid::parse(clsid_str)
        } else {
            None
        }
    }

    /// Retrieves class registration metadata given a `Clsid`.
    pub fn get_class_entry(&self, clsid: &Clsid) -> Option<&ClassEntry> {
        let key = clsid.to_string_with_braces();
        self.classes.get(&key)
    }

    /// Loads the registry from a JSON file, or falls back to default entries if absent.
    pub fn load_or_default(path: &Path) -> Self {
        if path.exists() {
            if let Ok(content) = fs::read_to_string(path) {
                if let Ok(reg) = serde_json::from_str::<Registry>(&content) {
                    return reg;
                }
            }
        }

        let default_reg = Self::with_defaults();
        let _ = default_reg.save_to_file(path);
        default_reg
    }

    /// Reads a string value from the virtual registry.
    pub fn get_value(&self, key: &str) -> Option<String> {
        let norm = key.trim().to_ascii_lowercase();
        self.values.get(&norm).cloned()
    }

    /// Sets a string value in the virtual registry.
    pub fn set_value(&mut self, key: &str, val: &str) {
        let norm = key.trim().to_ascii_lowercase();
        self.values.insert(norm, val.to_string());
    }

    /// Deletes a key/value from the virtual registry.
    pub fn delete_value(&mut self, key: &str) -> bool {
        let norm = key.trim().to_ascii_lowercase();
        self.values.remove(&norm).is_some()
    }

    /// Returns all key-value pairs whose keys begin with `prefix`.
    pub fn get_prefix_values(&self, prefix: &str) -> Vec<(String, String)> {
        let norm_prefix = prefix.trim().to_ascii_lowercase();
        self.values
            .iter()
            .filter(|(k, _)| k.starts_with(&norm_prefix))
            .map(|(k, v)| {
                let sub = if k.len() > norm_prefix.len() {
                    k[norm_prefix.len()..].to_string()
                } else {
                    k.clone()
                };
                (sub, v.clone())
            })
            .collect()
    }

    /// Deletes all entries whose keys begin with `prefix`.
    pub fn delete_prefix(&mut self, prefix: &str) -> usize {
        let norm_prefix = prefix.trim().to_ascii_lowercase();
        let keys_to_remove: Vec<String> = self
            .values
            .keys()
            .filter(|k| k.starts_with(&norm_prefix))
            .cloned()
            .collect();
        let count = keys_to_remove.len();
        for k in keys_to_remove {
            self.values.remove(&k);
        }
        count
    }

    /// Persists the virtual registry to a formatted JSON file.
    pub fn save_to_file(&self, path: &Path) -> std::io::Result<()> {
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                fs::create_dir_all(parent)?;
            }
        }
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        fs::write(path, json)
    }
}

/// Global registry instance for the application runtime.
static GLOBAL_REGISTRY: OnceLock<RwLock<Registry>> = OnceLock::new();

/// Returns the default path to the sandbox `registry.json` file.
pub fn get_default_registry_path() -> PathBuf {
    PathBuf::from("registry.json")
}

/// Retrieves the shared global registry, initializing it from `registry.json` if not yet loaded.
pub fn global_registry() -> &'static RwLock<Registry> {
    GLOBAL_REGISTRY.get_or_init(|| {
        let path = get_default_registry_path();
        let reg = Registry::load_or_default(&path);
        RwLock::new(reg)
    })
}

/// Resolves a ProgID string using the global virtual registry.
pub fn resolve_progid(prog_id: &str) -> Option<Clsid> {
    global_registry()
        .read()
        .ok()
        .and_then(|r| r.resolve_progid(prog_id))
}

/// Reads a value from the global virtual registry.
pub fn reg_read(key: &str) -> Option<String> {
    global_registry().read().ok().and_then(|r| r.get_value(key))
}

/// Writes a value to the global virtual registry.
pub fn reg_write(key: &str, val: &str) {
    if let Ok(mut r) = global_registry().write() {
        r.set_value(key, val);
    }
}

/// Deletes a value from the global virtual registry.
pub fn reg_delete(key: &str) -> bool {
    global_registry()
        .write()
        .ok()
        .map(|mut r| r.delete_value(key))
        .unwrap_or(false)
}

/// Retrieves all sub-keys and values under a given prefix.
pub fn reg_get_prefix(prefix: &str) -> Vec<(String, String)> {
    global_registry()
        .read()
        .ok()
        .map(|r| r.get_prefix_values(prefix))
        .unwrap_or_default()
}

/// Deletes all values matching a given prefix.
pub fn reg_delete_prefix(prefix: &str) -> usize {
    global_registry()
        .write()
        .ok()
        .map(|mut r| r.delete_prefix(prefix))
        .unwrap_or(0)
}

