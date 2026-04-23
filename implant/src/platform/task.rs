use super::{Platform, ProcessEntry, TaskError};

#[derive(Debug, Clone)]
pub enum Task {
    Whoami,
    Hostname,
    Pwd,
    ProcessList,
    Ls(String),
    Echo(Vec<String>),
    Sleep(u64),
    Shell(String),
    Id,
    Uname,
    WhoamiAll,
    CredAccessCheck,
    ListEnv,
    ListSsh,
    Collection(String),
    Shutdown,
}

impl Task {
    pub fn from_command(command: &str, args: &[String]) -> Result<Self, String> {
        match command {
            "whoami" => Ok(Task::Whoami),
            "hostname" => Ok(Task::Hostname),
            "pwd" => Ok(Task::Pwd),
            "ps" => Ok(Task::ProcessList),
            "ls" => Ok(Task::Ls(
                args.first().cloned().unwrap_or_else(|| ".".to_string()),
            )),
            "echo" => Ok(Task::Echo(args.to_vec())),
            "sleep" => {
                let secs = args.first().and_then(|s| s.parse().ok()).unwrap_or(1);
                Ok(Task::Sleep(secs))
            }
            "shell" => {
                if args.is_empty() {
                    return Err("shell command required".to_string());
                }
                Ok(Task::Shell(args.join(" ")))
            }
            "id" => Ok(Task::Id),
            "uname" => Ok(Task::Uname),
            "whoami_all" => Ok(Task::WhoamiAll),
            "cred-access-check" => Ok(Task::CredAccessCheck),
            "list-env" => Ok(Task::ListEnv),
            "list-ssh" => Ok(Task::ListSsh),
            "collect" | "enumerate" => Ok(Task::Collection(
                args.first().cloned().unwrap_or_else(|| "/".to_string()),
            )),
            "__shutdown" => Ok(Task::Shutdown),
            _ => Err(format!("unknown command: {}", command)),
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            Task::Whoami => "whoami",
            Task::Hostname => "hostname",
            Task::Pwd => "pwd",
            Task::ProcessList => "ps",
            Task::Ls(_) => "ls",
            Task::Echo(_) => "echo",
            Task::Sleep(_) => "sleep",
            Task::Shell(_) => "shell",
            Task::Id => "id",
            Task::Uname => "uname",
            Task::WhoamiAll => "whoami_all",
            Task::CredAccessCheck => "cred-access-check",
            Task::ListEnv => "list-env",
            Task::ListSsh => "list-ssh",
            Task::Collection(_) => "collect",
            Task::Shutdown => "__shutdown",
        }
    }

    pub fn mitre_id(&self) -> Option<&'static str> {
        match self {
            Task::Whoami => Some("T1033"),
            Task::Hostname => Some("T1106"),
            Task::Pwd => Some("T1083"),
            Task::ProcessList => Some("T1057"),
            Task::Ls(_) => Some("T1083"),
            Task::Echo(_) => None,
            Task::Sleep(_) => None,
            Task::Shell(_) => Some("T1059"),
            Task::Id => Some("T1033"),
            Task::Uname => Some("T1082"),
            Task::WhoamiAll => Some("T1033"),
            Task::CredAccessCheck => Some("T1003"),
            Task::ListEnv => Some("T1082"),
            Task::ListSsh => Some("T1082"),
            Task::Collection(_) => Some("T1074"),
            Task::Shutdown => None,
        }
    }

    pub fn execute(&self, platform: &impl Platform) -> Result<String, TaskError> {
        match self {
            Task::Whoami => platform.whoami(),
            Task::Hostname => platform.hostname(),
            Task::Pwd => platform.pwd(),
            Task::ProcessList => {
                let entries = platform.ps()?;
                Ok(format_ps_output(entries))
            }
            Task::Ls(path) => platform.ls(path),
            Task::Echo(args) => platform.echo(args),
            Task::Sleep(secs) => platform.sleep(*secs),
            Task::Shell(cmd) => platform.shell(cmd),
            Task::Id => platform.id(),
            Task::Uname => platform.uname(),
            Task::WhoamiAll => platform.whoami_all(),
            Task::CredAccessCheck => platform.cred_access_check(),
            Task::ListEnv => platform.list_env(),
            Task::ListSsh => platform.list_ssh(),
            Task::Collection(path) => platform.collection(path),
            Task::Shutdown => Ok("__shutdown__".to_string()),
        }
    }
}

fn format_ps_output(entries: Vec<ProcessEntry>) -> String {
    let mut out = String::from("    PID NAME                        CPU USER\n");
    out.push_str("    -- ----                        --- ----\n");
    for e in entries.iter().take(20) {
        let cpu_str = match e.cpu {
            Some(c) => format!("{:>4.1}", c),
            None => "   -".to_string(),
        };
        let user_str = e
            .user
            .as_deref()
            .unwrap_or("-")
            .chars()
            .take(8)
            .collect::<String>();
        out.push_str(&format!(
            "{:>6} {:<25} {} {:<8}\n",
            e.pid,
            e.name.chars().take(25).collect::<String>(),
            cpu_str,
            user_str
        ));
    }
    out
}
