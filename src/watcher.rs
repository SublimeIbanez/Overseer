use crate::fs_node::*;
use std::{io, hash::Hash, path::PathBuf, fs::{metadata, Metadata}};
use serde::{Deserialize, Serialize};
use walkdir::{WalkDir, Error};
use simplicio::*;

#[derive(Debug)]
pub enum WatcherError {
    PathDoesNotExist,
    NotADirectory,
    InvalidDirectoryName,
    IOError(io::Error),
    NodeError(FsNodeError),
    WalkDirError(Error),
}

impl std::fmt::Display for WatcherError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            WatcherError::PathDoesNotExist => write!(f, "Path does not exist"),
            WatcherError::NotADirectory => write!(f, "The path is not a directory"),
            WatcherError::InvalidDirectoryName => write!(f, "Invalid directory name"),
            WatcherError::IOError(e) => write!(f, "{}", e),
            WatcherError::NodeError(e) => write!(f, "{}", e),
            WatcherError::WalkDirError(e) => write!(f, "{}", e),
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

    pub fn walk(&mut self) -> Result<&mut Watcher<K, V>, WatcherError> {
        self.dir_info.content = match dir_recurse(
            &self.path, self.ignore_hidden, &self.ignore_list
        ) {
            Ok(c) => c,
            Err(e) => return Err(e),
        };

        return Ok(self);
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

fn dir_recurse<K, V>(
    path: &PathBuf, ignore_hidden: bool, ignore_list: &[String]
) -> Result<Vec<FsNode<K, V>>, WatcherError> 
where 
    K: Hash + Eq + Clone + Serialize + for<'de> Deserialize<'de>, 
    V: Clone + Serialize + for<'de> Deserialize<'de>
{
    let mut content: Vec<FsNode<K, V>> = vec![];

    let walkdir = WalkDir::new(path)
        .min_depth(1).into_iter().filter_entry(|entry| {
            let mut skip = true;
            entry.path().file_name().unwrap().to_str()
                .map(|s| skip = !ignore_list.contains(&s.to_string()));
            skip
        });
    for entry in walkdir.filter(|e| e.is_ok()) {
        // println!("meep: {:?}", &entry);
        let node = entry.unwrap();

        let name = match node.file_name().to_str() {
            Some(n) => n,
            None => return Err(WatcherError::PathDoesNotExist),
        };

        let metadata = match metadata(node.path()) {
            Ok(m) => m,
            Err(e) => return Err(WatcherError::IOError(e)),
        };

        if is_hidden(&name, &metadata) {
            continue;
        }

        let last_modified = match metadata.modified() {
            Ok(time) => time,
            Err(e) => return Err(WatcherError::IOError(e)),
        };

        content.push(match node.path().is_dir() {
            true => {
                let sub_path = PathBuf::from(node.path());
                let content = match dir_recurse(&sub_path, ignore_hidden, ignore_list) {
                    Ok(c) => c,
                    Err(e) => return Err(e)
                };
                FsNode::Directory(DirInfo { 
                    name: s!(name), 
                    path: sub_path, 
                    last_modified: Some(last_modified), 
                    expanded: true, 
                    content,
                    fields: None, 
                })
            },
            false => {
                FsNode::File(FileInfo {
                    name: s!(name),
                    path: PathBuf::from(node.path()),
                    last_modified: Some(last_modified),
                    fields: None,
                })
            },
        });
    }


    return Ok(content);
}

#[allow(unused_variables)]
fn is_hidden(name: &str, metadata: &Metadata) -> bool {
    #[cfg(target_os = "linux")]
    {
        return name.starts_with('.');
    }
    #[cfg(target_os = "windows")]
    {
        return metadata.map(|m| m.file_attributes() & 0x2 != O)
        .unwrap_or(false);
    }
}
