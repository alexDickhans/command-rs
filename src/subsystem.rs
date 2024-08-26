use alloc::sync::Arc;
use core::cell::RefCell;
use core::cmp::Ordering;
use core::fmt::Debug;

pub trait Subsystem: Debug + 'static {
    fn periodic(&mut self) {}
}

#[derive(Debug)]
pub struct AnySubsystem(pub Arc<RefCell<dyn Subsystem>>);

impl AnySubsystem {
    pub fn new(subsystem: impl Subsystem) -> Self {
        Self(Arc::new(RefCell::new(subsystem)))
    }
}

impl Clone for AnySubsystem {
    fn clone(&self) -> Self {
        Self (self.0.clone())
    }
}

impl PartialEq for AnySubsystem {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

impl Eq for AnySubsystem {}

impl PartialOrd for AnySubsystem {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        let ptr_self = Arc::as_ptr(&self.0);
        let ptr_other = Arc::as_ptr(&other.0);

        Some(ptr_self.cast::<()>().cmp(&ptr_other.cast::<()>()))
    }
}

impl Ord for AnySubsystem {
    fn cmp(&self, other: &Self) -> Ordering {
        let ptr_self = Arc::as_ptr(&self.0);
        let ptr_other = Arc::as_ptr(&other.0);

        ptr_self.cast::<()>().cmp(&ptr_other.cast::<()>())
    }
}

impl From<Arc<RefCell<dyn Subsystem>>> for AnySubsystem {
    fn from(value: Arc<RefCell<dyn Subsystem>>) -> Self {
        Self(value)
    }
}


impl<T> From<Arc<RefCell<T>>> for AnySubsystem where T: Subsystem {
    fn from(value: Arc<RefCell<T>>) -> Self {
        Self(value)
    }
}

pub trait SubsystemExt {
}

impl SubsystemExt for AnySubsystem {
}
