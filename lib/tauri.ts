import { mockApps, mockBindings } from "@/lib/mock-data";
import type { AppInfo, Binding, BindingDraft, RunResult } from "@/lib/types";

const MOCK_BINDINGS_KEY = "rid.mock.bindings.v1";

function isTauriRuntime() {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

function copy<T>(value: T): T {
  return JSON.parse(JSON.stringify(value)) as T;
}

function readMockBindings(): Binding[] {
  if (typeof window === "undefined") return copy(mockBindings);
  const value = window.localStorage.getItem(MOCK_BINDINGS_KEY);
  if (!value) return copy(mockBindings);

  try {
    return JSON.parse(value) as Binding[];
  } catch {
    return copy(mockBindings);
  }
}

function writeMockBindings(bindings: Binding[]) {
  window.localStorage.setItem(MOCK_BINDINGS_KEY, JSON.stringify(bindings));
}

async function tauriInvoke<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<T>(command, args);
}

interface NativeApp {
  id: string;
  name: string;
  path: string;
  launch_arguments?: string | null;
  working_directory?: string | null;
  icon?: string | null;
  category?: string;
  aliases?: string[];
}

interface NativeBinding {
  id: string;
  name?: string | null;
  main_app: NativeApp;
  open_apps: NativeApp[];
  close_apps: NativeApp[];
}

interface NativeReport {
  execution_id: string;
  binding_id?: string | null;
  mode: string;
  started_at: string;
  operations: Array<{
    app: NativeApp;
    action: string;
    status: "success" | "skipped" | "failed";
    message?: string | null;
  }>;
  recovery_pending: boolean;
}

function inferIcon(app: NativeApp): Pick<AppInfo, "iconKey" | "iconUrl"> {
  const key = `${app.name} ${app.path}`.toLocaleLowerCase();
  if (key.includes("cyberpunk")) {
    return { iconKey: "cyberpunk", iconUrl: "/assets/game-cyberpunk.png" };
  }
  if (key.includes("obsidian")) {
    return { iconKey: "obsidian", iconUrl: "/assets/obsidian.png" };
  }
  if (key.includes("snipping") || key.includes("截图")) {
    return { iconKey: "screenshot", iconUrl: "/assets/screenshot-tool.png" };
  }
  if (key.includes("codex")) return { iconKey: "codex" };
  if (key.includes("code.exe")) return { iconKey: "vscode" };
  if (key.includes("explorer.exe")) return { iconKey: "folder" };
  return { iconKey: "app" };
}

function normalizeApp(app: NativeApp): AppInfo {
  const inferred = inferIcon(app);
  return {
    id: app.id,
    name: app.name,
    path: app.path,
    launchArguments: app.launch_arguments ?? undefined,
    workingDirectory: app.working_directory ?? undefined,
    aliases: app.aliases ?? [],
    category: app.category,
    ...inferred,
    iconUrl:
      app.icon && (app.icon.startsWith("data:") || app.icon.startsWith("/"))
        ? app.icon
        : inferred.iconUrl,
  };
}

function normalizeBinding(binding: NativeBinding): Binding {
  return {
    id: binding.id,
    mainApp: normalizeApp(binding.main_app),
    openApps: (binding.open_apps ?? []).map(normalizeApp),
    closeApps: (binding.close_apps ?? []).map(normalizeApp),
  };
}

function toNativeApp(app: AppInfo): NativeApp {
  return {
    id: app.id,
    name: app.name,
    path: app.path,
    launch_arguments: app.launchArguments,
    working_directory: app.workingDirectory,
    aliases: app.aliases,
    category: app.category,
    icon: app.iconUrl?.startsWith("data:") ? app.iconUrl : undefined,
  };
}

function toNativeBinding(draft: BindingDraft): NativeBinding {
  if (!draft.mainApp) throw new Error("请先选择主应用");
  return {
    id: draft.id ?? "",
    name: draft.mainApp.name,
    main_app: toNativeApp(draft.mainApp),
    open_apps: draft.openApps.map(toNativeApp),
    close_apps: draft.closeApps.map(toNativeApp),
  };
}

function normalizeReport(report: NativeReport): RunResult {
  const steps = report.operations.map((operation) => ({
    appId: operation.app.id,
    appName: operation.app.name,
    action: operation.action,
      status: operation.status,
    message: operation.message ?? undefined,
  }));
  return {
    success: steps.every((step) => step.status !== "failed"),
    message:
      report.mode === "dry_run"
        ? "试运行完成，没有更改真实应用状态。"
        : "RID 已开始执行此 Bind Apps。",
    steps,
    executionId: report.execution_id,
    recoveryPending: report.recovery_pending,
  };
}

async function mockRun(binding: BindingDraft, dryRun: boolean): Promise<RunResult> {
  if (!binding.mainApp) throw new Error("请先选择主应用");
  await new Promise((resolve) => window.setTimeout(resolve, 380));
  return {
    success: true,
    message: dryRun
      ? "浏览器模拟运行完成，没有更改真实应用状态。"
      : "浏览器预览已模拟启动；桌面版会在这里执行真实应用联动。",
    steps: [
      ...binding.closeApps.map((app) => ({
        appId: app.id,
        appName: app.name,
        action: "close",
        status: "success" as const,
      })),
      ...binding.openApps.map((app) => ({
        appId: app.id,
        appName: app.name,
        action: "open",
        status: "success" as const,
      })),
      {
        appId: binding.mainApp.id,
        appName: binding.mainApp.name,
        action: "launch-main",
        status: "success",
      },
    ],
    recoveryPending: !dryRun && binding.closeApps.length > 0,
  };
}

export const ridBridge = {
  isNative: isTauriRuntime,

  async listApps(): Promise<AppInfo[]> {
    if (!isTauriRuntime()) return copy(mockApps);
    const apps = await tauriInvoke<NativeApp[]>("list_installed_apps");
    return apps.map(normalizeApp);
  },

  async listBindings(): Promise<Binding[]> {
    if (!isTauriRuntime()) return readMockBindings();
    const bindings = await tauriInvoke<NativeBinding[]>("list_bindings");
    return bindings.map(normalizeBinding);
  },

  async saveBinding(draft: BindingDraft): Promise<Binding> {
    if (!draft.mainApp) throw new Error("请先选择主应用");
    if (isTauriRuntime()) {
      const binding = await tauriInvoke<NativeBinding>("save_binding", {
        binding: toNativeBinding(draft),
      });
      return normalizeBinding(binding);
    }

    const saved: Binding = {
      id: draft.id ?? `bind-${draft.mainApp.id}-${Date.now()}`,
      mainApp: draft.mainApp,
      openApps: draft.openApps,
      closeApps: draft.closeApps,
    };
    const bindings = readMockBindings();
    const index = bindings.findIndex((binding) => binding.id === saved.id);
    if (index >= 0) bindings[index] = saved;
    else bindings.push(saved);
    writeMockBindings(bindings);
    return copy(saved);
  },

  async deleteBinding(id: string): Promise<void> {
    if (isTauriRuntime()) {
      await tauriInvoke<void>("delete_binding", { id });
      return;
    }
    writeMockBindings(readMockBindings().filter((binding) => binding.id !== id));
  },

  async runBinding(binding: BindingDraft): Promise<RunResult> {
    if (!binding.mainApp) throw new Error("请先选择主应用");
    if (isTauriRuntime()) {
      const report = await tauriInvoke<NativeReport>("dry_run_binding", {
        binding: toNativeBinding(binding),
      });
      return normalizeReport(report);
    }
    return mockRun(binding, true);
  },

  async launchBinding(binding: BindingDraft): Promise<RunResult> {
    if (!binding.mainApp) throw new Error("请先选择主应用");
    if (isTauriRuntime()) {
      const report = await tauriInvoke<NativeReport>("launch_binding", {
        binding: toNativeBinding(binding),
      });
      return normalizeReport(report);
    }
    return mockRun(binding, false);
  },
};
