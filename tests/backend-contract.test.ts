import { readFileSync } from "node:fs";
import { resolve } from "node:path";

import { describe, expect, it } from "vitest";

const read = (path: string) => readFileSync(resolve(process.cwd(), path), "utf8");

const commandsSource = read("src-tauri/src/commands.rs");
const handlerSource = read("src-tauri/src/lib.rs");
const modelsSource = read("src-tauri/src/models.rs");
const platformSource = read("src-tauri/src/platform.rs");
const runtimeSource = read("src-tauri/src/runtime.rs");
const shortcutSource = read("src-tauri/src/shortcut.rs");
const iconSource = read("src-tauri/src/icon.rs");
const bridgeSource = read("lib/tauri.ts");

const commandNames = [
  "list_installed_apps",
  "list_bindings",
  "save_binding",
  "delete_binding",
  "create_binding_shortcut",
  "dry_run_binding",
  "launch_binding",
] as const;

function commandDeclaration(name: string) {
  const start = commandsSource.indexOf(`pub fn ${name}`);
  if (start < 0) return "";
  const next = commandsSource.indexOf("#[tauri::command]", start);
  return commandsSource.slice(start, next < 0 ? undefined : next);
}

describe("Tauri 后端命令契约", () => {
  it.each(commandNames)("%s 是已注册的 Tauri command", (name) => {
    expect(commandsSource).toMatch(
      new RegExp(`#\\[tauri::command\\]\\s*pub fn ${name}\\s*\\(`),
    );
    expect(handlerSource).toContain(`commands::${name}`);
  });

  it("保存与删除命令接收前端约定参数", () => {
    expect(commandDeclaration("save_binding")).toMatch(/\bbinding:\s*Binding\b/);
    expect(commandDeclaration("delete_binding")).toMatch(/\bid:\s*String\b/);
  });

  it("快捷方式命令通过 binding id 和用户目录生成启动入口", () => {
    const declaration = commandDeclaration("create_binding_shortcut");
    expect(declaration).toMatch(/\bid:\s*String\b/);
    expect(declaration).toMatch(/\bdirectory:\s*String\b/);
    expect(shortcutSource).toContain("--run-binding");
    expect(shortcutSource).toContain("SetArguments");
    expect(handlerSource).toContain("binding_id_from_args");
    expect(handlerSource).toContain("run_saved_binding");
  });

  it.each(["dry_run_binding", "launch_binding"])(
    "%s 可接收 id 或完整 binding",
    (name) => {
      const declaration = commandDeclaration(name);
      expect(declaration).toMatch(/\bid:\s*Option<String>/);
      expect(declaration).toMatch(/\bbinding:\s*Option<Binding>/);
      expect(declaration).toContain("resolve_binding");
    },
  );

  it("Binding DTO 使用 bridge 约定的 snake_case 字段", () => {
    expect(modelsSource).toMatch(/pub struct Binding\s*\{[\s\S]*?\bshortcut_path:\s*Option<String>/);
    expect(modelsSource).toMatch(/pub struct Binding\s*\{[\s\S]*?\bmain_app:\s*AppDescriptor/);
    expect(modelsSource).toMatch(/pub struct Binding\s*\{[\s\S]*?\bopen_apps:\s*Vec<AppDescriptor>/);
    expect(modelsSource).toMatch(/pub struct Binding\s*\{[\s\S]*?\bclose_apps:\s*Vec<AppDescriptor>/);
    expect(modelsSource).toMatch(
      /pub struct Binding\s*\{[\s\S]*?\bforce_close_app_ids:\s*Vec<String>/,
    );
  });

  it("应用 DTO 保留快捷方式参数、工作目录和本地图标", () => {
    expect(modelsSource).toMatch(/\blaunch_arguments:\s*Option<String>/);
    expect(modelsSource).toMatch(/\bworking_directory:\s*Option<String>/);
    expect(shortcutSource).toContain("IShellLinkW");
    expect(shortcutSource).toContain("Start Menu");
    expect(shortcutSource).toContain("Desktop");
    expect(platformSource).toContain("discover_shortcuts");
    expect(iconSource).toContain("data:image/png;base64,");
  });

  it("快捷方式运行完成后保留 RID，并在保存时原位更新快捷方式", () => {
    expect(commandsSource).toContain("reveal_when_done");
    expect(commandsSource).toContain('get_webview_window("main")');
    expect(commandsSource).not.toContain("background_app.exit(0)");
    expect(commandsSource).toContain("replace_binding_shortcut");
    expect(shortcutSource).toContain("find_binding_shortcut");
  });

  it("应用图标优先使用 Windows 高分辨率原生提取", () => {
    expect(iconSource).toContain("SHDefExtractIconW");
    expect(iconSource).toContain("const ICON_SIZE: i32 = 128");
    expect(iconSource).toContain("const NATIVE_ICON_SIZE: u32 = 256");
  });

  it("执行报告公开恢复状态与 snake_case 枚举", () => {
    expect(modelsSource).toMatch(
      /#\[serde\(rename_all = "snake_case"\)\]\s*pub enum OperationStatus/,
    );
    expect(modelsSource).toMatch(
      /pub struct ExecutionReport\s*\{[\s\S]*?\brecovery_pending:\s*bool/,
    );
  });

  it("Windows 启动、托盘关闭和失败反馈使用安全的分层机制", () => {
    expect(runtimeSource).toContain("ShellExecuteExW");
    expect(runtimeSource).toContain("SEE_MASK_NOCLOSEPROCESS");
    expect(runtimeSource).toContain("WM_CLOSE");
    expect(runtimeSource).toContain("force_close");
    expect(runtimeSource).toContain("PROCESS_TERMINATE");
    expect(handlerSource).toContain("execution_failure_summary");
    expect(commandsSource).toContain("write_execution_report");
  });

  it.each(commandNames)("前端 bridge 与后端命令 %s 保持同名", (name) => {
    expect(bridgeSource).toContain(`"${name}"`);
  });
});
