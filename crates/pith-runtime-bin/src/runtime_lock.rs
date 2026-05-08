use std::sync::{Arc, Mutex, MutexGuard, TryLockError};

use pith_core::RuntimeContext;

pub(crate) type SharedRuntimeContext = Arc<Mutex<RuntimeContext>>;

pub(crate) fn lock_context(context: &SharedRuntimeContext) -> MutexGuard<'_, RuntimeContext> {
  context
    .lock()
    .unwrap_or_else(|poisoned| poisoned.into_inner())
}

pub(crate) fn try_lock_context(
  context: &SharedRuntimeContext,
) -> Option<MutexGuard<'_, RuntimeContext>> {
  match context.try_lock() {
    Ok(guard) => Some(guard),
    Err(TryLockError::WouldBlock) => None,
    Err(TryLockError::Poisoned(poisoned)) => Some(poisoned.into_inner()),
  }
}

#[cfg(test)]
mod tests {
  use std::panic::{catch_unwind, AssertUnwindSafe};

  use super::*;

  #[test]
  fn lock_context_recovers_from_poisoned_mutex() {
    let context = Arc::new(Mutex::new(RuntimeContext::new_in_memory()));
    let poisoned_context = Arc::clone(&context);

    let _ = catch_unwind(AssertUnwindSafe(|| {
      let _guard = poisoned_context.lock().expect("runtime context lock");
      panic!("poison runtime context lock");
    }));

    let _guard = lock_context(&context);
  }

  #[test]
  fn try_lock_context_recovers_from_poisoned_mutex() {
    let context = Arc::new(Mutex::new(RuntimeContext::new_in_memory()));
    let poisoned_context = Arc::clone(&context);

    let _ = catch_unwind(AssertUnwindSafe(|| {
      let _guard = poisoned_context.lock().expect("runtime context lock");
      panic!("poison runtime context lock");
    }));

    assert!(try_lock_context(&context).is_some());
  }
}
