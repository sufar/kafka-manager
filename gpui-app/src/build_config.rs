//! Build Configuration
//!
//! Provides build and packaging configuration for different platforms.

use std::path::PathBuf;

/// Build configuration
pub struct BuildConfig {
    /// Application name
    pub app_name: String,
    /// Version
    pub version: String,
    /// Binary name
    pub binary_name: String,
    /// Target platform
    pub target: BuildTarget,
    /// Output directory
    pub output_dir: PathBuf,
    /// Resources directory
    pub resources_dir: PathBuf,
}

/// Build target platform
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuildTarget {
    MacOS,
    MacOSArm,
    Windows,
    Linux,
    LinuxArm,
}

impl BuildConfig {
    /// Create default config
    pub fn default() -> Self {
        Self {
            app_name: "Kafka Manager".to_string(),
            version: "0.1.0".to_string(),
            binary_name: "kafka-manager-gpui".to_string(),
            target: BuildTarget::Linux,
            output_dir: PathBuf::from("target/release"),
            resources_dir: PathBuf::from("resources"),
        }
    }

    /// Get output file name for target
    pub fn output_filename(&self) -> String {
        match self.target {
            BuildTarget::MacOS | BuildTarget::MacOSArm => {
                format!("{}.app", self.app_name)
            }
            BuildTarget::Windows => {
                format!("{}.exe", self.binary_name)
            }
            BuildTarget::Linux | BuildTarget::LinuxArm => {
                self.binary_name.clone()
            }
        }
    }

    /// Get icon file for target
    pub fn icon_file(&self) -> Option<PathBuf> {
        match self.target {
            BuildTarget::MacOS | BuildTarget::MacOSArm => {
                Some(self.resources_dir.join("icon.icns"))
            }
            BuildTarget::Windows => {
                Some(self.resources_dir.join("icon.ico"))
            }
            BuildTarget::Linux | BuildTarget::LinuxArm => {
                Some(self.resources_dir.join("icon.png"))
            }
        }
    }

    /// Get target triple for Rust
    pub fn target_triple(&self) -> &'static str {
        match self.target {
            BuildTarget::MacOS => "x86_64-apple-darwin",
            BuildTarget::MacOSArm => "aarch64-apple-darwin",
            BuildTarget::Windows => "x86_64-pc-windows-msvc",
            BuildTarget::Linux => "x86_64-unknown-linux-gnu",
            BuildTarget::LinuxArm => "aarch64-unknown-linux-gnu",
        }
    }

    /// Get package format for target
    pub fn package_format(&self) -> PackageFormat {
        match self.target {
            BuildTarget::MacOS | BuildTarget::MacOSArm => PackageFormat::AppBundle,
            BuildTarget::Windows => PackageFormat::Exe,
            BuildTarget::Linux | BuildTarget::LinuxArm => PackageFormat::Binary,
        }
    }
}

/// Package format
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PackageFormat {
    AppBundle,
    Exe,
    Binary,
    Deb,
    Rpm,
    AppImage,
}

impl BuildTarget {
    /// Detect current target from environment
    pub fn current() -> Self {
        #[cfg(target_os = "macos")]
        {
            #[cfg(target_arch = "x86_64")]
            { BuildTarget::MacOS }
            #[cfg(target_arch = "aarch64")]
            { BuildTarget::MacOSArm }
        }
        #[cfg(target_os = "windows")]
        { BuildTarget::Windows }
        #[cfg(target_os = "linux")]
        {
            #[cfg(target_arch = "x86_64")]
            { BuildTarget::Linux }
            #[cfg(target_arch = "aarch64")]
            { BuildTarget::LinuxArm }
        }
    }
}