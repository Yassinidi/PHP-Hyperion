use crossbeam_deque::{Injector, Stealer, Worker, Steal};
use crossbeam_utils::sync::{Parker, Unparker};
use std::sync::Arc;
use crate::fibre::Fibre;

pub struct WorkerContext {
    pub local_queue: Worker<Fibre>,
    pub parker: Parker,
}

impl WorkerContext {
    pub fn find_task(&self, global: &Injector<Fibre>, stealers: &[Stealer<Fibre>]) -> Option<Fibre> {
        // 1. Pop from local
        if let Some(task) = self.local_queue.pop() {
            return Some(task);
        }
        
        // 2. Steal from global
        loop {
            match global.steal_batch_and_pop(&self.local_queue) {
                Steal::Success(task) => return Some(task),
                Steal::Empty => break,
                Steal::Retry => continue,
            }
        }
        
        // 3. Steal from other workers
        for stealer in stealers {
            loop {
                match stealer.steal_batch_and_pop(&self.local_queue) {
                    Steal::Success(task) => return Some(task),
                    Steal::Empty => break,
                    Steal::Retry => continue,
                }
            }
        }
        
        None
    }
}

pub struct Scheduler {
    pub global_queue: Arc<Injector<Fibre>>,
    pub workers: Vec<WorkerContext>,
    pub stealers: Vec<Stealer<Fibre>>,
    pub unparkers: Vec<Unparker>,
}

impl Scheduler {
    pub fn new(num_threads: usize) -> Self {
        let global_queue = Arc::new(Injector::new());
        let mut workers = Vec::with_capacity(num_threads);
        let mut stealers = Vec::with_capacity(num_threads);
        let mut unparkers = Vec::with_capacity(num_threads);

        for _ in 0..num_threads {
            let worker = Worker::new_fifo();
            let parker = Parker::new();
            
            unparkers.push(parker.unparker().clone());
            stealers.push(worker.stealer());
            workers.push(WorkerContext { local_queue: worker, parker });
        }

        Self {
            global_queue,
            workers,
            stealers,
            unparkers,
        }
    }

    pub fn spawn(&self, fibre: Fibre) {
        self.global_queue.push(fibre);
    }
}
