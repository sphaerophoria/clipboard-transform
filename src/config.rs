use log::{debug, error};
use notify::{watcher, DebouncedEvent, RecursiveMode, Watcher};
use serde::Deserialize;
use thiserror::Error as ThisError;

use std::{
    fs::File,
    path::{Path, PathBuf},
    sync::mpsc::{self, Receiver},
    time::Duration,
};

#[derive(Deserialize, Debug)]
pub(crate) enum MappingState {
    Disabled,
    LeftToRight,
    RightToLeft,
}

impl Default for MappingState {
    fn default() -> MappingState {
        MappingState::Disabled
    }
}

#[derive(Deserialize, Debug)]
#[serde(tag = "type")]
#[serde(rename_all = "snake_case")]
pub enum Mapping {
    Simple(SimpleMapping),
}

#[derive(Deserialize, Debug)]
pub struct SimpleMapping {
    pub left: String,
    pub right: String,
    // FIXME: support default directions
    #[serde(skip)]
    pub(crate) state: MappingState,
}

impl SimpleMapping {
    pub(crate) fn get_source_str(&self) -> Option<&str> {
        match self.state {
            MappingState::LeftToRight => Some(&self.left),
            MappingState::RightToLeft => Some(&self.right),
            MappingState::Disabled => None,
        }
    }

    pub(crate) fn get_dest_str(&self) -> Option<&str> {
        match self.state {
            MappingState::LeftToRight => Some(&self.right),
            MappingState::RightToLeft => Some(&self.left),
            MappingState::Disabled => None,
        }
    }
}

#[derive(Debug, ThisError)]
pub enum Error {
    #[error(transparent)]
    WatchError(#[from] notify::Error),
    #[error(transparent)]
    FileError(#[from] std::io::Error),
    #[error(transparent)]
    ParseError(#[from] serde_json::Error),
    #[error(transparent)]
    RecvError(#[from] std::sync::mpsc::RecvError),
    #[error("No parent directory for config file")]
    NoParentDir,
}

pub struct ConfigMonitor {
    rx: Receiver<DebouncedEvent>,
    path: PathBuf,
    // Will stop sending events when destructed
    _watcher: notify::RecommendedWatcher,
}

impl ConfigMonitor {
    pub fn new(path: PathBuf) -> Result<ConfigMonitor, Error> {
        let (tx, rx) = mpsc::channel();
        let mut watcher = watcher(tx, Duration::from_secs(1)).unwrap();

        let watch_path = path.parent().ok_or(Error::NoParentDir)?;

        watcher.watch(watch_path, RecursiveMode::NonRecursive)?;

        Ok(ConfigMonitor {
            rx,
            path,
            _watcher: watcher,
        })
    }

    pub fn recv(&mut self) -> Result<Vec<Mapping>, Error> {
        loop {
            let event = match self.rx.recv() {
                Ok(event) => event,
                Err(e) => return Err(Error::from(e)),
            };

            debug!("{:?}", event);

            match event {
                DebouncedEvent::Create(path) | DebouncedEvent::Write(path) => {
                    if path == self.path {
                        return load_mappings(&path);
                    }
                }
                _ => (),
            }
        }
    }
}

pub fn load_mappings(path: &Path) -> Result<Vec<Mapping>, Error> {
    let f = File::open(path)?;
    let mappings: Vec<Mapping> = serde_json::de::from_reader(f)?;
    Ok(mappings)
}
