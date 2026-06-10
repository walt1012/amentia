mod capabilities;
mod command_contract;
mod commands;
mod connectors;
mod hooks;
mod metadata;
mod workflow_commands;

pub use self::capabilities::build_capability_registry;
pub use self::commands::build_command_registry;
pub use self::connectors::build_connector_registry;
pub use self::hooks::build_hook_registry;

fn capability_identifier_is_safe(identifier: &str) -> bool {
  let identifier = identifier.trim();
  !identifier.is_empty()
    && identifier != "."
    && identifier != ".."
    && !identifier.contains('/')
    && !identifier.contains('\\')
    && !identifier.contains(':')
}
