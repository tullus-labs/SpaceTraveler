use std::fmt::{Display, Formatter};
use std::fs::{File, Permissions};
use std::io::Write;
use std::os::windows::fs::MetadataExt;
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command};
use std::time::UNIX_EPOCH;
use chrono::{DateTime, Local};
use futures::StreamExt;
use indicatif::{ProgressBar, ProgressStyle};
use meilisearch_sdk::client::Client;
use passwords::PasswordGenerator;
use serde::{Deserialize, Serialize};
use tokio::io;
use tokio::io::Error;
use tokio_stream::wrappers::ReadDirStream;
use walkdir::WalkDir;
use tracing::info;

//Structure for send data about files to local meilisearch server
#[derive(Serialize, Deserialize)]
pub struct DataFile {
    id: i32,
    file_path: String,
    file_name: String,
    metadata: Option<Metadata>
}

#[derive(Serialize, Deserialize)]
pub struct Metadata {
    file_type: String,
    is_dir: bool,
    is_file: bool,
    is_symlink: bool,
    size: u64,
    permissions: String,
    modified: String,
    accessed: String,
    created: String
}

//Structure for work with meilisearch server
pub struct MeilisearchRunner {
    host: MeilisearchHost,
    master_key: MeilisearchMasterKey,
    client: Option<Client>,
    process: Option<Child>,
    data_dir: PathBuf,
    exe_path: Option<PathBuf>,
}

impl MeilisearchRunner {
    pub async fn new(host: MeilisearchHost, master_key: MeilisearchMasterKey) -> Self {
        let data_dir = std::env::current_exe()
            .unwrap()
            .parent()
            .unwrap()
            .join("search_engine");
        if !data_dir.exists() {
            std::fs::create_dir(&data_dir).unwrap();
        }
        let mut runner = MeilisearchRunner {
            host,
            master_key,
            client: None,
            process: None,
            data_dir,
            exe_path: None,
        };
        let exe_path = runner.data_dir.clone().join("search_engine.exe");

        {
            if !exe_path.exists() {
                let mut file = File::create(&exe_path).unwrap();
                file.write_all(include_bytes!("../../assets/meilisearch-windows-amd64.exe"))
                    .unwrap();
            }
        }
        runner.exe_path = Some(exe_path);

        runner
    }

    async fn run(&self) -> io::Result<Child> {

            const CREATE_NO_WINDOW: u32 = 0x08000000;
            return Command::new(self.exe_path.clone().unwrap())
                .arg(format!("--master-key={}", self.master_key.0))
                .arg(format!(
                    "--db-path={}",
                    self.data_dir.clone().join("data.ms").display()
                ))
                .arg(format!(
                    "--dump-dir={}",
                    self.data_dir.clone().join("dump/").display()
                ))
                .creation_flags(CREATE_NO_WINDOW)
                .spawn()
    }

    pub async fn safe_run(&mut self) -> Result<(), MeilisearchRunnerError> {
        return match self.run().await {
            Ok(ch) => {
                self.process = Some(ch);
                Ok(())
            }
            Err(e) => Err(MeilisearchRunnerError::Error(e)),
        };
    }

    pub async fn stop(&mut self) {
        if let Some(ref mut ch) = self.process {
            ch.kill().unwrap();
        }
    }

    pub async fn run_client(&mut self) {
        self.client = Some(
            Client::new(
                format!("http://{}:{}", &self.host.0, &self.host.1),
                Some(&self.master_key.0),
            )
                .unwrap(),
        );
        info!("Client run");
    }

    //Update information about file system in meilisearch
    pub async fn update_fs_info(&self) {
        info!("updating info");
        if let Some(client) = self.client.clone() {
            if let Err(_) = client.get_index("files").await {
                client.create_index("files", None).await.unwrap();
            }
            let files = client.index("files");
            let data = files.search().execute::<DataFile>().await.expect("Failed to execute search");
            let mut id = 0;
            if !data.hits.is_empty() {
                id = data.hits.into_iter().last().unwrap().result.id + 1
            }
            let data_arr = self.walkdir(id, "C:\\").await;
            files.add_documents(&data_arr, Some("id")).await.unwrap();
        }
    }

    async fn walkdir(&self, id: i32, path: &str) -> Vec<DataFile> {

        let mut id = id;
        let walkdir = WalkDir::new(path);
        let mut data_arr = vec![];

        let total_entries = WalkDir::new(path).into_iter().count();
        let pb = ProgressBar::new(total_entries as u64);

        info!("walking");

        for entry in walkdir.into_iter().filter_map(|e| e.ok()) {
            let path = entry.clone().into_path();

            let name = if let Some(path) = path.file_name() {
                path.to_str().unwrap().to_string()
            }
            else {
                "".to_string()
            };
            let metadata = if let Ok(metadata) = entry.metadata() {
                let file_type = if metadata.is_dir() {
                    "directory"
                } else if metadata.is_file() {
                    "file"
                } else if metadata.is_symlink() {
                    "symlink"
                } else {
                    "unknown"
                }.to_string();

                let modified: DateTime<Local> = DateTime::from(UNIX_EPOCH + metadata.modified().unwrap().duration_since(UNIX_EPOCH)
                    .unwrap_or_default());

                let accessed: DateTime<Local> = DateTime::from(UNIX_EPOCH + metadata.accessed().unwrap().duration_since(UNIX_EPOCH)
                    .unwrap_or_default());

                let created: DateTime<Local> = DateTime::from(UNIX_EPOCH + metadata.created().unwrap().duration_since(UNIX_EPOCH)
                    .unwrap_or_default());

                Some(Metadata {
                    file_type,
                    is_dir: metadata.is_dir(),
                    is_file: metadata.is_file(),
                    is_symlink: metadata.is_symlink(),
                    size: metadata.file_size(),
                    permissions: format!("{}", metadata.permissions().readonly()),

                    modified: modified.to_rfc3339(),
                    accessed: accessed.to_rfc3339(),
                    created: created.to_rfc3339(),
                })
            }else {
                None
            };

            let data_file = DataFile {
                id,
                file_name: name,
                file_path: path.display().to_string(),
                metadata
            };
            id += 1;
            data_arr.push(data_file);
            pb.inc(1);
        }
        pb.finish();
        data_arr
    }

}

pub enum MeilisearchRunnerError {
    Error(Error),
}

impl Display for MeilisearchRunnerError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            MeilisearchRunnerError::Error(e) => write!(f, " MeilisearchRunnerError: {}", e),
        }
    }
}

#[derive(Clone)]
pub struct MeilisearchHost(String, u16);

impl Default for MeilisearchHost {
    fn default() -> Self {
        MeilisearchHost("localhost".to_string(), 7700)
    }
}

impl MeilisearchHost {
    pub fn new(ip: &str, port: u16) -> Self {
        MeilisearchHost(ip.to_string(), port)
    }
}

#[derive(Clone)]
pub struct MeilisearchMasterKey(String);

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
            strict: true,
        };

        MeilisearchMasterKey(pg.generate_one().unwrap())
    }
}

impl Display for MeilisearchMasterKey {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
