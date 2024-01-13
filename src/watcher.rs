use crate::fs_node::*;
use serde::{Deserialize, Serialize};
use std::{io, hash::Hash, path::PathBuf};
use simplicio::*;

#[derive(Debug)]
pub enum WatcherError {
    PathDoesNotExist,
    NotADirectory,
    InvalidDirectoryName,
    IOError(io::Error),
    NodeError(FsNodeError),
}

impl std::fmt::Display for WatcherError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            WatcherError::PathDoesNotExist => write!(f, "Path does not exist"),
            WatcherError::NotADirectory => write!(f, "The path is not a directory"),
            WatcherError::InvalidDirectoryName => write!(f, "Invalid directory name"),
            WatcherError::IOError(e) => write!(f, "{}", e),
            WatcherError::NodeError(e) => write!(f, "{}", e),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Watcher<K, V> 
where 
    K: Hash + Eq + Clone + Serialize, 
    V: Clone + Serialize 
{
    pub dir_name: String,
    pub path: PathBuf,
    pub ignore_hidden: bool,
    pub ignore_list: Vec<String>,
    pub dir_info: DirInfo<K, V>
}

impl<K, V> Watcher<K, V> 
where 
    K: Hash + Eq + Clone + Serialize + for<'de> Deserialize<'de>, 
    V: Clone + Serialize + for<'de> Deserialize<'de>
{
    pub fn new(input: &str) -> Result<Self, WatcherError> {
        let path = if input.is_empty() {
            std::env::current_dir().map_err(|_| WatcherError::PathDoesNotExist)?
        } else { PathBuf::from(input) };

        if !path.exists() { return Err(WatcherError::PathDoesNotExist); }
        if !path.is_dir() { return Err(WatcherError::NotADirectory); }

        let dir_name = if let Some(name) = path.file_name()
            .and_then(|n| n.to_str()) {
            name.to_owned()
        } else {
            return Err(WatcherError::InvalidDirectoryName);
        };

        let dir_info: DirInfo<K, V> = DirInfo::new(
            &s!(path.display()), None, true, vec![], None
        ).map_err(|e| WatcherError::NodeError(e))?;

        Ok(Self {
            dir_name,
            path,
            ignore_hidden: true,
            ignore_list: vec![],
            dir_info,
        })
    }

    pub fn from(dir_info: DirInfo<K, V>) -> Result<Watcher<K, V>, WatcherError> {
        let path = if dir_info.path.as_os_str().is_empty() {
            std::env::current_dir().map_err(|_| WatcherError::PathDoesNotExist)?
        } else { dir_info.path.clone() };

        if !path.exists() { return Err(WatcherError::PathDoesNotExist); }
        if !path.is_dir() { return Err(WatcherError::NotADirectory); }

        let dir_name = if let Some(name) = path.file_name()
            .and_then(|n| n.to_str()) {
            name.to_owned()
        } else {
            return Err(WatcherError::InvalidDirectoryName);
        };

        Ok(Watcher {
            dir_name,
            path,
            ignore_hidden: true,
            ignore_list: vec![],
            dir_info,
        })
    }

    pub fn path_string(&self) -> String {
        return s!(self.path.display());
    }

    pub fn ignore_reset(&mut self) -> &mut Watcher<K, V> {
        self.ignore_list = vec![];
        return self;
    }

    pub fn add_ignore(&mut self, item: &str) -> &mut Watcher<K, V> {
        self.ignore_list.push(s!(item));
        return self;
    }

    pub fn remove_ignore(&mut self, item: &str) -> &mut Watcher<K, V> {
        self.ignore_list.retain(|i| i != item);
        return self;
    }

    pub fn set_dir_info(&mut self, info: DirInfo<K, V>) -> &mut Watcher<K, V> {
        self.dir_info = info;
        return self;
    }

    pub fn save(&self) -> io::Result<()> {
        let mut path = self.path.clone();
        path.push(".watcher");
        let data = bincode::serialize(self)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

        std::fs::write(path, data)?;

        return Ok(());
    }

    pub fn load(input: &str) -> Result<Self, WatcherError> {
        let mut path = if input.is_empty() {
            std::env::current_dir()
                .map_err(|_| WatcherError::PathDoesNotExist)?
        } else { PathBuf::from(input) };
        path.push(".watcher");

        let data = std::fs::read(path).map_err(|e| WatcherError::IOError(e))?;

        let watcher = bincode::deserialize(&data)
            .map_err(|e| WatcherError::IOError(
                io::Error::new(io::ErrorKind::Other, e)))?;

        return Ok(watcher);
    }
}

