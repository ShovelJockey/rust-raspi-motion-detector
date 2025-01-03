use std::{
    sync::{mpsc, Arc, Mutex},
    thread,
};
use axum::{response::Response, BoxError};

type Job = Box<dyn Send + 'static + FnOnce() -> Result<Response, BoxError>>;

pub struct ThreadPool {
    threads: Vec<ThreadWorker>,
    sender: Option<mpsc::Sender<Job>>,
}

impl ThreadPool {
    pub fn new(thread_num: usize) -> ThreadPool {
        assert!(thread_num > 0);

        let mut threads = Vec::with_capacity(thread_num);
        let (sender, receiver) = mpsc::channel();
        let receiver = Arc::new(Mutex::new(receiver));
        for id in 0..thread_num {
            threads.push(ThreadWorker::new(id, Arc::clone(&receiver)))
        }
        ThreadPool {
            threads,
            sender: Some(sender),
        }
    }

    pub fn execute<F>(&self, f: F)
    where
        F: Send + 'static + FnOnce() -> Result<Response, BoxError>,
    {
        let job = Box::new(f);

        self.sender.as_ref().unwrap().send(job).unwrap();
    }
}

impl Drop for ThreadPool {
    fn drop(&mut self) {
        drop(self.sender.take());

        for worker in &mut self.threads {
            println!("Shutting down worker: {}", worker.id);

            if let Some(thread) = worker.thread.take() {
                thread.join().unwrap();
            }
        }
    }
}

struct ThreadWorker {
    id: usize,
    thread: Option<thread::JoinHandle<()>>,
    result: Arc<Mutex<TaskResult>>
}

impl ThreadWorker {
    fn new(id: usize, receiver: Arc<Mutex<mpsc::Receiver<Job>>>) -> ThreadWorker {
        let result = Arc::new(Mutex::new(TaskResult::new(id)));
        let thread_result = result.clone();
        let thread = Some(thread::spawn(move || loop {
            let message = receiver.lock().unwrap().recv();

            match message {
                Ok(job) => {
                    println!("Worker {id} recieved a new job executing");
                    let mut task_result = thread_result.lock().unwrap();
                    task_result.capture_result(job());
                }
                Err(_) => {
                    println!("Worker {id} disconnected; shutting down.");
                    break;
                }
            }
        }));

        ThreadWorker { id, thread, result }
    }
}

struct TaskResult {
    id: usize,
    result: Option<Response>
}

impl TaskResult {
    fn new(id: usize) -> TaskResult {
        TaskResult { id, result: None }
    }

    fn capture_result(&mut self, result: Result<Response, BoxError>) {
        match result {
            Ok(response) => {
                self.result = Some(response)
            },
            Err(error) => {
                println!("Error in task: {}, error: {}", self.id, error)
            }
        }
    }

    fn retrieve_result(&mut self) -> Option<Response> {
        self.result.take()
    }
}