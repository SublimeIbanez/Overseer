extern crate libc;
#[allow(unused_imports)]
use core::slice;
#[cfg(target_os = "linux")]
use std::{io::{Error, Write}, os::fd::IntoRawFd};
use serde::{Serialize, Deserialize};
#[cfg(target_os = "linux")]
use libc::{
    inotify_init1, 
    IN_NONBLOCK, IN_CLOEXEC, IN_MODIFY, IN_CREATE, IN_ACCESS, 
    IN_ATTRIB, IN_CLOSE_WRITE, IN_CLOSE_NOWRITE, IN_OPEN, 
    IN_MOVED_FROM, IN_MOVED_TO, IN_DELETE, IN_DELETE_SELF, IN_MOVE_SELF, IN_UNMOUNT, IN_Q_OVERFLOW, IN_IGNORED
};
#[allow(unused_imports)]
use simplicio::*;

// TODO: Implement for run-at-startup
// [Unit]
// Description=Add_Description_Here
//
// [Service]
// ExecStart=/path/to/program
// Restart=always
//
// [Install]
// WantedBy=multi-user.target

// Treat this library as a liaison between the daemon and the primary program
// TODO: Move this code to a small binary to run separately from this library and primary program
// reeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee
#[cfg(target_os = "linux")]
pub enum INotifyError {
    OSError(Error),
    IOError(Error),
    Utf8Error(std::str::Utf8Error),
}

#[repr(u32)]
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum Event {
    #[cfg(target_os = "linux")]
    Access = IN_ACCESS,              // 0x00000001   1
    #[cfg(target_os = "linux")]
    Modify = IN_MODIFY,              // 0x00000002   2
    #[cfg(target_os = "linux")]
    Attrib = IN_ATTRIB,              // 0x00000004   4
    #[cfg(target_os = "linux")]
    CloseWrite = IN_CLOSE_WRITE,     // 0x00000008   8
    #[cfg(target_os = "linux")]
    CloseNoWrite = IN_CLOSE_NOWRITE, // 0x00000010   16
    #[cfg(target_os = "linux")]
    Open = IN_OPEN,                  // 0x00000020   32
    #[cfg(target_os = "linux")]
    MovedFrom = IN_MOVED_FROM,       // 0x00000040   64
    #[cfg(target_os = "linux")]
    MovedTo = IN_MOVED_TO,           // 0x00000080   128
    #[cfg(target_os = "linux")]
    Create = IN_CREATE,              // 0x00000100   256
    #[cfg(target_os = "linux")]
    Delete = IN_DELETE,              // 0x00000200   512
    #[cfg(target_os = "linux")]
    DeleteSelf = IN_DELETE_SELF,     // 0x00000400   1024
    #[cfg(target_os = "linux")]
    MoveSelf = IN_MOVE_SELF,         // 0x00000800   2048
    #[cfg(target_os = "linux")]
    Unmount = IN_UNMOUNT,            // 0x00002000   8192
    #[cfg(target_os = "linux")]
    Overflow = IN_Q_OVERFLOW,        // 0x00004000   16384
    #[cfg(target_os = "linux")]
    Ignored = IN_IGNORED,            // 0x00008000   32768
    Uknown = 0,
}
#[cfg(target_os = "linux")]
impl std::fmt::Display for Event {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", *self as u32)
    }
}

#[cfg(target_os = "linux")]
impl From<u32> for Event {
    fn from(mask: u32) -> Self {
        match mask {
            1 | 2 | 4 | 8 | 16 | 32 | 64 | 128 | 256 | 512 | 
            1024 | 2048 | 8192 | 16384 | 32768 => unsafe { 
                std::mem::transmute(mask) 
            },
            _ => Self::Uknown, 
        }
        
        // match mask {
        //     IN_ACCESS => Self::Access,
        //     IN_MODIFY => Self::Modify,
        //     IN_ATTRIB => Self::Attrib,
        //     IN_CLOSE_WRITE => Self::CloseWrite,
        //     IN_CLOSE_NOWRITE => Self::CloseNoWrite,
        //     IN_OPEN => Self::Open,
        //     IN_MOVED_FROM => Self::MovedFrom,
        //     IN_MOVED_TO => Self::MovedTo,
        //     IN_CREATE => Self::Create,
        //     IN_DELETE => Self::Delete,
        //     IN_DELETE_SELF => Self::DeleteSelf,
        //     IN_MOVE_SELF => Self::MoveSelf,
        //     _ => Self::Uknown,
        // }
    }
}

#[cfg(target_os = "linux")]
impl std::ops::BitOr for Event {
    type Output = u32;

    fn bitor(self, rhs: Self) -> Self::Output {
        self | rhs 
    }
}


#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct INotify {
    pub(crate) id: i32,
    pub(crate) pid: i32,
    pub(crate) path: String,
    pub(crate) events: Vec<Event>,
    pub(crate) watch_ids: Vec<i32>,
}

#[cfg(target_os = "linux")]
impl INotify {
    pub(crate) fn new(path: &str) -> Result<Self, INotifyError> {
        let init = unsafe { inotify_init1(IN_NONBLOCK | IN_CLOEXEC) };
        let id = match init {
            -1 => return Err(INotifyError::OSError(Error::last_os_error())),
            _ => init,
        };
        Ok(Self {
            id,
            pid: -1,
            path: s!(path),
            events: vec![],
            watch_ids: vec![],
        })
    }

    pub(crate) fn add(&mut self, path: &str) -> Result<Self, INotifyError> {
        let c_path = std::ffi::CString::new(path)
            .expect("CString::new failed");

        let watch_id = unsafe { 
            libc::inotify_add_watch(
                self.id, 
                c_path.as_ptr(), 
                Event::Modify | Event::Create 
            )};
        self.watch_ids.push(watch_id);
        return Ok(self.clone())
    }

    /// Create a daemon to sit in the root path and catch the inotify calls
    /// Set up prior to inotify
    pub(crate) fn daemonize(&mut self) -> Result<Self, INotifyError> {
        unsafe { 
            // Fork program for daemon
            let pid = libc::fork();

            // Match on fork attempt
            match pid { 
                // Error creating a new process
                -1 => Err(INotifyError::OSError(Error::last_os_error())),
                // In newly created (child) process
                0 => {
                    // Start new session & detach child from program
                    libc::setsid();
                    // Allow creation of files with any permissions
                    libc::umask(0);
                    // Change child's current working directory to root
                    std::env::set_current_dir("/")
                        .map_err(|e| INotifyError::IOError(e))?;

                    // Create/Open the log file
                    let log = match std::fs::File::create(&self.path) {
                        Ok(num) => num.into_raw_fd(),
                        Err(e) => return Err(INotifyError::IOError(e)),
                    };

                    // dup2 duplicates /dev/null for redirecting stdin/out/error
                    libc::dup2(log, libc::STDIN_FILENO);
                    libc::dup2(log, libc::STDOUT_FILENO);
                    // libc::dup2(fd, libc::STDERR_FILENO);

                    self.pid = pid;
                    _ = self.listen();
                    return Ok(self.clone());
                },
                // Parent process
                _ => {
                    self.pid = pid;
                    return Ok(self.clone());
                },
            }
        }
    }

    pub(crate) fn listen(&self) -> Result<(), INotifyError> {
        let mut buffer = [0u8; 5120]; // Buffer for reading events 5kB
        loop {
            for id in self.watch_ids.clone().into_iter() {
                // Read the events in the watched directories, store in buffer
                let bytes_read = unsafe {
                    libc::read(
                        id, 
                        buffer.as_mut_ptr() as *mut _, 
                        buffer.len())};
                
                if bytes_read == -1 { // Error reading
                    return Err(INotifyError::OSError(Error::last_os_error()));
                } else if bytes_read > 0 { // Content exists
                    // Iterate through the buffer and read each item
                    let mut i = 0;
                    while i < bytes_read as usize {
                        let size = std::mem::size_of::<libc::inotify_event>();
                        let event = unsafe { &*(buffer.as_ptr() as *const libc::inotify_event) };
                        let mask = event.mask;
                        let file_name = unsafe { slice::from_raw_parts(
                                buffer.as_ptr().add(i + size), event.len as usize) };

                        let file_name = std::str::from_utf8(file_name)
                            .map_err(|e| INotifyError::Utf8Error(e))?;

                        let output = format!("{}|{}", Event::from(mask), file_name);

                        // Create/Open the log file
                        let mut log = match std::fs::OpenOptions::new()
                            .read(true).append(true).open(&self.path) {
                            Ok(log) => log,
                            Err(e) => return Err(INotifyError::IOError(e)),
                        };

                        write!(log, "{}", output).map_err(|e| INotifyError::IOError(e))?;


                        i += size + event.len as usize;
                    }
                }
            }
        }
    }
}

