use super::file_stream::FileStream;
use std::{
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    thread,
    time::Duration,
};
use tokio::fs::File;
use tokio_util::io::ReaderStream;

pub struct ThreadPool {
    threads: Vec<ThreadWorker>,
    file_queue: Arc<Mutex<Vec<PathBuf>>>,
    result_queue: Arc<Mutex<Vec<FileStream<ReaderStream<File>>>>>,
}

impl ThreadPool {
    pub async fn new(thread_num: usize) -> ThreadPool {
        assert!(thread_num > 0);

        let mut threads = Vec::with_capacity(thread_num);
        let file_queue = Arc::new(Mutex::new(Vec::new()));
        let result_queue = Arc::new(Mutex::new(Vec::new()));
        for id in 0..thread_num {
            threads.push(
                ThreadWorker::new(id, Arc::clone(&file_queue), Arc::clone(&result_queue)).await,
            )
        }

        ThreadPool {
            threads,
            file_queue,
            result_queue,
        }
    }

    pub async fn queue_file(&self, file: PathBuf) {
        self.file_queue.lock().unwrap().push(file);
    }

    pub async fn tasks_running(&self) -> bool {
        self.threads
            .iter()
            .any(|thread| thread.running_task.load(Ordering::Relaxed))
    }

    pub async fn get_result(&self) -> (bool, Option<FileStream<ReaderStream<tokio::fs::File>>>) {
        let file_queue_running = !self.file_queue.lock().unwrap().is_empty();
        let result_pop = self.result_queue.lock().unwrap().pop();
        match result_pop {
            Some(file_stream) => {
                return (true, Some(file_stream));
            }
            None => {
                if file_queue_running | self.tasks_running().await {
                    return (true, None);
                }
                return (false, None);
            }
        }
    }
}

impl Drop for ThreadPool {
    fn drop(&mut self) {
        for worker in &mut self.threads {
            println!("Shutting down worker: {}", worker.id);

            if let Some(thread) = worker.thread.take() {
                thread.abort();
            }
        }
    }
}

struct ThreadWorker {
    id: usize,
    thread: Option<tokio::task::JoinHandle<()>>,
    running_task: Arc<AtomicBool>,
}

impl ThreadWorker {
    async fn new(
        id: usize,
        file_queue: Arc<Mutex<Vec<PathBuf>>>,
        result_queue: Arc<Mutex<Vec<FileStream<ReaderStream<File>>>>>,
    ) -> ThreadWorker {
        // have thread none, init thread properly with self ref? or just arc clone here?
        let running_task = Arc::new(AtomicBool::new(false));
        let thread_running_task = running_task.clone();
        let thread = Some(tokio::spawn(async move {
            loop {
                // look for new items in file queue to process
                let queue_pop = match file_queue.lock() {
                    Ok(mut queue) => queue.pop(),
                    Err(error) => {
                        println!("Error accessing Mutex file queue for threadworker: {id}, error: {error}");
                        break;
                    }
                };

                let file_stream = match queue_pop {
                    Some(file_obj) => {
                        thread_running_task.store(true, Ordering::Relaxed);
                        FileStream::<ReaderStream<File>>::from_path(file_obj).await
                    }
                    None => {
                        println!("File queue empty for threadworker:");
                        thread::sleep(Duration::from_secs_f32(0.5));
                        continue;
                    }
                };

                match file_stream {
                    Ok(stream) => {
                        println!("Worker {id} finished preparing stream.");
                        result_queue.lock().unwrap().push(stream);
                        thread_running_task.store(false, Ordering::Relaxed);
                    }
                    Err(_) => {
                        println!("Worker {id} encountered error building stream.");
                        break;
                    }
                }
            }
        }));

        ThreadWorker {
            id,
            thread,
            running_task,
        }
    }
}
