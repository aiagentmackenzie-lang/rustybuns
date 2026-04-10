use std::process::Command;

use super::{Platform, ProcessEntry, TaskError};

pub struct UnixPlatform;

impl Platform for UnixPlatform {
    fn whoami(&self) -> Result<String, TaskError> {
        Command::new("whoami")
            .output()
            .map(|o| String::from_utf8_lossy(&o.stdout).into_owned())
            .map_err(|e| TaskError::CommandFailed(e.to_string()))
    }

    fn hostname(&self) -> Result<String, TaskError> {
        Command::new("hostname")
            .output()
            .map(|o| String::from_utf8_lossy(&o.stdout).into_owned())
            .map_err(|e| TaskError::CommandFailed(e.to_string()))
    }

    fn pwd(&self) -> Result<String, TaskError> {
        Command::new("pwd")
            .output()
            .map(|o| String::from_utf8_lossy(&o.stdout).into_owned())
            .map_err(|e| TaskError::CommandFailed(e.to_string()))
    }

    fn ps(&self) -> Result<Vec<ProcessEntry>, TaskError> {
        let output = Command::new("ps")
            .args(["aux", "--no-headers"])
            .output()
            .map_err(|e| TaskError::CommandFailed(e.to_string()))?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut processes = Vec::new();
        for line in stdout.lines() {
            let parts: Vec<&str> = line.splitn(11, ' ').filter(|s| !s.is_empty()).collect();
            if parts.len() >= 3 {
                let pid: u32 = parts[1].parse().unwrap_or(0);
                let cpu: Option<f64> = parts[2].parse().ok();
                let user = Some(parts[0].to_string());
                let name = parts[10..].join(" ");
                processes.push(ProcessEntry {
                    pid,
                    name,
                    cpu,
                    user,
                });
            }
        }
        Ok(processes)
    }

    fn ls(&self, path: &str) -> Result<String, TaskError> {
        Command::new("ls")
            .args(["-la", path])
            .output()
            .map(|o| String::from_utf8_lossy(&o.stdout).into_owned())
            .map_err(|e| TaskError::CommandFailed(e.to_string()))
    }

    fn echo(&self, args: &[String]) -> Result<String, TaskError> {
        Ok(args.join(" "))
    }

    fn sleep(&self, secs: u64) -> Result<String, TaskError> {
        std::thread::sleep(std::time::Duration::from_secs(secs));
        Ok(format!("slept {}s", secs))
    }

    fn shell(&self, cmd: &str) -> Result<String, TaskError> {
        Command::new("sh")
            .args(["-c", cmd])
            .output()
            .map(|o| {
                let stdout = String::from_utf8_lossy(&o.stdout).into_owned();
                let stderr = String::from_utf8_lossy(&o.stderr).into_owned();
                if stderr.is_empty() {
                    stdout
                } else {
                    format!("{}\n[stderr]: {}", stdout, stderr)
                }
            })
            .map_err(|e| TaskError::CommandFailed(e.to_string()))
    }

    fn id(&self) -> Result<String, TaskError> {
        Command::new("id")
            .output()
            .map(|o| String::from_utf8_lossy(&o.stdout).into_owned())
            .map_err(|e| TaskError::CommandFailed(e.to_string()))
    }

    fn uname(&self) -> Result<String, TaskError> {
        Command::new("uname")
            .args(["-a"])
            .output()
            .map(|o| String::from_utf8_lossy(&o.stdout).into_owned())
            .map_err(|e| TaskError::CommandFailed(e.to_string()))
    }

    fn whoami_all(&self) -> Result<String, TaskError> {
        Command::new("id")
            .output()
            .map(|o| String::from_utf8_lossy(&o.stdout).into_owned())
            .map_err(|e| TaskError::CommandFailed(e.to_string()))
    }

    fn cred_access_check(&self) -> Result<String, TaskError> {
        use std::fs;
        use std::path::Path;

        let targets = [
            ("/etc/passwd", "T1003.001: /etc/passwd"),
            ("/etc/shadow", "T1003.001: /etc/shadow"),
            ("/etc/sudoers", "T1003.001: /etc/sudoers"),
            ("~/.ssh", "T1082: SSH keys dir"),
        ];

        let mut out = String::new();
        out.push_str("=== Credential Access Paths (Metadata-Only) ===\n\n");

        for (path, desc) in &targets {
            let expanded = shellexp(path);
            let p = Path::new(&expanded);
            if p.exists() {
                let readable = fs::metadata(p)
                    .map(|m| m.permissions().readonly())
                    .unwrap_or(true);
                out.push_str(&format!(
                    "[ACCESSIBLE] {} — {} (readable: {})\n",
                    desc, expanded, !readable
                ));
            } else {
                out.push_str(&format!("[NOT FOUND] {} — {}\n", desc, expanded));
            }
        }
        out.push_str("\n[NOTE] No actual credentials extracted (metadata-only mode)");
        Ok(out)
    }

    fn list_env(&self) -> Result<String, TaskError> {
        use std::env;
        let vars: Vec<(String, String)> = env::vars().collect();
        let mut out = String::from("=== Environment Variables (Keys Only) ===\n\n");
        for (key, _) in vars {
            let sensitive = key.eq_ignore_ascii_case("PASSWORD")
                || key.eq_ignore_ascii_case("SECRET")
                || key.eq_ignore_ascii_case("API_KEY")
                || key.eq_ignore_ascii_case("TOKEN")
                || key.eq_ignore_ascii_case("PRIVATE_KEY")
                || key.to_uppercase().contains("KEY")
                || key.to_uppercase().contains("SECRET")
                || key.to_uppercase().contains("TOKEN")
                || key.to_uppercase().contains("PASSWORD");
            if sensitive {
                out.push_str(&format!("[SENSITIVE] {}=***REDACTED***\n", key));
            } else {
                out.push_str(&format!("{}={}\n", key, "[VALUE]"));
            }
        }
        Ok(out)
    }

    fn list_ssh(&self) -> Result<String, TaskError> {
        use std::fs;
        let home = std::env::var("HOME").unwrap_or_default();
        let ssh_dir = format!("{}/.ssh", home);
        let mut out = String::from("=== SSH Directory (Metadata-Only) ===\n\n");
        out.push_str(&format!("SSH dir: {}\n\n", ssh_dir));

        let entries =
            fs::read_dir(&ssh_dir).map_err(|e| TaskError::CommandFailed(e.to_string()))?;
        for entry in entries {
            if let Ok(entry) = entry {
                let path = entry.path();
                let meta = fs::metadata(&path).ok();
                let file_type = if path.is_dir() {
                    "dir"
                } else if path.is_file() {
                    "file"
                } else {
                    "other"
                };
                let size = meta.as_ref().map(|m| m.len()).unwrap_or(0);
                let name = path
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_default();
                out.push_str(&format!("  {}  {:>8} bytes  {}\n", file_type, size, name));
            }
        }
        out.push_str("\n[NOTE] No private key contents accessed (metadata-only mode)");
        Ok(out)
    }

    fn collection(&self, path: &str) -> Result<String, TaskError> {
        use std::fs;
        const MAX_SIZE: u64 = 1_000_000;
        const MAX_FILES: usize = 100;

        let expanded = shellexp(path);
        let path_obj = std::path::Path::new(&expanded);

        if !path_obj.exists() {
            return Err(TaskError::NotFound);
        }

        let mut out = String::new();
        out.push_str(&format!("=== Collection Report: {} ===\n\n", expanded));

        if path_obj.is_file() {
            let meta =
                fs::metadata(path_obj).map_err(|e| TaskError::CommandFailed(e.to_string()))?;
            if meta.len() > MAX_SIZE {
                out.push_str(&format!(
                    "[SKIPPED] File exceeds size limit ({} > {} bytes)\n",
                    meta.len(),
                    MAX_SIZE
                ));
                return Ok(out);
            }
            out.push_str(&format!("[FILE] {} ({} bytes)\n", expanded, meta.len()));
            return Ok(out);
        }

        out.push_str("[DIR] Scanning (max 100 files, 1MB total)...\n\n");

        let mut total_size: u64 = 0;
        let mut file_count = 0;

        fn crawl(
            dir: &std::path::Path,
            out: &mut String,
            total_size: &mut u64,
            file_count: &mut usize,
        ) -> bool {
            if *file_count >= MAX_FILES || *total_size >= MAX_SIZE {
                return false;
            }
            if let Ok(entries) = fs::read_dir(dir) {
                for entry in entries.flatten() {
                    if *file_count >= MAX_FILES || *total_size >= MAX_SIZE {
                        return false;
                    }
                    let path = entry.path();
                    if let Ok(meta) = fs::metadata(&path) {
                        if meta.is_file() {
                            let size = meta.len();
                            if *total_size + size > MAX_SIZE {
                                continue;
                            }
                            *total_size += size;
                            *file_count += 1;
                            out.push_str(&format!(
                                "  [FILE] {} ({} bytes)\n",
                                path.display(),
                                size
                            ));
                        } else if meta.is_dir() {
                            out.push_str(&format!("  [DIR]  {}/\n", path.display()));
                            if !crawl(&path, out, total_size, file_count) {
                                return false;
                            }
                        }
                    }
                }
            }
            true
        }

        let incomplete = !crawl(path_obj, &mut out, &mut total_size, &mut file_count);
        out.push_str(&format!(
            "\n[STATS] {} files, {} bytes total\n",
            file_count, total_size
        ));
        if incomplete {
            out.push_str("[TRUNCATED] Collection limit reached\n");
        }
        Ok(out)
    }
}

fn shellexp(path: &str) -> String {
    if path.starts_with("~/") {
        if let Ok(home) = std::env::var("HOME") {
            return format!("{}{}", home, &path[1..]);
        }
    }
    path.to_string()
}
