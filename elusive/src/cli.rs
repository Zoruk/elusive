use crate::config::Config;
use crate::encoder::Encoder;
use crate::initramfs::InitramfsBuilder;
use crate::io::{Input, Output};
use crate::microcode::MicrocodeBundle;

use anyhow::{bail, Result};
use clap::{Parser, Subcommand};
use log::{error, info};
use std::fs::File;
use std::io::Read;
use std::io::{BufReader, BufWriter};
use std::path::PathBuf;
use std::{fs, io};

/// Default path for the config file
const CONFIG_PATH: &str = "/etc/elusive.toml";
const CONFDIR_PATH: &str = "/etc/elusive.d";

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
pub struct Args {
    /// Path to the configuration file
    #[clap(short, long)]
    #[clap(global = true)]
    pub config: Option<PathBuf>,
    /// Path to the configuration directory
    #[clap(short = 'C', long)]
    #[clap(global = true)]
    pub confdir: Option<PathBuf>,
    #[clap(short, long)]
    #[clap(global = true)]
    /// Encoder to use for compression
    pub encoder: Option<Encoder>,
    #[clap(subcommand)]
    pub command: Command,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// Generate a compressed cpio archive to use as initramfs
    Initramfs {
        /// Microcode archive to include
        #[clap(short, long)]
        ucode: Option<PathBuf>,
        /// Path to the kernel module source directory
        #[clap(short, long)]
        modules: Option<PathBuf>,
        /// Path where the initramfs will be written
        #[clap(short, long)]
        output: PathBuf,
    },
    /// Generate a compressed cpio archive for CPU microcode
    Microcode {
        /// Path where the microcode archive will be written
        #[clap(short, long)]
        output: PathBuf,
    },
}

/// Entrypoint of the program
pub fn elusive(args: Args) -> Result<()> {
    let Args {
        config,
        confdir,
        encoder,
        command,
    } = args;

    let config = config.unwrap_or_else(|| PathBuf::from(CONFIG_PATH));
    let confdir = confdir.unwrap_or_else(|| PathBuf::from(CONFDIR_PATH));
    let encoder = encoder.unwrap_or(Encoder::Zstd);

    let mut buf = Vec::new();

    if config.exists() && config.is_file() {
        File::open(&config)?.read_to_end(&mut buf)?;
    }

    if confdir.exists() && confdir.is_dir() {
        for entry in fs::read_dir(&confdir)? {
            let entry = entry?;
            let path = entry.path();

            if path.exists() && path.is_file() {
                File::open(path)?.read_to_end(&mut buf)?;
            }
        }
    }

    if buf.is_empty() {
        bail!(
            "configuration was file or directory was found in {}, {}",
            config.display(),
            confdir.display(),
        );
    }

    let config: Config = toml::from_slice(&buf)?;

    match command {
        Command::Initramfs {
            ucode,
            modules,
            output,
        } => {
            if let Some(config) = config.initramfs {
                let initramfs = InitramfsBuilder::from_config(config, modules.as_deref())?.build();

                info!("Writing initramfs to: {}", output.display());
                let write = Output::from_path(output)?;
                let mut write = BufWriter::new(write);

                if let Some(path) = ucode {
                    info!("Adding microcode bundle from: {}", path.display());

                    let read = Input::from_path(path)?;
                    let mut read = BufReader::new(read);

                    io::copy(&mut read, &mut write)?;
                }

                encoder.encode_archive(initramfs.into_archive(), write)?;
            } else {
                error!("No configuration provided for initramfs generation");
                bail!("configuration was empty");
            }
        }
        Command::Microcode { output } => {
            if let Some(config) = config.microcode {
                let bundle = MicrocodeBundle::from_config(config)?;

                info!("Writing microcode cpio to: {}", output.display());
                let write = Output::from_path(output)?;
                let write = BufWriter::new(write);

                encoder.encode_archive(bundle.build(), write)?;
            } else {
                error!("No configuration provided for microcode generation");
                bail!("configuration was empty");
            }
        }
    }

    Ok(())
}
