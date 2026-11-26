use std::{
    cmp::Ordering,
    collections::{BinaryHeap, VecDeque},
    sync::Arc,
    thread,
    time::{Duration, Instant},
};

use android_activity::AndroidAppWaker;
use gpui::{PlatformDispatcher, Priority, RunnableVariant, profiler};
use parking_lot::{Condvar, Mutex};

const MIN_THREADS: usize = 2;

/// A FIFO per priority level; higher levels are always drained first.
#[derive(Default)]
pub(crate) struct TaskQueue {
    lanes: Mutex<[VecDeque<RunnableVariant>; 3]>,
    available: Condvar,
}

impl TaskQueue {
    fn lane(priority: Priority) -> usize {
        match priority {
            Priority::RealtimeAudio | Priority::High => 0,
            Priority::Low => 2,
            _ => 1,
        }
    }

    fn push(&self, priority: Priority, runnable: RunnableVariant) {
        self.lanes.lock()[Self::lane(priority)].push_back(runnable);
        self.available.notify_one();
    }

    fn try_pop(&self) -> Option<RunnableVariant> {
        self.lanes.lock().iter_mut().find_map(VecDeque::pop_front)
    }

    fn pop_blocking(&self) -> RunnableVariant {
        let mut lanes = self.lanes.lock();
        loop {
            if let Some(runnable) = lanes.iter_mut().find_map(VecDeque::pop_front) {
                return runnable;
            }
            self.available.wait(&mut lanes);
        }
    }
}

pub(crate) struct AndroidDispatcher {
    main_queue: Arc<TaskQueue>,
    waker: AndroidAppWaker,
    background_queue: Arc<TaskQueue>,
    timers: Arc<TimerQueue>,
    main_thread_id: thread::ThreadId,
}

impl AndroidDispatcher {
    /// Must be called on the thread that runs the `android_main` event loop, which
    /// becomes the GPUI main thread. Returns the queue that loop must drain.
    pub(crate) fn new(waker: AndroidAppWaker) -> (Self, Arc<TaskQueue>) {
        let main_queue = Arc::new(TaskQueue::default());
        let background_queue = Arc::new(TaskQueue::default());
        let thread_count = thread::available_parallelism().map_or(MIN_THREADS, |count| count.get().max(MIN_THREADS));
        for index in 0..thread_count {
            let queue = background_queue.clone();
            let spawned = thread::Builder::new().name(format!("Worker-{index}")).spawn(move || {
                loop {
                    run(queue.pop_blocking());
                }
            });
            if let Err(error) = spawned {
                log::error!("failed to spawn GPUI worker thread {index}: {error}");
            }
        }

        let timers = Arc::new(TimerQueue::default());
        let spawned = thread::Builder::new().name("Timer".to_owned()).spawn({
            let timers = timers.clone();
            move || timers.run()
        });
        if let Err(error) = spawned {
            log::error!("failed to spawn GPUI timer thread: {error}");
        }

        (Self { main_queue: main_queue.clone(), waker, background_queue, timers, main_thread_id: thread::current().id() }, main_queue)
    }
}

impl PlatformDispatcher for AndroidDispatcher {
    fn is_main_thread(&self) -> bool {
        thread::current().id() == self.main_thread_id
    }

    fn dispatch(&self, runnable: RunnableVariant, priority: Priority) {
        self.background_queue.push(priority, runnable);
    }

    fn dispatch_on_main_thread(&self, runnable: RunnableVariant, priority: Priority) {
        self.main_queue.push(priority, runnable);
        self.waker.wake();
    }

    fn dispatch_after(&self, duration: Duration, runnable: RunnableVariant) {
        self.timers.schedule(Instant::now() + duration, runnable);
    }

    fn spawn_realtime(&self, function: Box<dyn FnOnce() + Send>) {
        let spawned = thread::Builder::new().name("Realtime".to_owned()).spawn(move || {
            // Android reserves SCHED_FIFO for privileged processes; the closest an app
            // can get is the audio thread priority, applied through the nice value.
            // SAFETY: setpriority on the calling thread has no memory-safety requirements.
            let result = unsafe { libc::setpriority(libc::PRIO_PROCESS, 0, -16) };
            if result != 0 {
                log::warn!("failed to raise realtime thread priority");
            }
            function();
        });
        if let Err(error) = spawned {
            log::error!("failed to spawn realtime thread: {error}");
        }
    }
}

fn run(runnable: RunnableVariant) {
    let location = runnable.metadata().location;
    let spawned = runnable.metadata().spawned;
    profiler::update_running_task(spawned, location);
    runnable.run();
    profiler::save_task_timing();
}

/// Runs every main-thread runnable that is queued right now. Returns whether any ran.
pub(crate) fn run_main_thread_tasks(queue: &TaskQueue) -> bool {
    let mut ran = false;
    while let Some(runnable) = queue.try_pop() {
        run(runnable);
        ran = true;
    }
    ran
}

struct TimerEntry {
    deadline: Instant,
    sequence: u64,
    runnable: RunnableVariant,
}

impl PartialEq for TimerEntry {
    fn eq(&self, other: &Self) -> bool {
        self.deadline == other.deadline && self.sequence == other.sequence
    }
}

impl Eq for TimerEntry {}

impl PartialOrd for TimerEntry {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for TimerEntry {
    // Reversed so the max-heap pops the earliest deadline first, and timers with the
    // same deadline fire in the order they were scheduled.
    fn cmp(&self, other: &Self) -> Ordering {
        other.deadline.cmp(&self.deadline).then_with(|| other.sequence.cmp(&self.sequence))
    }
}

#[derive(Default)]
struct TimerQueue {
    state: Mutex<TimerState>,
    condvar: Condvar,
}

#[derive(Default)]
struct TimerState {
    heap: BinaryHeap<TimerEntry>,
    next_sequence: u64,
}

impl TimerQueue {
    fn schedule(&self, deadline: Instant, runnable: RunnableVariant) {
        let mut state = self.state.lock();
        let sequence = state.next_sequence;
        state.next_sequence += 1;
        state.heap.push(TimerEntry { deadline, sequence, runnable });
        self.condvar.notify_one();
    }

    fn run(&self) {
        let mut state = self.state.lock();
        loop {
            let now = Instant::now();
            match state.heap.peek().map(|entry| entry.deadline) {
                Some(deadline) if deadline <= now => {
                    if let Some(entry) = state.heap.pop() {
                        drop(state);
                        run(entry.runnable);
                        state = self.state.lock();
                    }
                }
                Some(deadline) => {
                    self.condvar.wait_until(&mut state, deadline);
                }
                None => self.condvar.wait(&mut state),
            }
        }
    }
}
