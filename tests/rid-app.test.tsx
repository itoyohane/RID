import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { RidApp } from "@/components/rid-app";
import { mockApps } from "@/lib/mock-data";
import type { Binding, BindingDraft } from "@/lib/types";

const bridge = vi.hoisted(() => ({
  isNative: vi.fn(() => false),
  listApps: vi.fn(),
  listBindings: vi.fn(),
  saveBinding: vi.fn(),
  deleteBinding: vi.fn(),
  selectShortcutDirectory: vi.fn(),
  createBindingShortcut: vi.fn(),
  runBinding: vi.fn(),
  launchBinding: vi.fn(),
}));

vi.mock("@/lib/tauri", () => ({
  ridBridge: bridge,
}));

function section(name: string) {
  const heading = screen.getByRole("heading", { name });
  const element = heading.closest("section");
  if (!element) throw new Error(`找不到 ${name} 分区`);
  return within(element);
}

async function chooseApp(trigger: HTMLElement, query: string, appName: string) {
  const user = userEvent.setup();
  await user.click(trigger);
  const dialog = screen.getByRole("dialog", { name: "选择一个应用" });
  const search = within(dialog).getByPlaceholderText("模糊搜索应用名、路径或别名");
  await user.type(search, query);
  await user.click(within(dialog).getByRole("button", { name: new RegExp(appName) }));
}

describe("RID 新增选项页", () => {
  beforeEach(() => {
    bridge.isNative.mockReturnValue(false);
    bridge.listApps.mockResolvedValue(mockApps);
    bridge.listBindings.mockResolvedValue([]);
    bridge.deleteBinding.mockResolvedValue(undefined);
    bridge.selectShortcutDirectory.mockResolvedValue("C:\\Users\\Demo\\Desktop");
    bridge.createBindingShortcut.mockResolvedValue(
      "C:\\Users\\Demo\\Desktop\\Obsidian · RID.lnk",
    );
    bridge.runBinding.mockResolvedValue({
      success: true,
      message: "试运行完成",
      steps: [],
    });
    bridge.launchBinding.mockResolvedValue({
      success: true,
      message: "RID 已开始执行此 Bind Apps。",
      steps: [],
    });
    bridge.saveBinding.mockImplementation(async (draft: BindingDraft): Promise<Binding> => {
      if (!draft.mainApp) throw new Error("请先选择主应用");
      return {
        id: draft.id ?? `bind-${draft.mainApp.id}`,
        shortcutPath: draft.shortcutPath,
        mainApp: draft.mainApp,
        openApps: draft.openApps,
        closeApps: draft.closeApps,
        forceCloseAppIds: draft.forceCloseAppIds,
      };
    });
  });

  it("默认进入新增页，且侧栏 Logo 下首项为新增应用", async () => {
    render(<RidApp />);

    expect(screen.getByRole("heading", { name: "新增选项" })).toBeInTheDocument();
    const navigation = screen.getByRole("complementary", { name: "RID 导航" });
    const firstButton = within(navigation).getAllByRole("button")[0];
    expect(firstButton).toHaveTextContent("新增应用");
    expect(firstButton).toHaveAttribute("aria-current", "page");
    expect(await screen.findByText("选择主应用")).toBeInTheDocument();
  });

  it("右上角可打开并关闭使用指南", async () => {
    const user = userEvent.setup();
    render(<RidApp />);

    await user.click(screen.getByRole("button", { name: "打开使用指南" }));
    const dialog = screen.getByRole("dialog", { name: "三步创建应用联动" });
    expect(within(dialog).getByText("选择主应用")).toBeInTheDocument();
    expect(within(dialog).getByText("配置打开与临时关闭")).toBeInTheDocument();
    expect(within(dialog).getByText("保存并创建快捷方式")).toBeInTheDocument();

    await user.click(within(dialog).getByRole("button", { name: "开始配置" }));
    expect(screen.queryByRole("dialog", { name: "三步创建应用联动" })).not.toBeInTheDocument();
  });

  it("未选择主应用时禁用试运行与保存", async () => {
    render(<RidApp />);
    await screen.findByText("选择主应用");

    expect(screen.getByRole("button", { name: "试运行" })).toBeDisabled();
    expect(screen.getByRole("button", { name: "保存 Bind Apps" })).toBeDisabled();
  });

  it("可用非连续字符模糊搜索并选择主应用", async () => {
    render(<RidApp />);
    await screen.findByText("选择主应用");

    await chooseApp(
      section("主应用").getByRole("button", { name: "选择应用" }),
      "vscd",
      "Visual Studio Code",
    );

    expect(section("主应用").getByText("Visual Studio Code")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "试运行" })).toBeEnabled();
    expect(screen.getByRole("button", { name: "保存 Bind Apps" })).toBeEnabled();
  });

  it("可配置打开与关闭应用，保存后侧栏按主应用名称出现", async () => {
    const user = userEvent.setup();
    render(<RidApp />);
    await screen.findByText("选择主应用");

    await chooseApp(
      section("主应用").getByRole("button", { name: "选择应用" }),
      "obs",
      "Obsidian",
    );
    await chooseApp(
      section("同时打开应用").getByRole("button", { name: "添加应用" }),
      "code",
      "Codex",
    );
    await chooseApp(
      section("临时关闭应用").getByRole("button", { name: "添加应用" }),
      "jietu",
      "截图工具",
    );

    expect(section("同时打开应用").getByText("Codex")).toBeInTheDocument();
    expect(section("同时打开应用").getByText("将打开")).toBeInTheDocument();
    expect(section("临时关闭应用").getByText("截图工具")).toBeInTheDocument();
    expect(section("临时关闭应用").getByText("将临时关闭")).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "截图工具 更多操作" }));
    const forceOption = screen.getByRole("button", { name: /关闭失败时强制结束/ });
    expect(forceOption).toHaveAttribute("aria-pressed", "false");
    await user.click(forceOption);

    await user.click(screen.getByRole("button", { name: "保存 Bind Apps" }));

    await waitFor(() =>
      expect(bridge.saveBinding).toHaveBeenCalledWith(
        expect.objectContaining({
          mainApp: expect.objectContaining({ id: "obsidian" }),
          openApps: [expect.objectContaining({ id: "codex" })],
          closeApps: [expect.objectContaining({ id: "screenshot" })],
          forceCloseAppIds: ["screenshot"],
        }),
      ),
    );
    expect(await screen.findByRole("dialog", { name: "Bind Apps 已保存" })).toBeInTheDocument();
    const navigation = screen.getByRole("complementary", { name: "RID 导航" });
    expect(within(navigation).getByRole("button", { name: "Obsidian" })).toBeInTheDocument();
  });

  it("保存后可选择位置并创建启动快捷方式", async () => {
    const user = userEvent.setup();
    render(<RidApp />);
    await screen.findByText("选择主应用");

    await chooseApp(
      section("主应用").getByRole("button", { name: "选择应用" }),
      "obs",
      "Obsidian",
    );
    await user.click(screen.getByRole("button", { name: "保存 Bind Apps" }));
    const dialog = await screen.findByRole("dialog", { name: "Bind Apps 已保存" });
    await user.click(within(dialog).getByRole("button", { name: "选择位置并创建" }));

    await waitFor(() => {
      expect(bridge.selectShortcutDirectory).toHaveBeenCalledTimes(1);
      expect(bridge.createBindingShortcut).toHaveBeenCalledWith(
        expect.objectContaining({ id: "bind-obsidian" }),
        "C:\\Users\\Demo\\Desktop",
      );
    });
    expect(
      await screen.findByRole("dialog", { name: "快捷方式已创建" }),
    ).toHaveTextContent("C:\\Users\\Demo\\Desktop\\Obsidian · RID.lnk");
  });

  it("试运行通过 bridge 返回明确结果", async () => {
    const user = userEvent.setup();
    render(<RidApp />);
    await screen.findByText("选择主应用");
    await chooseApp(
      section("主应用").getByRole("button", { name: "选择应用" }),
      "obs",
      "Obsidian",
    );

    await user.click(screen.getByRole("button", { name: "试运行" }));

    expect(bridge.runBinding).toHaveBeenCalledWith(
      expect.objectContaining({
        mainApp: expect.objectContaining({ id: "obsidian" }),
      }),
    );
    expect(await screen.findByRole("dialog", { name: "Bind Apps 已准备好" })).toBeInTheDocument();
  });

  it("已保存模块可通过 bridge 执行真实运行", async () => {
    const user = userEvent.setup();
    const saved: Binding = {
      id: "bind-obsidian",
      mainApp: mockApps[1],
      openApps: [mockApps[2]],
      closeApps: [mockApps[4]],
      forceCloseAppIds: [],
    };
    bridge.listBindings.mockResolvedValue([saved]);
    render(<RidApp />);

    const navigation = screen.getByRole("complementary", { name: "RID 导航" });
    await user.click(await within(navigation).findByRole("button", { name: "Obsidian" }));
    await user.click(screen.getByRole("button", { name: "运行 Bind Apps" }));

    expect(bridge.launchBinding).toHaveBeenCalledWith({
      id: "bind-obsidian",
      mainApp: expect.objectContaining({ id: "obsidian" }),
      openApps: [expect.objectContaining({ id: "codex" })],
      closeApps: [expect.objectContaining({ id: "screenshot" })],
      forceCloseAppIds: [],
    });
  });

  it("编辑已有模块后直接更新原快捷方式", async () => {
    const user = userEvent.setup();
    const saved: Binding = {
      id: "bind-obsidian",
      shortcutPath: "C:\\Users\\Demo\\Desktop\\Obsidian 路 RID.lnk",
      mainApp: mockApps[1],
      openApps: [mockApps[2]],
      closeApps: [],
      forceCloseAppIds: [],
    };
    bridge.listBindings.mockResolvedValue([saved]);
    render(<RidApp />);

    const navigation = screen.getByRole("complementary", { name: "RID 导航" });
    await user.click(await within(navigation).findByRole("button", { name: "Obsidian" }));
    await user.click(screen.getByRole("button", { name: "保存更改" }));

    await waitFor(() =>
      expect(bridge.saveBinding).toHaveBeenCalledWith(
        expect.objectContaining({
          id: "bind-obsidian",
          shortcutPath: saved.shortcutPath,
        }),
      ),
    );
    expect(screen.queryByRole("dialog", { name: "Bind Apps 已保存" })).not.toBeInTheDocument();
    expect(screen.getByRole("status")).toHaveTextContent("原快捷方式已更新");
    expect(bridge.selectShortcutDirectory).not.toHaveBeenCalled();
  });
});
