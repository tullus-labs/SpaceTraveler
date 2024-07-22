use std::any::Any;
use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::Arc;
use atomic_refcell::AtomicRefCell;
use futures::stream::FuturesUnordered;
use futures::StreamExt;
use tokio::task::JoinHandle;

//Struct for manage Arcs and tokio tasks
pub struct Tasker {
    tasks: FuturesUnordered<JoinHandle<TaskerResult<()>>>,
    arcs: HashMap<String, Arc<AtomicRefCell<dyn Any + Send + Sync>>>
}

impl Tasker {
    pub fn init() -> Self {
        Self {
            tasks: FuturesUnordered::new(),
            arcs: HashMap::new()
        }
    }

    pub async fn add_task(&mut self, task: JoinHandle<TaskerResult<()>>) {
        self.tasks.push(task);
    }

    pub async fn add_arc(&mut self, name: &str, arc: Arc<AtomicRefCell<dyn Any + Send + Sync>>) {
        self.arcs.insert(name.to_string(), arc);
    }

    pub fn get_arc(&self, name: &str) -> Arc<AtomicRefCell<dyn Any + Send + Sync>> {
        self.arcs.get(name).unwrap().clone()
    }

    pub async fn run_tasks(&mut self) -> TaskerResult<()> {
        while let Some(result) = self.tasks.next().await {
            return match result {
                Ok(_) => TaskerResult::Ok(()),
                Err(e) => TaskerResult::Err(TaskerError::ERROR(e.to_string()))
            }
        }
        return TaskerResult::Ok(())
    }
}

pub enum TaskerResult <T> {
    Ok(T),
    Err(TaskerError)
}

pub enum TaskerError {
    ERROR(String)
}