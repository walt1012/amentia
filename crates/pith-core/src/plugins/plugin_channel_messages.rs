use std::collections::HashMap;

use pith_protocol::{
  JsonRpcRequest, JsonRpcResponse, PluginChannelInboundPreviewParams,
  PluginChannelInboundPreviewResult, PluginChannelOutboundPreviewParams,
  PluginChannelOutboundPreviewResult, PluginChannelOutboundRequestParams, TimelineItem,
  TurnStartResult,
};
use serde::Serialize;

use crate::approval_state::approvals_for_thread;
use crate::approval_types::PendingApproval;
use crate::request_params::parse_required_params;
use crate::thread_summary::refresh_thread_summary_note;
use crate::RuntimeContext;

const CHANNEL_OUTBOUND_APPROVAL_ACTION: &str = "send_channel_message";

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

pub(crate) fn handle_plugin_channel_outbound_request(
  context: &mut RuntimeContext,
  request: JsonRpcRequest,
) -> JsonRpcResponse {
  let params = match parse_required_params::<PluginChannelOutboundRequestParams>(
    &request,
    "plugin/channelOutboundRequest",
  ) {
    Ok(params) => params,
    Err(response) => return response,
  };

  let thread_id = params.thread_id.trim().to_string();
  let external_conversation_id = params.external_conversation_id.trim().to_string();
  let normalized_text = params.text.trim().to_string();
  if thread_id.is_empty() {
    return channel_message_error_response(
      request.id,
      -32063,
      "invalidEnvelope",
      &params.channel_id,
      None,
      "Channel outbound thread id must not be empty.",
    );
  }
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
  let current_workspace = context.workspace_state.current_cloned();
  {
    let Some(thread) = context.thread_state.find_mut(&thread_id) else {
      return JsonRpcResponse::error(request.id, -32004, "Thread not found");
    };
    thread.bind_workspace_if_missing(current_workspace);
    if thread.workspace_cloned().is_none() {
      return JsonRpcResponse::error(
        request.id,
        -32031,
        "Open a workspace for this thread before requesting channel sends",
      );
    }
  }
  let approval_id = context.sequence_state.next_approval_id();
  let reply_to_external_message_id =
    normalized_external_message_id(params.reply_to_external_message_id);
  let (approval, items) = build_channel_outbound_approval_request(
    approval_id,
    &thread_id,
    &channel,
    &external_conversation_id,
    reply_to_external_message_id.as_deref(),
    &normalized_text,
  );
  context.execution_state.insert_pending_approval(approval);

  let prepared_thread = {
    let thread = context
      .thread_state
      .find_mut(&thread_id)
      .expect("checked channel outbound thread");
    let prepared_thread = thread.begin_channel_message();
    thread.append_items(items.clone());
    thread.mark_ready();
    prepared_thread
  };
  if let Err(error) = context.persist_runtime_state() {
    return JsonRpcResponse::error(request.id, -32010, error.to_string());
  }
  if let Err(error) = refresh_thread_summary_note(context, &thread_id) {
    return JsonRpcResponse::error(request.id, -32012, error.to_string());
  }

  JsonRpcResponse::success(
    request.id,
    &TurnStartResult {
      turn_id: prepared_thread.turn_id,
      thread_id: thread_id.clone(),
      items,
      pending_approvals: approvals_for_thread(context, &thread_id),
      active_turn_id: None,
    },
  )
}

pub(crate) fn build_channel_outbound_approval_request(
  approval_id: String,
  thread_id: &str,
  channel: &pith_plugin_host::PluginChannelEntry,
  external_conversation_id: &str,
  reply_to_external_message_id: Option<&str>,
  normalized_text: &str,
) -> (PendingApproval, Vec<TimelineItem>) {
  let approval = PendingApproval {
    id: approval_id,
    thread_id: thread_id.to_string(),
    action: CHANNEL_OUTBOUND_APPROVAL_ACTION.to_string(),
    title: format!("Send message to {}", channel.display_name),
    relative_path: format!("channel:{}", channel.channel_id),
    content: Some(normalized_text.to_string()),
    command: None,
  };
  let mut approval_attributes = HashMap::from([
    ("approvalId".to_string(), approval.id.clone()),
    ("action".to_string(), approval.action.clone()),
    ("channelId".to_string(), channel.channel_id.clone()),
    ("channelService".to_string(), channel.service.clone()),
    ("channelProtocol".to_string(), channel.protocol.clone()),
    ("pluginId".to_string(), channel.plugin_id.clone()),
    (
      "pluginDisplayName".to_string(),
      channel.plugin_display_name.clone(),
    ),
    (
      "externalConversationId".to_string(),
      external_conversation_id.to_string(),
    ),
    ("channelMessage".to_string(), normalized_text.to_string()),
  ]);
  if let Some(reply_to_external_message_id) = reply_to_external_message_id {
    approval_attributes.insert(
      "replyToExternalMessageId".to_string(),
      reply_to_external_message_id.to_string(),
    );
  }

  (
    approval.clone(),
    vec![
      TimelineItem {
        kind: "approvalRequested".to_string(),
        title: "Channel Send Approval Requested".to_string(),
        content: format!(
          "Pith needs approval before sending a message through {} to conversation {}.\nMessage: {}",
          channel.display_name, external_conversation_id, normalized_text
        ),
        attributes: Some(approval_attributes),
      },
      TimelineItem {
        kind: "assistantMessage".to_string(),
        title: "Assistant".to_string(),
        content:
          "Pith is waiting for your approval before sending this channel message."
            .to_string(),
        attributes: Some(HashMap::from([
          ("approvalId".to_string(), approval.id.clone()),
          ("channelId".to_string(), channel.channel_id.clone()),
          ("pluginId".to_string(), channel.plugin_id.clone()),
        ])),
      },
    ],
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
