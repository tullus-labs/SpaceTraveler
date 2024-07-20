use std::fmt::{Display, Formatter};
use std::fs::File;
use std::io::{Error, Write};
use std::os::windows::process::CommandExt;
use std::path::PathBuf;
use std::process::{Child, Command};
use sysinfo::System;
use tokio::io;
use crate::meilisearch_runner::runner::MeilisearchRunnerError;

pub struct BlazzyRunner {
    process: Option<Child>,
    exe_path: PathBuf
}

impl BlazzyRunner {
    pub async fn init() -> Self {
        let exe_path = std::env::current_exe().unwrap().parent().unwrap().join("blazzy").join("blazzy.exe");
        let autostart_path = std::env::current_exe().unwrap().parent().unwrap().join("blazzy").join("autostart.xml");
        {
            if !exe_path.exists() {
                if !exe_path.parent().unwrap().exists() {
                    std::fs::create_dir(exe_path.parent().unwrap()).unwrap();
                }
                if !autostart_path.exists() {
                    let task_xml = format!(
                        r#"<Task version="1.2" xmlns="http://schemas.microsoft.com/windows/2004/02/mit/task">
                            <Triggers>
                                <LogonTrigger>
                                    <Enabled>true</Enabled>
                                </LogonTrigger>
                            </Triggers>
                            <Actions Context="Author">
                                <Exec>
                                    <Command>{}</Command>
                                </Exec>
                            </Actions>
                        </Task>"#, exe_path.display());
                    let mut file = File::create(&autostart_path).unwrap();
                    file.write_all(task_xml.as_bytes()).unwrap();

                    Command::new("schtasks")
                        .arg("/Create")
                        .arg("/TN")
                        .arg("blazzy")
                        .arg("/XML")
                        .arg(autostart_path)
                        .output()
                        .expect("Failed to create task");
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

    async fn run(&self) -> io::Result<Child> {
        const CREATE_NO_WINDOW: u32 = 0x08000000;

        Command::new(self.exe_path.clone())
            .arg("-p \"C:\\\"")
            .creation_flags(CREATE_NO_WINDOW)
            .spawn()
    }

    pub async fn safe_run(&mut self) -> Result<(), BlazzyRunnerError> {
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