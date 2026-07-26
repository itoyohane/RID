export type AppIconKey =
  | "cyberpunk"
  | "obsidian"
  | "accelerator"
  | "screenshot"
  | "codex"
  | "vscode"
  | "folder"
  | "rocket"
  | "app";

export interface AppInfo {
  id: string;
  name: string;
  path: string;
  launchArguments?: string;
  workingDirectory?: string;
  aliases: string[];
  category?: string;
  iconKey?: AppIconKey;
  iconUrl?: string;
}

export interface Binding {
  id: string;
  mainApp: AppInfo;
  openApps: AppInfo[];
  closeApps: AppInfo[];
  forceCloseAppIds: string[];
}

export interface BindingDraft {
  id?: string;
  mainApp: AppInfo | null;
  openApps: AppInfo[];
  closeApps: AppInfo[];
  forceCloseAppIds: string[];
}

export type PickerGroup = "mainApp" | "openApps" | "closeApps";

export interface RunStep {
  appId: string;
  appName: string;
  action: string;
  status: "success" | "skipped" | "failed";
  message?: string;
}

export interface RunResult {
  success: boolean;
  message: string;
  steps: RunStep[];
  executionId?: string;
  recoveryPending?: boolean;
}
