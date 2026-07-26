import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { mockApps } from "@/lib/mock-data";
import type { BindingDraft } from "@/lib/types";

const { invoke } = vi.hoisted(() => ({
  invoke: vi.fn(),
}));

vi.mock("@tauri-apps/api/core", () => ({
  invoke,
}));

const emptyDraft: BindingDraft = {
  mainApp: null,
  openApps: [],
  closeApps: [],
};

describe("RID Tauri bridge", () => {
  beforeEach(() => {
    vi.resetModules();
    invoke.mockReset();
  });

  afterEach(() => {
    Reflect.deleteProperty(window, "__TAURI_INTERNALS__");
  });

  it("浏览器环境返回可搜索的 mock 应用", async () => {
    const { ridBridge } = await import("@/lib/tauri");

    await expect(ridBridge.listApps()).resolves.toEqual(mockApps);
    expect(invoke).not.toHaveBeenCalled();
  });

  it("浏览器环境可保存并重新读取 Bind Apps", async () => {
    const { ridBridge } = await import("@/lib/tauri");
    const draft: BindingDraft = {
      mainApp: mockApps[1],
      openApps: [mockApps[2]],
      closeApps: [mockApps[4]],
    };

    const saved = await ridBridge.saveBinding(draft);
    const bindings = await ridBridge.listBindings();

    expect(saved.mainApp.name).toBe("Obsidian");
    expect(bindings).toContainEqual(saved);
  });

  it("主应用缺失时拒绝保存和运行", async () => {
    const { ridBridge } = await import("@/lib/tauri");

    await expect(ridBridge.saveBinding(emptyDraft)).rejects.toThrow("请先选择主应用");
    await expect(ridBridge.runBinding(emptyDraft)).rejects.toThrow("请先选择主应用");
  });

  it("浏览器试运行返回关闭、打开、主应用的有序步骤", async () => {
    vi.useFakeTimers();
    const { ridBridge } = await import("@/lib/tauri");
    const draft: BindingDraft = {
      mainApp: mockApps[1],
      openApps: [mockApps[2]],
      closeApps: [mockApps[4]],
    };

    const promise = ridBridge.runBinding(draft);
    await vi.runAllTimersAsync();
    const result = await promise;
    vi.useRealTimers();

    expect(result.success).toBe(true);
    expect(result.steps.map((step) => step.action)).toEqual([
      "close",
      "open",
      "launch-main",
    ]);
  });

  it("native 环境通过 invoke 调用命令并规范化缺失的 aliases", async () => {
    Object.defineProperty(window, "__TAURI_INTERNALS__", {
      configurable: true,
      value: {},
    });
    invoke.mockResolvedValueOnce([
      {
        id: "native-app",
        name: "Native App",
        path: "C:\\Native.exe",
      },
    ]);
    const { ridBridge } = await import("@/lib/tauri");

    await expect(ridBridge.listApps()).resolves.toEqual([
      expect.objectContaining({
        id: "native-app",
        name: "Native App",
        path: "C:\\Native.exe",
        aliases: [],
        iconKey: "app",
      }),
    ]);
    expect(invoke).toHaveBeenCalledWith("list_installed_apps", undefined);
  });

  it("native 试运行发送 snake_case binding 并转换执行报告", async () => {
    Object.defineProperty(window, "__TAURI_INTERNALS__", {
      configurable: true,
      value: {},
    });
    invoke.mockResolvedValueOnce({
      execution_id: "execution-1",
      mode: "dry_run",
      started_at: "2026-07-26T00:00:00Z",
      operations: [
        {
          app: {
            id: "obsidian",
            name: "Obsidian",
            path: "C:\\Obsidian.exe",
          },
          action: "launch-main",
          status: "success",
        },
      ],
      recovery_pending: false,
    });
    const { ridBridge } = await import("@/lib/tauri");
    const draft: BindingDraft = {
      id: "bind-obsidian",
      mainApp: mockApps[1],
      openApps: [mockApps[2]],
      closeApps: [mockApps[4]],
    };

    const result = await ridBridge.runBinding(draft);

    expect(invoke).toHaveBeenCalledWith(
      "dry_run_binding",
      expect.objectContaining({
        binding: expect.objectContaining({
          id: "bind-obsidian",
          main_app: expect.objectContaining({ id: "obsidian" }),
          open_apps: [expect.objectContaining({ id: "codex" })],
          close_apps: [expect.objectContaining({ id: "screenshot" })],
        }),
      }),
    );
    expect(result).toMatchObject({
      success: true,
      executionId: "execution-1",
      recoveryPending: false,
    });
    expect(result.steps[0].status).toBe("success");
  });

  it("native 正式运行调用 launch_binding 并传输完整 binding", async () => {
    Object.defineProperty(window, "__TAURI_INTERNALS__", {
      configurable: true,
      value: {},
    });
    invoke.mockResolvedValueOnce({
      execution_id: "execution-launch",
      mode: "launch",
      started_at: "2026-07-26T00:00:00Z",
      operations: [],
      recovery_pending: true,
    });
    const { ridBridge } = await import("@/lib/tauri");
    const draft: BindingDraft = {
      id: "bind-obsidian",
      mainApp: mockApps[1],
      openApps: [mockApps[2]],
      closeApps: [mockApps[4]],
    };

    const result = await ridBridge.launchBinding(draft);

    expect(invoke).toHaveBeenCalledWith(
      "launch_binding",
      expect.objectContaining({
        binding: expect.objectContaining({
          main_app: expect.objectContaining({ id: "obsidian" }),
          open_apps: [expect.objectContaining({ id: "codex" })],
          close_apps: [expect.objectContaining({ id: "screenshot" })],
        }),
      }),
    );
    expect(result).toMatchObject({
      success: true,
      executionId: "execution-launch",
      recoveryPending: true,
    });
  });
});
