pub use crate::{
    config::{load_mappings, ConfigMonitor, Mapping},
    tray::MonitorTray,
};

use log::{error, info};
use thiserror::Error as ThisError;
use x11_clipboard::{error::Error as X11Error, Clipboard};

use std::{str::Utf8Error, sync::Mutex};

mod config;
mod tray;

#[derive(Debug, ThisError)]
pub enum Error {
    #[error("Failed to create clipboard")]
    ClipboardCreationError(#[source] X11Error),
    #[error(transparent)]
    ClipboardError(#[from] X11Error),
    #[error("Clipboard did not have a valid utf8 string")]
    ClipboardUtf8Error(#[source] Utf8Error),
}

pub struct Monitor {
    clipboard: Clipboard,
}

impl Monitor {
    pub fn new() -> Result<Monitor, Error> {
        let ret = Monitor {
            clipboard: Clipboard::new().map_err(Error::ClipboardCreationError)?,
        };

        Ok(ret)
    }

    pub fn run(&self, mappings: &Mutex<Vec<Mapping>>) {
        while let Err(e) = self.run_failable(mappings) {
            error!("{}", e);
        }
    }

    fn run_failable(&self, mappings: &Mutex<Vec<Mapping>>) -> Result<(), Error> {
        loop {
            // FIXME: Monitor both clipboards
            let buf = self.clipboard.load_wait(
                self.clipboard.getter.atoms.clipboard,
                self.clipboard.getter.atoms.utf8_string,
                self.clipboard.getter.atoms.property,
            )?;

            let clipboard_str = std::str::from_utf8(&buf).map_err(Error::ClipboardUtf8Error)?;

            info!("{}", clipboard_str);

            let mappings = mappings.lock().expect("Poisoned mappings");

            let mapping = mappings
                .iter()
                .find(|mapping| mapping_matches(mapping, clipboard_str));

            if let Some(Mapping::Simple(mapping)) = mapping {
                let mapping_source = mapping
                    .get_source_str()
                    .expect("Mapping found without source str");
                let mapping_dest = mapping
                    .get_dest_str()
                    .expect("Mapping found without dest str");
                let clipboard_replacement = clipboard_str.replacen(mapping_source, mapping_dest, 1);

                info!("Replacing clipboard text with {}", clipboard_replacement);

                self.clipboard.store(
                    self.clipboard.getter.atoms.clipboard,
                    self.clipboard.getter.atoms.utf8_string,
                    clipboard_replacement.into_bytes(),
                )?;

                // FIXME: Racey? Clear the just set value
                let _ = self.clipboard.load_wait(
                    self.clipboard.getter.atoms.clipboard,
                    self.clipboard.getter.atoms.utf8_string,
                    self.clipboard.getter.atoms.property,
                )?;
            }
        }
    }
}

fn mapping_matches(mapping: &Mapping, s: &str) -> bool {
    match mapping {
        Mapping::Simple(simple_mapping) => {
            let mapping_source = match simple_mapping.get_source_str() {
                Some(x) => x,
                None => return false,
            };

            s.starts_with(mapping_source)
        }
    }
}
