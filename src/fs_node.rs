use std::{
    cmp::Ordering, collections::HashMap, hash::Hash, path::{Path, PathBuf}, 
    time::SystemTime, 
};
use serde::{Deserialize, Serialize};
use dekor::*;
use simplicio::*;

#[derive(Debug)]
pub enum FsNodeError {
    PathDoesNotExist,
    IncorrectFSType,
    InvalidName,
}

impl std::fmt::Display for FsNodeError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            FsNodeError::PathDoesNotExist => write!(f, "Path does not exist"),
            FsNodeError::IncorrectFSType => write!(f, "Incorrect filesystem type"),
            FsNodeError::InvalidName => write!(f, "Invalid name was provided"),
        }
    }
}


#[derive(Debug, Deserialize, Serialize)]
pub enum FsNode<K, V> where K: Hash + Eq + Clone, V: Clone {
    Directory(DirInfo<K, V>),
    File(FileInfo<K, V>),
}

impl<K, V> FsNode<K, V> where K: Hash + Eq + Clone, V: Clone {
    pub fn is_dir(&self) -> bool {
        match self {
            FsNode::Directory(_) => true,
            FsNode::File(_) => false,
        }
    }

    pub fn name(&self) -> String {
        match self {
            FsNode::Directory(d) => s!(d.name),
            FsNode::File(f) => s!(f.name),
        }
    }

    pub fn path(&self) -> PathBuf {
        match self {
            FsNode::Directory(d) => d.path.clone(),
            FsNode::File(f) => f.path.clone(),
        }
    }
}

impl<K, V> Clone for FsNode<K, V> where K: Hash + Eq + Clone, V: Clone {
    fn clone(&self) -> Self {
        match self {
            FsNode::Directory(d) => FsNode::Directory(d.clone()),
            FsNode::File(f) => FsNode::File(f.clone()),
        }
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct DirInfo<K, V> where K: Hash + Eq + Clone, V: Clone {
    pub name: String,
    pub path: PathBuf,
    pub last_modified: Option<SystemTime>,
    pub expanded: bool,
    pub content: Vec<FsNode<K, V>>,
    pub fields: Option<HashMap<K, V>>,
}

impl<K, V> DirInfo<K, V> where K: Hash + Eq + Clone, V: Clone {
    pub fn new(
        path: &str, last_modified: Option<SystemTime>, expanded: bool,
        content: Vec<FsNode<K, V>>, fields: Option<HashMap<K, V>>
    ) -> Result<Self, FsNodeError> {
        let path = if path.is_empty() {
            std::env::current_dir().map_err(|_| FsNodeError::PathDoesNotExist)?
        } else { PathBuf::from(path) };

        if !path.exists() { return Err(FsNodeError::PathDoesNotExist); }
        if !path.is_dir() { return Err(FsNodeError::IncorrectFSType); }

        let name = if let Some(n) = path.file_name()
            .and_then(|n| n.to_str()) {
            n.to_owned()
        } else {
            return Err(FsNodeError::InvalidName);
        };
        Ok(Self {
            name: s!(name), 
            path, 
            last_modified, 
            expanded, 
            content, 
            fields,
        })
    }

    pub fn from(path: &str) -> Result<DirInfo<K, V>, FsNodeError> {
        let path = if path.is_empty() {
            std::env::current_dir().map_err(|_| FsNodeError::PathDoesNotExist)?
        } else { PathBuf::from(path) };

        if !path.exists() { return Err(FsNodeError::PathDoesNotExist); }
        if !path.is_dir() { return Err(FsNodeError::IncorrectFSType); }

        let name = if let Some(n) = path.file_name()
            .and_then(|n| n.to_str()) {
            n.to_owned()
        } else {
            return Err(FsNodeError::InvalidName);
        };
        Ok(Self {
            name: s!(name), 
            path, 
            last_modified: None, 
            expanded: true, 
            content: vec![], 
            fields: None,
        })
    }

    pub fn set_path(&mut self, path: &str) -> &mut Self {
        self.path = PathBuf::from(path);
        return self;
    }

    pub fn path_string(&self) -> String {
        return s!(self.path.display());
    }

    pub fn parent(&self) -> Option<&Path> {
        return self.path.parent();
    } 

    pub fn set_last_modified(&mut self, last_modified: SystemTime) -> &mut Self {
        self.last_modified = Some(last_modified);
        return self;
    }

    /// let dir_info = DirInfo::new(/*...*/);
    /// let time_now = std::time::SystemTime::now();
    ///
    /// // Returns Some(Less) if time_now is less than dir_info.time
    /// // Returns Some(Equal) if time_now is equal to dir_info.time
    /// // Returns Some(Greater) if time_now is greater than dir_info.time
    /// // Returns `None` if `dir_info` does not have a set `last_modified`
    /// let compared_time = dir_info.cmp(time_now);
    pub fn cmp(&self, time: &SystemTime) -> Option<Ordering> {
        match self.last_modified {
            Some(last_time) => return Some(last_time.cmp(time)),
            None => None,
        }
    }

    pub fn expand(&mut self) -> &mut Self {
        self.expanded = true;
        return self;
    }

    pub fn unexpand(&mut self) -> &mut Self {
        self.expanded = false;
        return self;
    }

    pub fn expanded(&mut self, value: bool) -> &mut Self {
        self.expanded = value;
        return self;
    }

    pub fn set_content(&mut self, content: Vec<FsNode<K, V>>) -> &mut Self {
        self.content = content;
        return self;
    }

    pub fn insert(&mut self, content: FsNode<K, V>) -> &mut Self {
        self.content.push(content);
        return self;
    }

    pub fn remove(&mut self, path: PathBuf) {
        self.content.retain(|n| n.path() != path);
    }

    pub fn set_fields(&mut self, fields: Option<HashMap<K, V>>) -> &mut Self {
        self.fields = fields;
        return self;
    }

    pub fn add_field(&mut self, key: K, value: V) -> &mut Self {
        match self.fields.as_mut() {
            Some(map) => { map.insert(key, value); },
            None => self.fields = Some(map!(key : value)),
        }
        return self.into();
    }

    pub fn build(&self) -> Self {
        return self.clone();
    }
}

impl<K, V> Clone for DirInfo<K, V> where K: Hash + Eq + Clone, V: Clone {
    fn clone(&self) -> Self {
        Self {
            name: s!(self.name),
            path: self.path.clone(),
            last_modified: self.last_modified,
            expanded: self.expanded,
            content: self.content.clone(),
            fields: self.fields.clone(),
        }
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct FileInfo<K, V> where K: Hash + Eq + Clone, V: Clone {
    pub name: String,
    pub path: PathBuf,
    pub last_modified: Option<SystemTime>,
    pub fields: Option<HashMap<K, V>>,
}

impl<K, V> FileInfo<K, V> where K: Hash + Eq + Clone, V: Clone {
    pub fn new(
        name: String, path: PathBuf, last_modified: 
        Option<SystemTime>, fields: Option<HashMap<K, V>>
    ) -> Self {
        Self {
            name, path, last_modified, fields,
        }
    }

    pub fn set_name(&mut self, name: &str) -> &mut Self {
        self.name = s!(name);
        return self;
    }

    pub fn set_path(&mut self, path: &str) -> &mut Self {
        self.path = PathBuf::from(path);
        return self;
    }

    pub fn path_string(&self) -> String {
        return s!(self.path.display());
    }

    pub fn parent(&self) -> Option<&Path> {
        return self.path.parent();
    } 

    pub fn set_last_modified(&mut self, last_modified: SystemTime) -> &mut Self {
        self.last_modified = Some(last_modified);
        return self;
    }

    /// let dir_info = DirInfo::new(/*...*/);
    /// let time_now = std::time::SystemTime::now();
    ///
    /// // Returns Some(Less) if time_now is less than dir_info.time
    /// // Returns Some(Equal) if time_now is equal to dir_info.time
    /// // Returns Some(Greater) if time_now is greater than dir_info.time
    /// // Returns `None` if `dir_info` does not have a set `last_modified`
    /// let compared_time = dir_info.cmp(time_now);
    pub fn cmp(&self, time: &SystemTime) -> Option<Ordering> {
        match self.last_modified {
            Some(last_time) => return Some(last_time.cmp(time)),
            None => None,
        }
    }

    pub fn add_field(&mut self, key: K, value: V) -> &mut Self {
        match self.fields.as_mut() {
            Some(map) => { map.insert(key, value); },
            None => self.fields = Some(map!(key : value)),
        }
        return self.into();
    }

    pub fn build(&self) -> Self {
        return self.clone();
    }
}

impl<K, V> Clone for FileInfo<K, V> where K: Hash + Eq + Clone, V: Clone {
    fn clone(&self) -> Self {
        Self {
            name: s!(self.name),
            path: self.path.clone(),
            last_modified: self.last_modified,
            fields: self.fields.clone(),
        }
    }
}








pub fn build_tree<K: Hash + Eq + Clone, V: Clone>(dir_info: &DirInfo<K, V>) -> Vec<String> {
    let mut tree: Vec<String> = Vec::new();

    tree.push(s!(
        "[", 
        style!(Bold, FGGreen => 
            if dir_info.expanded { Utf8::ModLetterDownArrowhead }
            else { Utf8::ModLetterRightArrowhead }), // ˅ ˃
        "]",
        style!(Bold, FGBlue => dir_info.name),
    ));
    tree_recursion(dir_info, s!(), &mut tree);
    tree
}

fn tree_recursion<K: Hash + Eq + Clone, V: Clone>(
    dir_info: &DirInfo<K, V>, path: String, tree: &mut Vec<String>
) {
    //Force files first
    //TODO: make a config choice if folders or files first
    let (mut contents, other_content): (Vec<_>, Vec<_>) = dir_info
        .content
        .iter()
        .partition(|n| matches!(n, FsNode::File(_)));
    contents.extend(other_content);

    //Set up the formatted values
    let joint = format!(" {}{}", 
        Utf8::JointPipeSlim, Utf8::HPipeSlim.repeat(2));
    let node = format!(" {}{}", 
        Utf8::NodePipeSlim, Utf8::HPipeSlim.repeat(2));
    let vline = format!(" {}  ", Utf8::VPipeSlim);

    //Iterate through contents and add them to the tree
    let contents_len = contents.len();
    for (index, entity) in contents.iter().enumerate() {
        //Determine if the current entity is last
        let is_last = index == contents_len - 1;
        //Create the prefix
        let prefix = format!("{}{}", path, if is_last { &node } else { &joint });

        match entity {
            FsNode::File(file) => tree.push(prefix.clone() + " " + &file.name),
            FsNode::Directory(subdir) => {
                tree.push(s!(
                    prefix.clone(),
                    "[", 
                    style!(Bold, FGGreen => 
                        if dir_info.expanded { Utf8::ModLetterDownArrowhead } 
                        else { Utf8::ModLetterRightArrowhead }), // ˅ ˃
                    "]",
                    style!(Bold, FGBlue => subdir.name),
                ));
                // tree.push(format!(
                //     "{}{}{}{}{}",
                //     prefix.clone(),
                //     "[",
                //     expanded_color
                //         .paint(if subdir.expanded { "˅" } else { "˃" })
                //         .to_string()
                //         .as_str(),
                //     bracket_color.paint("]").to_string().as_str(),
                //     dir_color.paint(&subdir.name).to_string().as_str(),
                // ));

                //Recursively process expanded directories
                let sub_path = if is_last {
                    path.clone() + "    "
                } else {
                    path.clone() + &vline
                };
                if subdir.expanded {
                    tree_recursion(subdir, sub_path, tree);
                }
            }
        }
    }
}
    
