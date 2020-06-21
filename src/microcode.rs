//! Microcode bundle generation
//!
//! This module provides an API to help generating a microcode bundle
//! for early loading by the Linux kernel according to its initramfs
//! specification.

use crate::config::Microcode;
use crate::newc::Archive;
use crate::utils;

use anyhow::Result;
use flate2::write::GzEncoder;
use flate2::Compression;
use log::{info, warn};
use std::fs::File;
use std::path::{Path, PathBuf};
use std::{fs, io};
use tempfile::TempDir;

/// Path where the blobs will be searched by the Linux kernel
const UCODE_TREE: &str = "kernel/x86/microcode";

/// Name of the microcode blob for AMD
const AMD_UCODE_NAME: &str = "AuthenticAMD.bin";
/// Name of the microcode blob for Intel
const INTEL_UCODE_NAME: &str = "GenuineIntel.bin";

/// Builder pattern for microcode bundle generation
pub(crate) struct Builder {
    /// Optional path to AMD specific blobs
    amd: Option<PathBuf>,
    /// Optional path to Intel specific blobs
    intel: Option<PathBuf>,
}

impl Builder {
    /// Create a new builder
    pub(crate) fn new() -> Result<Self> {
        Ok(Builder {
            amd: None,
            intel: None,
        })
    }

    /// Create a new builder from a configuration
    pub(crate) fn from_config(config: Microcode) -> Result<Self> {
        let mut builder = Builder::new()?;

        builder.amd = config.amd;
        builder.intel = config.intel;

        Ok(builder)
    }

    /// Build the microcode bundle by writing all entries to a temporary directory
    /// and the walking it to create the compressed cpio archive
    pub(crate) fn build<P>(self, output: P) -> Result<()>
    where
        P: AsRef<Path>,
    {
        let output = output.as_ref();
        info!("Writing microcode cpio to: {}", output.to_string_lossy());

        let tmp = TempDir::new()?;
        let tmp_path = tmp.path();

        let ucode_tree = tmp_path.join(UCODE_TREE);
        fs::create_dir_all(&ucode_tree)?;

        if let (None, None) = (&self.amd, &self.intel) {
            warn!("Nothing to do...");
            return Ok(());
        }

        if let Some(amd) = &self.amd {
            add_amd(amd, &ucode_tree)?;
        }

        if let Some(intel) = &self.intel {
            add_intel(intel, &ucode_tree)?;
        }

        let output_file = utils::maybe_stdout(&output)?;
        let mut encoder = GzEncoder::new(output_file, Compression::default());
        Archive::from_root(tmp_path, &mut encoder)?;

        Ok(())
    }
}

/// Add the AMD specific blob to the bundle
fn add_amd<P>(dir: &Path, output: P) -> Result<()>
where
    P: AsRef<Path>,
{
    let output = output.as_ref().join(AMD_UCODE_NAME);
    bundle_ucode(dir, output)
}

/// Add the Intel specific blob to the bundle
fn add_intel<P>(dir: &Path, output: P) -> Result<()>
where
    P: AsRef<Path>,
{
    let output = output.as_ref().join(INTEL_UCODE_NAME);
    bundle_ucode(dir, output)
}

/// Bundle multiple vendor specific microcode blobs into a single blob
fn bundle_ucode<P>(dir: &Path, output: P) -> Result<()>
where
    P: AsRef<Path>,
{
    let mut output_file = File::create(output.as_ref())?;

    for entry in fs::read_dir(dir)? {
        let entry = entry?;

        if entry.file_type()?.is_file() {
            let mut file = File::open(entry.path())?;
            io::copy(&mut file, &mut output_file)?;
        }
    }

    Ok(())
}
