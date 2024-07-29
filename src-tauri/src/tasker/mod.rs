
use std::env::current_exe;
use std::fmt::{Display, Formatter};
use std::fs::{create_dir, File};
use std::io::Write;
use std::os::windows::process::CommandExt;
use std::path::PathBuf;
use std::process::{Child, Command};
use tokio::io;
use tracing::info;

//Struct for run task observer
pub struct Tasker {
    exe_path: PathBuf,
    process: Option<Child>
}

impl Tasker {
    pub fn init() -> Self {
        let exe_path = current_exe().unwrap().parent().unwrap().join("services").join("tasker").join("task_observer.exe");

        {
            if !exe_path.exists() {
                if !exe_path.parent().unwrap().exists() {
                    create_dir(exe_path.parent().unwrap().parent().unwrap()).expect("Failed to create dir");
                    create_dir(exe_path.parent().unwrap()).expect("Failed to create dir");
                }
                let mut file = File::create(&exe_path).unwrap();
                file.write_all(include_bytes!("../../assets/task_observer.exe"))
                    .unwrap();
            }
        }

        Self {
            exe_path,
            process: None,
        }
    }

    async fn run(&self) -> io::Result<Child> {
        //const CREATE_NO_WINDOW: u32 = 0x08000000;

        Command::new(self.exe_path.clone())
            .spawn()
    }

    pub async fn safe_run(&mut self) -> Result<(), TaskerError> {
        return match self.run().await {
            Ok(ch) => {
                self.process = Some(ch);
                Ok(())
            }
            Err(e) => Err(TaskerError::Error(e.to_string()))
        }
    }
}


pub enum TaskerError {
    Error(String)
}

impl Display for TaskerError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            TaskerError::Error(e) => write!(f, "{}", e)
        }
    }
}