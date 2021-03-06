use futures::{Async, Future, Poll};
use rayon;

use std::mem;
use std::sync::{Arc, Mutex};

/// An asynchronous task
pub struct Task<T, E> {
    state: Arc<Mutex<TaskState<T, E>>>,
}

enum TaskState<T, E> {
    Ready(T),
    NotReady,
    Consumed,
    Errored(E),
}

impl<T, E> Task<T, E> {
    /// Make a new task that is already finished and successful.
    pub fn resolved(t: T) -> Self {
        Self {
            state: Arc::new(Mutex::new(TaskState::Ready(t))),
        }
    }

    /// Make a new task that is already finished and failed.
    pub fn errored(e: E) -> Self {
        Self {
            state: Arc::new(Mutex::new(TaskState::Errored(e))),
        }
    }
}

impl<T, E> Task<T, E>
where
    T: Sync + Send + 'static,
    E: Send + 'static,
{
    /// Make a new asynchronous task from closure.
    pub fn new<F>(f: F) -> Self
    where
        F: FnOnce() -> Result<T, E> + Send + 'static,
    {
        let state = Arc::new(Mutex::new(TaskState::NotReady));
        let passed_state = state.clone();

        rayon::spawn(move || {
            let r = f();
            let mut lock = passed_state.lock().unwrap();
            *lock = match r {
                Ok(t) => TaskState::Ready(t),
                Err(e) => TaskState::Errored(e),
            };
        });
        Self { state }
    }
}

impl<T, E> Future for Task<T, E> {
    type Item = T;
    type Error = E;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        match self.state.lock() {
            Ok(mut lock) => {
                if let TaskState::Ready(_) = *lock {
                    let ready_state = mem::replace(&mut *lock, TaskState::Consumed);
                    if let TaskState::Ready(t) = ready_state {
                        Ok(Async::Ready(t))
                    } else {
                        unreachable!()
                    }
                } else if let TaskState::Errored(_) = *lock {
                    let errored_state = mem::replace(&mut *lock, TaskState::Consumed);
                    if let TaskState::Errored(e) = errored_state {
                        Err(e)
                    } else {
                        unreachable!()
                    }
                } else {
                    Ok(Async::NotReady)
                }
            }
            Err(_) => {
                // TODO: Handle cleaning poison error
                panic!("Poison error")
            }
        }
    }
}
