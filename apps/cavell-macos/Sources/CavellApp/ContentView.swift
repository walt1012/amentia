import AppKit
import SwiftUI

struct ContentView: View {
  @ObservedObject var viewModel: AppViewModel

  var body: some View {
    NavigationView {
      sidebar
      timeline
      inspector
    }
    .toolbar {
      ToolbarItem(placement: .primaryAction) {
        Button("Launch Runtime") {
          viewModel.launchRuntime()
        }
      }
    }
  }

  private var sidebar: some View {
    List(selection: $viewModel.selectedThreadID) {
      Section("Threads") {
        ForEach(viewModel.threads) { thread in
          VStack(alignment: .leading, spacing: 4) {
            Text(thread.title)
              .font(.headline)
            Text(thread.preview)
              .font(.caption)
              .foregroundColor(.secondary)
          }
          .padding(.vertical, 4)
          .tag(thread.id)
        }
      }
    }
    .frame(minWidth: 240)
    .listStyle(.sidebar)
  }

  private var timeline: some View {
    VStack(alignment: .leading, spacing: 0) {
      HStack {
        Text("Timeline")
          .font(.title2.weight(.semibold))
        Spacer()
        VStack(alignment: .trailing, spacing: 2) {
          Text(viewModel.runtimeState.rawValue.capitalized)
            .font(.caption.weight(.medium))
            .foregroundColor(.secondary)
          Text(viewModel.runtimeDetail)
            .font(.caption2)
            .foregroundColor(.secondary)
        }
      }
      .padding(20)

      Divider()

      ScrollView {
        VStack(alignment: .leading, spacing: 16) {
          ForEach(viewModel.timeline) { entry in
            TimelineCard(entry: entry)
          }
        }
        .padding(20)
      }
    }
    .frame(minWidth: 520)
  }

  private var inspector: some View {
    VStack(alignment: .leading, spacing: 16) {
      Text("Inspector")
        .font(.title3.weight(.semibold))

      GroupBox("Milestone 0") {
        VStack(alignment: .leading, spacing: 8) {
          Text("Local app shell")
          Text("Rust runtime workspace")
          Text("Protocol scaffold")
          Text("Plugin-ready repository layout")
        }
        .frame(maxWidth: .infinity, alignment: .leading)
        .font(.subheadline)
      }

      GroupBox("Next Integration") {
        Text("Replace the mock runtime state with live stdio messaging and render real timeline events.")
          .font(.subheadline)
          .foregroundColor(.secondary)
      }

      Spacer()
    }
    .padding(20)
    .frame(minWidth: 280)
  }
}

private struct TimelineCard: View {
  let entry: TimelineEntry

  var body: some View {
    VStack(alignment: .leading, spacing: 8) {
      Text(entry.title)
        .font(.headline)
      Text(entry.body)
        .font(.body)
        .foregroundColor(.secondary)
    }
    .padding(16)
    .frame(maxWidth: .infinity, alignment: .leading)
    .background(Color(NSColor.controlBackgroundColor))
    .clipShape(RoundedRectangle(cornerRadius: 12, style: .continuous))
  }
}

private struct SettingsView: View {
  var body: some View {
    Form {
      Section("Model") {
        Text("Default built-in model: LFM2.5-350M")
      }

      Section("Platform") {
        Text("Target: macOS 12+ on Intel")
      }
    }
    .padding(20)
    .frame(width: 420)
  }
}
