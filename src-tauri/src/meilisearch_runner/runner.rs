use std::fmt::{Display, Formatter};
use std::fs::File;
use std::io::Write;
use std::os::windows::process::CommandExt;
use std::path::PathBuf;
use std::process::{Child, Command};
use std::sync::mpsc::Sender;
use std::time::Duration;
use meilisearch_sdk::client::Client;
use meilisearch_sdk::errors::MeilisearchError;
use passwords::PasswordGenerator;
use tempfile::{tempdir, TempDir};
use tokio::io;
use tokio::io::Error;
pub struct MeilisearchRunner {
    host: MeilisearchHost,
    master_key: MeilisearchMasterKey,
    client: Option<Client>,
    process: Option<Child>,
    data_dir: PathBuf,
    exe_path: Option<PathBuf>
}

impl MeilisearchRunner {
    pub async fn new(host: MeilisearchHost, master_key: MeilisearchMasterKey) -> Self {
        let data_dir = std::env::current_exe().unwrap().parent().unwrap().join("search_engine");
        if !data_dir.exists() {
            std::fs::create_dir(&data_dir).unwrap();
        }
        let mut runner = MeilisearchRunner {
            host,
            master_key,
            client: None,
            process: None,
            data_dir,
            exe_path: None
        };
        let exe_path = runner.data_dir.clone().join("search_engine.exe");

        {
            if !exe_path.exists() {
                let mut file = File::create(&exe_path).unwrap();
                file.write_all(include_bytes!("../../assets/meilisearch-windows-amd64.exe")).unwrap();
            }
        }
        runner.exe_path = Some(exe_path);

        runner
    }

    async fn run(&self) -> io::Result<Child> {

        const CREATE_NO_WINDOW: u32 = 0x08000000;

        Command::new(self.exe_path.clone().unwrap())
            .arg(format!("--master-key={}", self.master_key))
            .arg(format!("--db-path={}", self.data_dir.clone().join("data.ms").display()))
            .arg(format!("--dump-dir={}", self.data_dir.clone().join("dump/").display()))
            .creation_flags(CREATE_NO_WINDOW)
            .spawn()

    }

    pub async fn safe_run(&mut self) -> Result<(), MeilisearchRunnerError> {

        return match self.run().await {
            Ok(ch) => {
                self.process = Some(ch);
                Ok(())
            }
            Err(e) => Err(MeilisearchRunnerError::Error(e))
        }
    }

    pub async fn stop(&mut self) {
        if let Some(ref mut ch) = self.process {
            ch.kill().unwrap();
        }
    }

}

pub enum MeilisearchRunnerError {
    Error(Error)
}

impl Display for MeilisearchRunnerError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            MeilisearchRunnerError::Error(e) => write!(f, " MeilisearchRunnerError: {}", e)
        }
    }
}

#[derive(Clone)]
pub struct MeilisearchHost (String, u16);

impl Default for MeilisearchHost {
    fn default() -> Self {
        MeilisearchHost("127.0.0.1".to_string(), 7700)
    }
}

impl MeilisearchHost {
    pub fn new(ip: & str, port: u16) -> Self {
        MeilisearchHost(ip.to_string(), port)
    }
}

#[derive(Clone)]
pub struct MeilisearchMasterKey (String);

impl MeilisearchMasterKey {
    pub async fn gen() -> MeilisearchMasterKey {
        let pg = PasswordGenerator {
            length: 16,
            numbers: true,
            lowercase_letters: true,
            uppercase_letters: true,
            symbols: true,
            spaces: false,
            exclude_similar_characters: false,
            strict: true
        };

        MeilisearchMasterKey(pg.generate_one().unwrap())

    }
}

impl Display for MeilisearchMasterKey {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}