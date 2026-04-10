use std::process::Command;

use super::{Platform, ProcessEntry, TaskError};

pub struct WindowsPlatform;

impl Platform for WindowsPlatform {
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
        Command::new("cd")
            .output()
            .map(|o| String::from_utf8_lossy(&o.stdout).into_owned())
            .map_err(|e| TaskError::CommandFailed(e.to_string()))
    }

    fn ps(&self) -> Result<Vec<ProcessEntry>, TaskError> {
        use windows::Win32::Foundation::*;
        use windows::Win32::System::ProcessStatus::*;
        use windows::Win32::System::Threading::*;

        let mut processes = Vec::new();
        unsafe {
            let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0).map_err(|e| {
                TaskError::CommandFailed(format!("CreateToolhelp32Snapshot: {}", e))
            })?;

            let mut entry = PROCESSENTRY32W {
                dwSize: std::mem::size_of::<PROCESSENTRY32W>() as u32,
                ..Default::default()
            };

            if Process32FirstW(snapshot, &mut entry).is_ok() {
                loop {
                    let name = String::from_utf16_lossy(&entry.szExeFile)
                        .trim_end_matches('\0')
                        .to_string();
                    processes.push(ProcessEntry {
                        pid: entry.th32ProcessID,
                        name,
                        cpu: None,
                        user: None,
                    });
                    if Process32NextW(snapshot, &mut entry).is_err() {
                        break;
                    }
                }
            }
            let _ = CloseHandle(snapshot);
        }
        Ok(processes)
    }

    fn ls(&self, path: &str) -> Result<String, TaskError> {
        Command::new("cmd")
            .args(["/C", "dir", path])
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
        Command::new("cmd")
            .args(["/C", cmd])
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
        Command::new("whoami")
            .output()
            .map(|o| String::from_utf8_lossy(&o.stdout).into_owned())
            .map_err(|e| TaskError::CommandFailed(e.to_string()))
    }

    fn uname(&self) -> Result<String, TaskError> {
        Command::new("ver")
            .output()
            .map(|o| String::from_utf8_lossy(&o.stdout).into_owned())
            .map_err(|e| TaskError::CommandFailed(e.to_string()))
    }

    fn whoami_all(&self) -> Result<String, TaskError> {
        Command::new("whoami")
            .args(["/all"])
            .output()
            .map(|o| String::from_utf8_lossy(&o.stdout).into_owned())
            .map_err(|e| TaskError::CommandFailed(e.to_string()))
    }

    fn cred_access_check(&self) -> Result<String, TaskError> {
        use std::process::Command;
        let mut out = String::new();
        out.push_str("=== Credential Access Paths (Metadata-Only) ===\n\n");

        let targets = [
            ("HKLM\\SAM", "T1003.001: SAM registry"),
            ("HKLM\\SECURITY", "T1003.001: SECURITY registry"),
            ("HKLM\\SYSTEM", "T1003.001: SYSTEM registry"),
            (
                "%PROGRAMDATA%\\Microsoft\\Windows\\Credentials",
                "T1003.001: Credentials dir",
            ),
        ];

        for (path, desc) in &targets {
            let expanded = env_expand(path);
            let result = Command::new("cmd")
                .args([
                    "/C",
                    &format!(
                        "if exist \"{}\" (echo EXISTS) else (echo NOTFOUND)",
                        expanded
                    ),
                ])
                .output();

            if result.is_err() {
                out.push_str(&format!("[ERROR] {} — {}\n", desc, expanded));
            } else {
                let exists = String::from_utf8_lossy(&result.unwrap().stdout).contains("EXISTS");
                out.push_str(&format!(
                    "[{}] {} — {}\n",
                    if exists { "ACCESSIBLE" } else { "NOT FOUND" },
                    desc,
                    expanded
                ));
            }
        }

        let lsass_check = Command::new("cmd")
            .args(["/C", "tasklist /FI \"IMAGENAME eq lsass.exe\" /FO CSV /NH"])
            .output();
        if let Ok(output) = lsass_check {
            let stdout = String::from_utf8_lossy(&output.stdout);
            if stdout.contains("lsass") {
                out.push_str("\n[LSASS] Process found (T1003.001 - credential dumping target)\n");
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
        let userprofile =
            std::env::var("USERPROFILE").unwrap_or_else(|_| "C:\\Users\\Default".to_string());
        let ssh_dir = format!("{}\\.ssh", userprofile);
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

        let expanded = env_expand(path);
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
                            out.push_str(&format!("  [DIR]  {}\\\n", path.display()));
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

fn env_expand(path: &str) -> String {
    let mut result = path.to_string();
    for (key, val) in std::env::vars() {
        result = result.replace(&format!("%{}%", key), &val);
    }
    result
}
