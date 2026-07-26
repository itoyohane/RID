# RID

一次打开需要的应用，临时收起不需要的应用。

RID 是一款 Windows 桌面工具。你可以围绕一个“主应用”创建联动规则：

- 打开主应用时，同时打开其他应用；
- 打开主应用前，临时关闭容易干扰的应用；
- 主应用退出后，只重新打开这一次由 RID 成功关闭的应用。

例如：

- 打开游戏，同时启动语音工具，并临时关闭截图软件；
- 打开 Obsidian，同时启动 Codex；
- 打开工作软件时，一并启动常用的编辑器和文件夹。

![RID 新增选项页面](docs/assets/rid-overview.png)

## 下载与安装

RID 当前支持 64 位 Windows 10 和 Windows 11。

1. 打开 [GitHub Releases](https://github.com/itoyohane/RID/releases)。
2. 进入最新版本，下载 `RID_版本号_x64-setup.exe`。这是推荐给普通用户的安装程序。
3. 双击安装程序，按照提示完成安装。
4. 安装完成后，从 Windows 开始菜单搜索并打开 **RID**。

如果你需要 MSI，可以下载 `RID_版本号_x64_en-US.msi`，它更适合企业部署或统一安装。

> RID 目前是未签名的 MVP。Windows SmartScreen 可能显示“Windows 已保护你的电脑”。请确认安装包来自本仓库的 Releases 页面，再点击“更多信息”→“仍要运行”。如果 Releases 页面暂时没有安装包，请向项目维护者获取，不要从未知来源下载。

卸载 RID：打开 Windows“设置”→“应用”→“已安装的应用”，找到 **RID** 并选择卸载。

## 使用方法

1. 点击侧栏中的“新增应用”。
2. 选择主应用，再添加需要同时打开或临时关闭的应用。
3. 点击“试运行”确认操作，然后保存 Bind Apps。
4. 在保存成功窗口中点击“选择位置并创建”，选择桌面或其他文件夹。
5. 以后直接双击生成的“主应用名 · RID”快捷方式即可。RID 会在后台执行关闭、打开和恢复流程，不需要先打开 RID 主窗口。

快捷方式保存的是 Bind Apps 的 ID，而不是一份独立配置。之后在 RID 中修改同一个模块，原快捷方式会自动执行最新配置。

RID 会自动搜索 Windows 中已安装的应用、桌面快捷方式和开始菜单快捷方式，并显示应用自身的图标。搜索支持应用名称、快捷方式名称和路径片段。

## 安全说明

- RID 默认请求应用正常退出，不会强制结束进程。
- 只恢复启动前正在运行、并且由 RID 成功关闭的应用。
- 浏览器预览只使用演示数据，不会打开或关闭电脑上的真实应用。
- 部分仅驻留系统托盘或以管理员身份运行的应用，可能无法被普通权限的 RID 关闭。

## 当前版本

RID 目前是 Windows MVP，支持：

- 创建、编辑和删除 Bind Apps；
- 搜索注册表、桌面及开始菜单中的本地应用；
- 读取 `.lnk` 快捷方式的目标、启动参数和工作目录；
- 自动读取本地应用图标；
- 保存规则、试运行和正式运行；
- 在用户指定目录生成带主应用图标的启动快捷方式；
- 从快捷方式后台执行 Bind Apps，无需显示 RID 主窗口；
- 主应用退出后的选择性恢复。

## 开发者运行

需要 Node.js、Rust 和 Windows WebView2 开发环境。

```powershell
npm install
npm run tauri:dev
```

生成 Windows 安装包：

```powershell
npm run tauri:build
```

构建完成后可以在以下目录找到：

- `src-tauri/target/release/bundle/nsis/`：推荐给普通用户的 `.exe` 安装程序；
- `src-tauri/target/release/bundle/msi/`：用于企业部署的 `.msi` 安装包。

浏览器界面预览：

```powershell
npm run dev
```

> 浏览器受安全限制，只显示演示应用。检测本机应用请运行 Tauri 桌面版。

产品说明见 [RID PRD](docs/RID/PRD_RID.md)，技术实现见 [架构说明](docs/architecture.md)。
