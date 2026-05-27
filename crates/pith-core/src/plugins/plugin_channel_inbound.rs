use pith_protocol::{
  JsonRpcRequest, JsonRpcResponse, PluginChannelInboundPreviewParams,
  PluginChannelInboundPreviewResult,
};
use serde::Serialize;

use crate::request_params::parse_required_params;
use crate::RuntimeContext;

pub(crate) fn handle_plugin_channel_inbound_preview(
  context: &RuntimeContext,
  request: JsonRpcRequest,
) -> JsonRpcResponse {
  let params = match parse_required_params::<PluginChannelInboundPreviewParams>(
    &request,
    "plugin/channelInboundPreview",
  ) {
    Ok(params) => params,
    Err(response) => return response,
  };

  let external_conversation_id = params.external_conversation_id.trim().to_string();
  let external_message_id = params.external_message_id.trim().to_string();
  let normalized_text = params.text.trim().to_string();
  if external_conversation_id.is_empty() {
    return channel_inbound_error_response(
      request.id,
      -32063,
      "invalidEnvelope",
      &params.channel_id,
      None,
      "Channel inbound conversation id must not be empty.",
    );
  }
  if external_message_id.is_empty() {
    return channel_inbound_error_response(
      request.id,
      -32063,
      "invalidEnvelope",
      &params.channel_id,
      None,
      "Channel inbound message id must not be empty.",
    );
  }
  if normalized_text.is_empty() {
    return channel_inbound_error_response(
      request.id,
      -32062,
      "emptyMessage",
      &params.channel_id,
      None,
      "Channel inbound message text must not be empty.",
    );
  }

  let Some(channel) = context
    .plugin_state
    .channel_entries()
    .into_iter()
    .find(|channel| channel.channel_id == params.channel_id)
  else {
    return channel_inbound_error_response(
      request.id,
      -32060,
      "notFound",
      &params.channel_id,
      None,
      "Channel not found.",
    );
  };

  if !channel.adapter_available {
    return channel_inbound_error_response(
      request.id,
      -32058,
      "channelAdapterPending",
      &params.channel_id,
      Some(&channel),
      channel
        .activation_blocker
        .clone()
        .unwrap_or_else(|| "Channel adapter is not available yet.".to_string()),
    );
  }

  if !channel.enabled {
    return channel_inbound_error_response(
      request.id,
      -32061,
      "disabled",
      &params.channel_id,
      Some(&channel),
      "Channel plugin is disabled.",
    );
  }

  JsonRpcResponse::success(
    request.id,
    &PluginChannelInboundPreviewResult {
      channel_id: channel.channel_id,
      service: channel.service,
      protocol: channel.protocol,
      plugin_id: channel.plugin_id,
      plugin_display_name: channel.plugin_display_name,
      external_conversation_id,
      external_message_id,
      sender_label: normalized_sender_label(params.sender_label),
      normalized_text,
      status: "accepted".to_string(),
      accepted: true,
    },
  )
}

fn channel_inbound_error_response(
  request_id: serde_json::Value,
  code: i32,
  status: &str,
  channel_id: &str,
  channel: Option<&pith_plugin_host::PluginChannelEntry>,
  message: impl Into<String>,
) -> JsonRpcResponse {
  JsonRpcResponse::error_with_data(
    request_id,
    code,
    message,
    &PluginChannelInboundRecovery {
      channel_id,
      status,
      service: channel.map(|channel| channel.service.as_str()),
      protocol: channel.map(|channel| channel.protocol.as_str()),
      plugin_id: channel.map(|channel| channel.plugin_id.as_str()),
      adapter_status: channel.map(|channel| channel.adapter_status.as_str()),
      adapter_available: channel.map(|channel| channel.adapter_available),
      activation_blocker: channel.and_then(|channel| channel.activation_blocker.as_deref()),
    },
  )
}

fn normalized_sender_label(sender_label: Option<String>) -> Option<String> {
  sender_label
    .map(|label| label.trim().to_string())
    .filter(|label| !label.is_empty())
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct PluginChannelInboundRecovery<'a> {
  channel_id: &'a str,
  status: &'a str,
  #[serde(skip_serializing_if = "Option::is_none")]
  service: Option<&'a str>,
  #[serde(skip_serializing_if = "Option::is_none")]
  protocol: Option<&'a str>,
  #[serde(skip_serializing_if = "Option::is_none")]
  plugin_id: Option<&'a str>,
  #[serde(skip_serializing_if = "Option::is_none")]
  adapter_status: Option<&'a str>,
  #[serde(skip_serializing_if = "Option::is_none")]
  adapter_available: Option<bool>,
  #[serde(skip_serializing_if = "Option::is_none")]
  activation_blocker: Option<&'a str>,
}
