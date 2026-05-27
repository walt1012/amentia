use pith_protocol::{
  JsonRpcRequest, JsonRpcResponse, PluginChannelInboundPreviewParams,
  PluginChannelInboundPreviewResult, PluginChannelOutboundPreviewParams,
  PluginChannelOutboundPreviewResult,
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
    return channel_message_error_response(
      request.id,
      -32063,
      "invalidEnvelope",
      &params.channel_id,
      None,
      "Channel inbound conversation id must not be empty.",
    );
  }
  if external_message_id.is_empty() {
    return channel_message_error_response(
      request.id,
      -32063,
      "invalidEnvelope",
      &params.channel_id,
      None,
      "Channel inbound message id must not be empty.",
    );
  }
  if normalized_text.is_empty() {
    return channel_message_error_response(
      request.id,
      -32062,
      "emptyMessage",
      &params.channel_id,
      None,
      "Channel inbound message text must not be empty.",
    );
  }

  let channel = match validated_channel(context, request.id.clone(), &params.channel_id) {
    Ok(channel) => channel,
    Err(response) => return response,
  };

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

pub(crate) fn handle_plugin_channel_outbound_preview(
  context: &RuntimeContext,
  request: JsonRpcRequest,
) -> JsonRpcResponse {
  let params = match parse_required_params::<PluginChannelOutboundPreviewParams>(
    &request,
    "plugin/channelOutboundPreview",
  ) {
    Ok(params) => params,
    Err(response) => return response,
  };

  let external_conversation_id = params.external_conversation_id.trim().to_string();
  let normalized_text = params.text.trim().to_string();
  if external_conversation_id.is_empty() {
    return channel_message_error_response(
      request.id,
      -32063,
      "invalidEnvelope",
      &params.channel_id,
      None,
      "Channel outbound conversation id must not be empty.",
    );
  }
  if normalized_text.is_empty() {
    return channel_message_error_response(
      request.id,
      -32062,
      "emptyMessage",
      &params.channel_id,
      None,
      "Channel outbound message text must not be empty.",
    );
  }

  let channel = match validated_channel(context, request.id.clone(), &params.channel_id) {
    Ok(channel) => channel,
    Err(response) => return response,
  };

  JsonRpcResponse::success(
    request.id,
    &PluginChannelOutboundPreviewResult {
      channel_id: channel.channel_id,
      service: channel.service,
      protocol: channel.protocol,
      plugin_id: channel.plugin_id,
      plugin_display_name: channel.plugin_display_name,
      external_conversation_id,
      reply_to_external_message_id: normalized_external_message_id(
        params.reply_to_external_message_id,
      ),
      normalized_text,
      status: "needsApproval".to_string(),
      approval_required: true,
      accepted: true,
    },
  )
}

fn validated_channel(
  context: &RuntimeContext,
  request_id: serde_json::Value,
  channel_id: &str,
) -> Result<pith_plugin_host::PluginChannelEntry, JsonRpcResponse> {
  let Some(channel) = context
    .plugin_state
    .channel_entries()
    .into_iter()
    .find(|channel| channel.channel_id == channel_id)
  else {
    return Err(channel_message_error_response(
      request_id,
      -32060,
      "notFound",
      channel_id,
      None,
      "Channel not found.",
    ));
  };

  if !channel.adapter_available {
    return Err(channel_message_error_response(
      request_id,
      -32058,
      "channelAdapterPending",
      channel_id,
      Some(&channel),
      channel
        .activation_blocker
        .clone()
        .unwrap_or_else(|| "Channel adapter is not available yet.".to_string()),
    ));
  }

  if !channel.enabled {
    return Err(channel_message_error_response(
      request_id,
      -32061,
      "disabled",
      channel_id,
      Some(&channel),
      "Channel plugin is disabled.",
    ));
  }

  Ok(channel)
}

fn channel_message_error_response(
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
    &PluginChannelMessageRecovery {
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

fn normalized_external_message_id(external_message_id: Option<String>) -> Option<String> {
  external_message_id
    .map(|message_id| message_id.trim().to_string())
    .filter(|message_id| !message_id.is_empty())
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct PluginChannelMessageRecovery<'a> {
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
