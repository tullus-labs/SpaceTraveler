use std::fmt::{Display, Formatter};
use std::fs::File;
use std::io::{Error, Write};
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command};
use sysinfo::System;
use tokio::io;

pub struct BlazzyRunner {
    process: Option<Child>,
    exe_path: PathBuf
}

impl BlazzyRunner {
    pub async fn init() -> Self {
        let exe_path = std::env::current_exe().unwrap().parent().unwrap().join("blazzy").join("blazzy.exe");
        {
            if !exe_path.exists() {
                if !exe_path.parent().unwrap().exists() {
                    std::fs::create_dir(exe_path.parent().unwrap()).unwrap();
                }
                let mut file = File::create(&exe_path).unwrap();
                file.write_all(include_bytes!("../../assets/blazzy.exe")).unwrap();
            }
        }

        Self {
            process: None,
            exe_path
        }
    }

    async fn add_autostart(&self) {
        let username = whoami::username();
        let startup_folder = format!(
            "C:\\Users\\{}\\AppData\\Roaming\\Microsoft\\Windows\\Start Menu\\Programs\\Startup",
            username
        );
        let shortcut_path = Path::new(&startup_folder).join("blazzy.lnk");
        if !shortcut_path.exists() {
            let target_path = format!("{} -p \"C:\\\" ", self.exe_path.display());

            // Create shortcut command
            let cmd = format!(
                r"powershell.exe $ws = New-Object -ComObject WScript.Shell; $s = $ws.CreateShortcut('{}'); $s.TargetPath = '{}'; $s.Save()",
                shortcut_path.display(),
                target_path
            );

            // Execute the command
            let _output = Command::new("cmd")
                .args(&["/C", &cmd])
                .output()
                .expect("Failed to create shortcut");
        }
    }

    async fn run(&self) -> io::Result<Child> {
        const CREATE_NO_WINDOW: u32 = 0x08000000;

        Command::new(self.exe_path.clone())
            .arg("-p")
            .arg("C:\\")
            .arg("--host")
            .arg("127.0.0.1:8000")
            .creation_flags(CREATE_NO_WINDOW)
            .spawn()
    }

    pub async fn safe_run(&mut self) -> Result<(), BlazzyRunnerError> {
        self.add_autostart().await;
        if self.is_running() { return Ok(()) }
        return match self.run().await {
            Ok(ch) => {
                self.process = Some(ch);
                Ok(())
            }
            Err(e) => Err(BlazzyRunnerError::Error(e))
        }
    }

    pub async fn stop(&mut self) {
        if let Some(ref mut ch) = self.process {
            ch.kill().unwrap();
        }
    }

    fn is_running(&self) -> bool {
        let mut system = System::new_all();
        system.refresh_all();

        for (_pid, process) in system.processes() {
            if process.name().to_lowercase().contains("blazzy") {
                return true;
            }
        }

        false
    }

}

pub enum BlazzyRunnerError {
    Error(Error)
}

impl Display for BlazzyRunnerError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            BlazzyRunnerError::Error(e) => write!(f, "BlazzyRunnerError: {}", e)
        }
    }
}