pub mod node;

#[cfg(all(target_family = "wasm", target_os = "unknown"))]
pub mod wasm;

#[cfg(feature = "keyboard")]
pub mod hid;

#[cfg(feature = "keyboard")]
pub mod serial;

// Shared types for keyboard and serial modules
#[cfg(feature = "keyboard")]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Suffix {
    #[default]
    Enter,
    Tab,
    None,
}
