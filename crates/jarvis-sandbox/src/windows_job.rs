#[derive(Debug, Clone)]
pub struct WindowsJobPolicy {
    pub max_processes: u32,
    pub kill_on_close: bool,
    pub memory_limit_bytes: Option<usize>,
}

impl Default for WindowsJobPolicy {
    fn default() -> Self {
        Self {
            max_processes: 16,
            kill_on_close: true,
            memory_limit_bytes: None,
        }
    }
}

#[cfg(windows)]
pub fn job_objects_supported() -> bool {
    true
}

#[cfg(not(windows))]
pub fn job_objects_supported() -> bool {
    false
}
