#[derive(Debug, Clone)]
pub struct ProcessEntry {
    pub pid: u32,
    pub name: String,
    pub cpu: Option<f64>,
    pub user: Option<String>,
}

#[derive(Debug)]
pub enum TaskError {
    CommandFailed(String),
    NotFound,
    PermissionDenied,
    InvalidArgs(String),
    Unsupported(String),
}

impl std::fmt::Display for TaskError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TaskError::CommandFailed(s) => write!(f, "command failed: {}", s),
            TaskError::NotFound => write!(f, "command not found"),
            TaskError::PermissionDenied => write!(f, "permission denied"),
            TaskError::InvalidArgs(s) => write!(f, "invalid arguments: {}", s),
            TaskError::Unsupported(s) => write!(f, "unsupported: {}", s),
        }
    }
}

impl std::error::Error for TaskError {}

pub trait Platform {
    fn whoami(&self) -> Result<String, TaskError>;
    fn hostname(&self) -> Result<String, TaskError>;
    fn pwd(&self) -> Result<String, TaskError>;
    fn ps(&self) -> Result<Vec<ProcessEntry>, TaskError>;
    fn ls(&self, path: &str) -> Result<String, TaskError>;
    fn echo(&self, args: &[String]) -> Result<String, TaskError>;
    fn sleep(&self, secs: u64) -> Result<String, TaskError>;
    fn shell(&self, cmd: &str) -> Result<String, TaskError>;
    fn id(&self) -> Result<String, TaskError>;
    fn uname(&self) -> Result<String, TaskError>;
    fn whoami_all(&self) -> Result<String, TaskError>;
    fn cred_access_check(&self) -> Result<String, TaskError>;
    fn list_env(&self) -> Result<String, TaskError>;
    fn list_ssh(&self) -> Result<String, TaskError>;
    fn collection(&self, path: &str) -> Result<String, TaskError>;
}

#[cfg(windows)]
mod windows;

#[cfg(windows)]
pub use windows::WindowsPlatform;

#[cfg(unix)]
mod unix;

#[cfg(unix)]
pub use unix::UnixPlatform;

mod task;
pub use task::Task;

pub fn platform() -> impl Platform {
    #[cfg(windows)]
    {
        WindowsPlatform
    }
    #[cfg(unix)]
    {
        UnixPlatform
    }
}
