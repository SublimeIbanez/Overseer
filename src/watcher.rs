use crate::fs_node::*;
use std::{io, hash::Hash, marker::Send, path::PathBuf, fs::Metadata};
use serde::{Deserialize, Serialize};
use async_recursion::async_recursion;
use simplicio::*;
use tokio::fs;
#[cfg(target_os = "windows")]
use std::os::windows::fs::MetadataExt;

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
    K: Hash + Eq + Clone + Send + 'static + Serialize + for<'de> Deserialize<'de>, 
    V: Clone + Serialize + Send + 'static + for<'de> Deserialize<'de>
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
            &s!(path.display()), None, vec![], None
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
        let dir_path = self.path.clone();
        let ignore_hidden = self.ignore_hidden;
        let ignore_list = self.ignore_list.clone();
        println!("boop");

        let runtime = match tokio::runtime::Runtime::new() {
            Ok(rt) => rt,
            Err(e) => return Err(WatcherError::IOError(e)),
        };

        let dir_info = match runtime.block_on(dir_recurse_async(&dir_path, ignore_hidden, &ignore_list)) {
            Ok(content) => content,
            Err(e) => return Err(e),
        };

        self.dir_info = dir_info;
        return Ok(self);
    }

    pub fn build_tree(&self) -> Vec<String> {
        return self.dir_info.build_tree();
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


// use std::fs;

// struct FileNode {
//     path: String,
//     is_directory: bool,
// }

// fn walk_file_tree(root_path: &str) {
//     let mut stack = vec![FileNode {
//         path: root_path.to_string(),
//         is_directory: true,
//     }];

//     while let Some(current_node) = stack.pop() {
//         if current_node.is_directory {
//             // Process the directory
//             println!("Directory: {}", current_node.path);

//             // Get the list of files and directories in the current directory
//             if let Ok(contents) = fs::read_dir(&current_node.path) {
//                 // Push directories onto the stack
//                 for entry in contents {
//                     if let Ok(entry) = entry {
//                         let item_path = entry.path();
//                         let is_directory = item_path.is_dir();
//                         stack.push(FileNode {
//                             path: item_path.to_string_lossy().to_string(),
//                             is_directory,
//                         });
//                     }
//                 }
//             }
//         } else {
//             // Process the file
//             println!("File: {}", current_node.path);
//         }
//     }
// }

// fn main() {
//     // Example usage
//     walk_file_tree("/path/to/root");
// }

#[async_recursion]
async fn dir_recurse_async<K, V>(
    path: &PathBuf, ignore_hidden: bool, ignore_list: &[String]
) -> Result<DirInfo<K, V>, WatcherError> 
where 
    K: Hash + Eq + Clone + Serialize + for<'de> Deserialize<'de> + Send + 'static, 
    V: Clone + Serialize + for<'de> Deserialize<'de> + Send + 'static
{
    let mut content: Vec<FsNode<K, V>> = vec![];
    
    let mut dir = match fs::read_dir(path).await {
        Ok(d) => d,
        Err(e) => return Err(WatcherError::IOError(e)),
    };

    let dir_metadata = match path.metadata() {
        Ok(md) => md,
        Err(e) => return Err(WatcherError::IOError(e)),
    };

    while let Some(entry) = match dir.next_entry().await {
        Ok(entry) => entry,
        Err(e) => return Err(WatcherError::IOError(e)),
    } {
        let filetype = match entry.file_type().await {
            Ok(ft) => ft,
            Err(e) => return Err(WatcherError::IOError(e)),
        };

        let metadata = match entry.metadata().await {
            Ok(m) => m,
            Err(e) => return Err(WatcherError::IOError(e)),
        };

        let name = entry.file_name().to_string_lossy().into_owned();

        if (ignore_hidden && is_hidden(&name, &metadata)) || ignore_list.contains(&name) {
            continue;
        }

        let last_modified = match metadata.modified() {
            Ok(time) => time,
            Err(e) => return Err(WatcherError::IOError(e)),
        };

        content.push(match filetype.is_dir() {
            true => {
                let sub_path = entry.path();
                FsNode::Directory(
                    dir_recurse_async(&sub_path, ignore_hidden, ignore_list).await?
                )
            },
            false => {
                FsNode::File(FileInfo {
                    name,
                    path: entry.path(),
                    last_modified: Some(last_modified),
                    fields: None,
                })
            }
        });
    }
    let dir_name = match path.file_name() {
        Some(n) => {
            match n.to_str() {
                Some(dirn) => dirn.to_string(),
                None => return Err(WatcherError::PathDoesNotExist),
            }
        },
        None => return Err(WatcherError::PathDoesNotExist),
    };

    let last_modified = match dir_metadata.modified() {
        Ok(lm) => Some(lm),
        Err(e) => return Err(WatcherError::IOError(e)),
    };
    
    Ok(DirInfo { 
        name: dir_name, 
        path: path.to_owned(), 
        last_modified,
        content, 
        fields: Some(map!()), 
    })
}

#[allow(unused_variables)]
fn is_hidden(name: &str, metadata: &Metadata) -> bool {
    if name.starts_with('.') { return true; }
    #[cfg(target_os = "windows")]
    {
        return (metadata.file_attributes() & 0x2) != 0;
    }
    return false;
}

