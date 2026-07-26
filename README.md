# RID

[English](README.md) | [简体中文](README.zh-CN.md)

RID coordinates Windows apps through smart shortcuts: launch companion apps, temporarily close distractions, and restore only what RID closed.

Create a binding around any main application:

- open other apps alongside it;
- temporarily close selected apps before it starts;
- restore only the apps RID successfully closed when the main app exits.

For example, RID can start a voice chat tool and close screenshot software with a game, or open Codex whenever you launch Obsidian.

![RID new binding screen](docs/assets/rid-overview.png)

## Download

RID supports 64-bit Windows 10 and Windows 11.

1. Download the latest [RID Windows installer](https://github.com/itoyohane/RID/releases/latest/download/RID_0.1.0_x64-setup.exe), or browse all files on [GitHub Releases](https://github.com/itoyohane/RID/releases).
2. For most users, choose `RID_0.1.0_x64-setup.exe`.
3. Run the installer and follow the setup steps.
4. Open **RID** from the Windows Start menu.

The `RID_0.1.0_x64_en-US.msi` package is also available for managed or enterprise deployment.

> RID is currently unsigned. Windows SmartScreen may show a warning. Verify that the installer came from this repository, then choose **More info → Run anyway**.

To uninstall RID, open **Windows Settings → Apps → Installed apps**, find **RID**, and select **Uninstall**.

## Usage

1. Click **New App (新增应用)** in the sidebar.
2. Select a main app, then add apps to open or temporarily close.
3. For tray apps that cannot exit normally, enable **Force close if graceful close fails (关闭失败时强制结束)** from that app's menu. This option is off by default and may discard unsaved work.
4. Use **Dry Run (试运行)** to review the actions, then save the binding.
5. Choose a folder when RID asks where to create the shortcut.
6. Launch the generated `Main App · RID` shortcut from then on.

RID runs the close, launch, monitor, and restore workflow in the background. The hidden runner exits after the main app closes and recovery finishes.

A shortcut stores the binding ID rather than a separate copy of its configuration. Editing a binding updates its existing shortcut in place. If the shortcut was moved, renamed, or deleted, the binding still saves and RID asks for a new location.

RID discovers applications from Windows registration data, desktop shortcuts, and Start menu shortcuts. It extracts native Windows icons where possible and supports Steam `.url` game shortcuts.

## Safety

- RID requests a normal application exit by default.
- Force close must be enabled separately for each app and can discard unsaved work.
- RID restores only apps that were running beforehand and that RID successfully closed.
- Launcher child processes are tracked so updaters and self-restarting apps do not trigger recovery too early.
- Windows handles UAC prompts for elevated applications. If elevation is declined or launch fails, RID restores closed apps and reports the cause.
- Execution logs are stored in `%APPDATA%\com.rid.desktop\logs\`.
- The browser preview uses demo data and cannot control local applications.
- Apps running only in the system tray or with elevated permissions may not close from a normal RID process.

## Current features

- Create, edit, and delete app bindings.
- Search registered applications and desktop or Start menu shortcuts.
- Read `.lnk` targets, arguments, and working directories.
- Discover and launch Steam game shortcuts.
- Extract native application icons.
- Run a dry run before executing a binding.
- Create a launcher shortcut in a user-selected folder.
- Execute bindings in a hidden background runner.
- Support UAC-aware launches and per-app force-close fallback.
- Restore only applications RID closed.

## Development

Requirements:

- Node.js
- Rust
- Windows WebView2 development environment

Run the desktop app:

```powershell
npm install
npm run tauri:dev
```

Run checks:

```powershell
npm run check
```

Build Windows installers:

```powershell
npm run tauri:build
```

Build outputs:

- `src-tauri/target/release/bundle/nsis/` — recommended `.exe` installer;
- `src-tauri/target/release/bundle/msi/` — MSI package.

Run the browser-only UI preview:

```powershell
npm run dev
```

The browser preview cannot discover or control applications. Run the Tauri desktop app for native Windows behavior.

See [Architecture](docs/architecture.md) for implementation details.
