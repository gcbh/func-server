use std::thread;
use std::sync::mpsc;
use std::sync::Arc;
use std::sync::Mutex;

pub struct ThreadPool{
    workers: Vec<Worker>,
    sender: mpsc::Sender<Message>,
}

impl ThreadPool {
    pub fn new(size: usize) -> Result<ThreadPool, ThreadPoolError> {
        if size <= 0 { return Err(ThreadPoolError::SizeLteZero) }
        // let mut workers = Vec::with_capacity(size);
        let (tx, rx) = mpsc::channel();
        let rx = Arc::new(Mutex::new(rx));

        let workers = (0..size).map(|idx| {
            Worker::new(idx, Arc::clone(&rx))
        }).collect();

        Ok(ThreadPool { workers, sender: tx })
    }

    pub fn execute<F>(&self, f: F) -> Result<(), ThreadPoolError>
        where F: FnOnce() + Send + 'static
    {
        let job = Box::new(f);
        
        self.sender.send(Message::NewJob(job))
            .map_err(|send_error| {
                ThreadPoolError::Failure(format!("Failure: {}", send_error))
            })
    }
}

impl Drop for ThreadPool {
    fn drop(&mut self) {

        println!("Sending terminate message to all workers.");

        for _ in &mut self.workers {
            self.sender.send(Message::Terminate).unwrap();
        }

        println!("Shutting down all workers.");

        for worker in &mut self.workers {
            println!("Shutting down worker {}", worker.id);

            if let Some(thread) = worker.thread.take() {
                thread.join().unwrap();
            }
        }
    }
}

pub enum ThreadPoolError {
    Failure(String),
    SizeLteZero,
}

enum Message {
    NewJob(Job),
    Terminate,
}

pub struct Worker {
    id: usize,
    thread: Option<thread::JoinHandle<()>>,
}

impl Worker {
    fn new(id: usize, receiver: Arc<Mutex<mpsc::Receiver<Message>>>) -> Worker {
        let thread = thread::spawn(move || { 
            loop {
                let message = receiver.lock().unwrap().recv().unwrap();

                match message {
                    Message::NewJob(job) => {
                        println!("Worker {} got a job; executing", id);
                        job.call_box();
                    },
                    Message::Terminate => {
                        println!("Worker {} was told to terminate!", id);
                        break;
                    },
                }
            }
        });
        Worker { id, thread: Some(thread) }
    }
}

type Job = Box<FnBox + Send + 'static>;

trait FnBox {
    fn call_box(self: Box<Self>);
}

impl<F: FnOnce()> FnBox for F {
    fn call_box(self: Box<F>) {
        (*self)()
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
