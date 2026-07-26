# RID MVP Architecture

## Runtime boundary

The Next.js frontend is a static-exported client application. It never uses Next.js server actions, route handlers, or SSR-only APIs at runtime. All privileged desktop behavior crosses a typed Tauri command bridge.

The browser development build uses deterministic mock data. This keeps UI development and end-to-end tests safe: opening the browser cannot launch or close real processes.

## Command contract

The frontend uses these Tauri commands:

| Command | Input | Output |
| --- | --- | --- |
| `list_installed_apps` | optional search query | `AppDescriptor[]` |
| `list_bindings` | none | `Binding[]` |
| `save_binding` | `{ binding }` | saved `Binding` |
| `delete_binding` | `{ id }` | none |
| `dry_run_binding` | `{ binding }` | `ExecutionReport` |
| `launch_binding` | `{ binding }` | `ExecutionReport` |

Tauri serializes Rust snake_case fields to the JSON shape consumed by the bridge.

## Application discovery

The Windows catalog merges registered `App Paths`, uninstall metadata, user/shared
desktop shortcuts, and user/shared Start menu shortcuts. `.lnk` files are resolved
through `IShellLinkW`, including their raw launch arguments and working directory.
Results are deduplicated by executable path plus launch arguments and cached for the
desktop session.

Associated Windows icons are rendered to small PNG data URLs in Rust, so the webview
can display local application icons without exposing a general filesystem protocol.

## Binding lifecycle

1. Validate every executable path and reject protected/system targets.
2. Snapshot which configured close-app processes are currently running.
3. Ask those processes to close normally.
4. Record only the applications that actually stopped.
5. Launch configured companion applications that are not already running.
6. Launch the main application.
7. Monitor the launched main process without blocking the Tauri UI thread.
8. When it exits, relaunch only the applications recorded in step 4.

The MVP never force-kills a process by default.

## Persistence

Bindings are stored as versioned JSON under the Tauri application data directory. Writes use a temporary sibling file followed by replacement to reduce the chance of leaving a partially written configuration.

## Platform scope

The initial release targets Windows. Non-Windows development builds may return mock or limited application discovery results, but they must compile and must not claim full process-control support.
