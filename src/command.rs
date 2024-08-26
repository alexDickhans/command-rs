use alloc::sync::Arc;
use core::cell::RefCell;
use core::cmp::Ordering;
use core::time::Duration;
use vexide::core::time::Instant;
use crate::subsystem::{AnySubsystem};

#[derive(Debug, Eq, PartialEq, Clone)]
pub enum CancelBehavior {
    CancelIncoming,
    CancelRunning
}

pub trait Command {

    fn initialize(&mut self) {}

    fn execute(&mut self) {}

    #[allow(unused_variables)]
    fn end(&mut self, interrupted: bool) {}

    fn finished(&self) -> bool {
        false
    }

    fn requirements(&self) -> &[AnySubsystem];

    fn runs_when_disabled(&self) -> bool {
        false
    }

    fn cancel_behavior(&self) -> CancelBehavior {
        CancelBehavior::CancelRunning
    }
}

pub struct AnyCommand(pub Arc<RefCell<dyn Command>>);

impl AnyCommand {
    pub fn new(command: impl Command + 'static) -> Self {
        Self(Arc::new(RefCell::new(command)))
    }
}

impl Clone for AnyCommand {
    fn clone(&self) -> Self {
        Self (self.0.clone())
    }
}

impl PartialEq for AnyCommand {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

impl Eq for AnyCommand {}

impl PartialOrd for AnyCommand {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        let ptr_self = Arc::as_ptr(&self.0);
        let ptr_other = Arc::as_ptr(&other.0);

        Some(ptr_self.cast::<()>().cmp(&ptr_other.cast::<()>()))
    }
}

impl Ord for AnyCommand {
    fn cmp(&self, other: &Self) -> Ordering {
        let ptr_self = Arc::as_ptr(&self.0);
        let ptr_other = Arc::as_ptr(&other.0);

        ptr_self.cast::<()>().cmp(&ptr_other.cast::<()>())
    }
}

impl From<Arc<RefCell<dyn Command>>> for AnyCommand {
    fn from(value: Arc<RefCell<dyn Command>>) -> Self {
        Self(value)
    }
}

impl<T> From<Arc<RefCell<T>>> for AnyCommand where T: Command + 'static {
    fn from(value: Arc<RefCell<T>>) -> Self {
        Self(value)
    }
}

pub trait CommandExt {
}

impl CommandExt for AnyCommand {
}

pub struct WaitCommand{
    duration: Duration,
    start_time: Option<Instant>,
}

impl WaitCommand {
    pub fn new(duration: Duration) -> Self {
        Self {
            duration,
            start_time: None
        }
    }
}

impl Command for WaitCommand {
    fn initialize(&mut self) {
        self.start_time = Some(Instant::now());
    }

    fn finished(&self) -> bool {
        Instant::now() - self.start_time.expect("Initialize failed to run") > self.duration
    }

    fn requirements(&self) -> &[AnySubsystem] {
        &[]
    }
}
