use crate::runtime_context::RuntimeContext;

impl RuntimeContext {
  pub fn cancel_running_work(&mut self) {
    self.execution_state.cancel_running_work();
  }
}
