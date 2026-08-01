# RID 调试报告：快捷方式创建与启动卡顿

日期：2026-08-01

## 问题与结论

1. **创建快捷方式失败**：快捷方式是在 Tauri/WebView 命令线程中直接初始化 COM 并写入。该线程的 COM apartment 可能已被 WebView 以不同模式初始化，导致创建 Shell Link 时出现不稳定的 Windows COM 失败。超长绑定名称还会让 `.lnk` 的完整路径超过部分 Windows Shell API 的兼容长度。
2. **启动时卡顿**：应用一启动便并行请求已保存绑定和 Windows 应用目录；后者会递归扫描桌面和开始菜单、解析 `.lnk`，并提取图标。在应用数量较多的机器上，这段原生工作会占用命令分发线程，同时界面也等待两个请求都完成才更新。

## 修复

- `src-tauri/src/shortcut.rs`
  - 将 Shell Link 写入移到专用、全新初始化的 STA COM 工作线程，避开 WebView 线程的 COM 模式冲突。
  - 保留明确的 Windows 错误信息，并在工作线程异常退出时提供可诊断错误。
  - 按输出目录可用的 UTF-16 路径长度截断快捷方式文件名；普通名称不变，超长名称不再导致创建失败。
- `src-tauri/src/commands.rs`
  - 将 `list_installed_apps` 改为异步 Tauri 命令，并使用后台阻塞任务执行目录扫描与 COM 图标解析。
- `components/rid-app.tsx`
  - 将“已保存 Bind Apps”和“Windows 应用扫描”拆为两个独立请求。绑定先加载、界面先可用；应用列表继续在后台准备。
- `tests/backend-contract.test.ts`
  - 契约测试同时覆盖同步和异步的 Tauri command 声明，避免把性能修复误判为接口变化。

## 验证

执行：`cargo test --manifest-path src-tauri/Cargo.toml`

结果：20 个 Rust 测试全部通过，其中包含创建、解析和更新 Windows `.lnk` 的回归测试。

前端 Vitest 未执行：当前工作区没有安装 `node_modules`，`vitest` 命令不可用；本次 TypeScript 改动仅拆分现有异步请求，没有改变 bridge 参数或 UI 数据结构。

## 建议的人工验收

1. 启动 RID，确认已保存的 Bind Apps 会先显示，应用列表仍在扫描时界面可操作。
2. 用一个很长的主应用/绑定名称，选择桌面目录创建快捷方式；确认生成并可双击执行。
3. 编辑已有绑定并保存，确认原 `.lnk` 会原地更新。
