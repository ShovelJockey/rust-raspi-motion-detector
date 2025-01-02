use std::{
    sync::{mpsc, Arc, Mutex},
    thread,
};

type Job = Box<dyn FnOnce() + Send + 'static>;

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
        F: FnOnce() + Send + 'static,
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
}

impl ThreadWorker {
    fn new(id: usize, receiver: Arc<Mutex<mpsc::Receiver<Job>>>) -> ThreadWorker {
        let thread = Some(thread::spawn(move || loop {
            let message = receiver.lock().unwrap().recv();

            match message {
                Ok(job) => {
                    println!("Worker {id} recieved a new job executing");
                    job();
                }
                Err(_) => {
                    println!("Worker {id} disconnected; shutting down.");
                    break;
                }
            }
        }));

        ThreadWorker { id, thread }
    }
}
