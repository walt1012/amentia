use pith_plugin_host::PluginHookEntry as HostPluginHookEntry;
use pith_tools::ShellSandboxSummary;

#[derive(Debug, Clone)]
pub(crate) struct PluginHookMemoryCapture {
  pub(crate) hook: HostPluginHookEntry,
  pub(crate) content: String,
  pub(crate) command: String,
  pub(crate) exit_code: i32,
  pub(crate) sandbox: ShellSandboxSummary,
  pub(crate) stdout_preview: String,
  pub(crate) stderr_preview: String,
}
