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
const MAIN_TASKS_PER_TURN: usize = 64;

/// A FIFO per priority level; higher levels are always drained first.
#[derive(Default)]
pub(crate) struct TaskQueue {
    state: Mutex<TaskQueueState>,
    available: Condvar,
}

#[derive(Default)]
struct TaskQueueState {
    lanes: [VecDeque<RunnableVariant>; 3],
    closed: bool,
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
        let mut state = self.state.lock();
        if state.closed {
            return;
        }
        state.lanes[Self::lane(priority)].push_back(runnable);
        self.available.notify_one();
    }

    fn try_pop(&self) -> Option<RunnableVariant> {
        self.state.lock().lanes.iter_mut().find_map(VecDeque::pop_front)
    }

    pub(crate) fn has_pending(&self) -> bool {
        self.state.lock().lanes.iter().any(|lane| !lane.is_empty())
    }

    fn pop_blocking(&self) -> Option<RunnableVariant> {
        let mut state = self.state.lock();
        while !state.closed {
            if let Some(runnable) = state.lanes.iter_mut().find_map(VecDeque::pop_front) {
                return Some(runnable);
            }
            self.available.wait(&mut state);
        }
        None
    }

    fn close(&self) {
        let mut state = self.state.lock();
        state.closed = true;
        let pending = std::mem::take(&mut state.lanes);
        self.available.notify_all();
        drop(state);
        // Cancelling a runnable may wake tasks that dispatch again.
        drop(pending);
    }

    fn take_pending(&self) -> [VecDeque<RunnableVariant>; 3] {
        std::mem::take(&mut self.state.lock().lanes)
    }
}

pub(crate) struct AndroidDispatcher {
    main_queue: Arc<TaskQueue>,
    waker: Mutex<Option<AndroidAppWaker>>,
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
                while let Some(runnable) = queue.pop_blocking() {
                    run(runnable);
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

        (Self { main_queue: main_queue.clone(), waker: Mutex::new(Some(waker)), background_queue, timers, main_thread_id: thread::current().id() }, main_queue)
    }

    pub(crate) fn shutdown(&self) {
        assert!(self.is_main_thread());
        // In-flight work may still need the activity thread. Let it finish without joining here.
        self.background_queue.close();
        self.timers.close();
        let pending = {
            let mut waker = self.waker.lock();
            waker.take();
            self.main_queue.take_pending()
        };
        drop(pending);
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
        let waker = self.waker.lock();
        if waker.is_none() && self.is_main_thread() {
            return;
        }
        // The pinned dispatcher API cannot distinguish Send tasks from thread-affine
        // ones. Preserve enqueueing for foreign wakes; dropping their runnable here
        // may drop a local future on the wrong thread.
        self.main_queue.push(priority, runnable);
        if let Some(waker) = &*waker {
            waker.wake();
        }
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

/// Limits consecutive foreground work so tasks that reschedule cannot starve Android events.
pub(crate) fn run_main_thread_tasks(queue: &TaskQueue, should_quit: impl Fn() -> bool) {
    for _ in 0..MAIN_TASKS_PER_TURN {
        if should_quit() {
            break;
        }
        let Some(runnable) = queue.try_pop() else {
            break;
        };
        run(runnable);
    }
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
    closed: bool,
}

impl TimerQueue {
    fn schedule(&self, deadline: Instant, runnable: RunnableVariant) {
        let mut state = self.state.lock();
        if state.closed {
            return;
        }
        let sequence = state.next_sequence;
        state.next_sequence += 1;
        state.heap.push(TimerEntry { deadline, sequence, runnable });
        self.condvar.notify_one();
    }

    fn run(&self) {
        let mut state = self.state.lock();
        while !state.closed {
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

    fn close(&self) {
        let mut state = self.state.lock();
        state.closed = true;
        let pending = std::mem::take(&mut state.heap);
        self.condvar.notify_one();
        drop(state);
        drop(pending);
    }
}
