use amentia_model_runtime::LocalModelRuntime;

const DEFAULT_MODEL_CONTEXT_TOKENS: usize = 4096;
const CONTEXT_MEMORY_BUDGET_PERCENT: usize = 30;
const MIN_CONTEXT_MEMORY_CHAR_BUDGET: usize = 900;
const MAX_CONTEXT_MEMORY_CHAR_BUDGET: usize = 2400;

pub(super) fn context_budget_for_model(model_runtime: &LocalModelRuntime) -> (usize, usize) {
  let health = model_runtime.health();
  let context_window_tokens = health
    .metrics
    .get("contextSize")
    .and_then(|value| value.parse::<usize>().ok())
    .filter(|value| *value > 0)
    .unwrap_or(DEFAULT_MODEL_CONTEXT_TOKENS);
  let raw_budget = context_window_tokens.saturating_mul(CONTEXT_MEMORY_BUDGET_PERCENT) / 100;
  let budget_char_count = raw_budget.clamp(
    MIN_CONTEXT_MEMORY_CHAR_BUDGET,
    MAX_CONTEXT_MEMORY_CHAR_BUDGET,
  );
  (budget_char_count, context_window_tokens)
}
