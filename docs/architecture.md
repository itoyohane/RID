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
| `create_binding_shortcut` | `{ id, directory }` | generated `.lnk` path |
| `dry_run_binding` | `{ binding }` | `ExecutionReport` |
| `launch_binding` | `{ binding }` | `ExecutionReport` |

Tauri serializes Rust snake_case fields to the JSON shape consumed by the bridge.

## Application discovery

The Windows catalog merges registered `App Paths`, uninstall metadata, user/shared
desktop shortcuts, and user/shared Start menu shortcuts. `.lnk` files are resolved
through `IShellLinkW`, including their raw launch arguments and working directory.
Results are deduplicated by executable path plus launch arguments and cached for the
desktop session.

Associated Windows icons are requested from the native shell extractor at 256 pixels,
rendered to 128-pixel PNG data URLs in Rust, and only fall back to `ExtractIconExW`
when the high-resolution resource cannot be loaded. The webview can therefore display
crisp local application icons without exposing a general filesystem protocol.

## Binding lifecycle

1. Validate every executable path and reject protected/system targets.
2. Snapshot which configured close-app processes are currently running.
3. Ask those processes to close normally.
4. Include hidden top-level windows for tray applications. If cooperative close
   fails, force termination is available only for application IDs explicitly
   opted in by the user.
5. Record only the applications that actually stopped.
6. Launch configured companion applications that are not already running.
7. Launch the main application through `ShellExecuteExW`. Windows handles manifest
   driven UAC, while `SEE_MASK_NOCLOSEPROCESS` returns a waitable process handle.
8. Monitor the launch handle, its descendant process family, and matching process IDs
   without blocking the Tauri UI thread. Require an eight-second quiet period before
   considering the main application stopped so updater-driven process replacement does
   not trigger premature recovery.
9. When the main application exits, relaunch only the applications recorded in step 5.

The MVP never force-kills a process by default.

Every launch report is persisted under the application data `logs` directory. Main
launch failures are fatal: RID restores any applications it already closed, displays
a native error, and exits. Partial close/open failures are displayed as warnings while
the successfully launched main application continues.

## Generated shortcut runtime

RID creates the user-facing `.lnk` with Windows `IShellLinkW`. The link targets the
installed RID executable, passes `--run-binding <binding-id>`, and uses the main
application executable as its icon source. It does not copy the binding into the
shortcut, so later edits remain effective. The generated path is persisted with the
binding; subsequent saves rewrite that exact `.lnk` in place to refresh its metadata,
target executable, and icon without asking for a directory again.

The Tauri window is initially hidden. A normal launch shows and focuses the window.
A shortcut launch keeps it hidden, loads the saved binding by ID, runs the binding
lifecycle on a worker thread, and shows/focuses the RID window after recovery
completes instead of terminating RID with the main application. Startup failures are
shown with a native error dialog.

## Persistence

Bindings are stored as versioned JSON under the Tauri application data directory. Writes use a temporary sibling file followed by replacement to reduce the chance of leaving a partially written configuration.

## Platform scope

The initial release targets Windows. Non-Windows development builds may return mock or limited application discovery results, but they must compile and must not claim full process-control support.
