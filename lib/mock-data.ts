import type { AppInfo, Binding } from "@/lib/types";

export const mockApps: AppInfo[] = [
  {
    id: "cyberpunk",
    name: "Cyberpunk 2077",
    path: "D:\\Games\\Cyberpunk 2077\\bin\\x64\\Cyberpunk2077.exe",
    aliases: ["cyberpunk", "2077", "game", "游戏"],
    iconKey: "cyberpunk",
    iconUrl: "/assets/game-cyberpunk.png",
  },
  {
    id: "obsidian",
    name: "Obsidian",
    path: "C:\\Users\\User\\AppData\\Local\\Obsidian\\Obsidian.exe",
    aliases: ["obs", "notes", "笔记", "知识库"],
    iconKey: "obsidian",
    iconUrl: "/assets/obsidian.png",
  },
  {
    id: "codex",
    name: "Codex",
    path: "C:\\Program Files\\Codex\\Codex.exe",
    aliases: ["code", "ai", "coding", "编程"],
    iconKey: "codex",
  },
  {
    id: "accelerator",
    name: "加速器",
    path: "C:\\Program Files\\NetBooster\\NetBooster.exe",
    aliases: ["jiasuqi", "netbooster", "network", "网络"],
    iconKey: "accelerator",
    iconUrl: "/assets/accelerator.png",
  },
  {
    id: "screenshot",
    name: "截图工具",
    path: "C:\\Windows\\System32\\SnippingTool.exe",
    aliases: ["jietu", "snipping", "capture", "screen", "截图"],
    iconKey: "screenshot",
    iconUrl: "/assets/screenshot-tool.png",
  },
  {
    id: "launcher",
    name: "应用启动器",
    path: "C:\\Program Files\\AppLauncher\\Launcher.exe",
    aliases: ["launcher", "qidongqi", "启动器"],
    iconKey: "rocket",
  },
  {
    id: "files",
    name: "文件资源管理器",
    path: "C:\\Windows\\explorer.exe",
    aliases: ["explorer", "folder", "file", "wenjian", "文件"],
    iconKey: "folder",
  },
  {
    id: "vscode",
    name: "Visual Studio Code",
    path: "C:\\Users\\User\\AppData\\Local\\Programs\\Microsoft VS Code\\Code.exe",
    aliases: ["vscode", "vs code", "editor", "编辑器"],
    iconKey: "vscode",
  },
];

export const mockBindings: Binding[] = [
  {
    id: "bind-cyberpunk",
    mainApp: mockApps[0],
    openApps: [mockApps[3]],
    closeApps: [mockApps[4], mockApps[1]],
  },
  {
    id: "bind-obsidian",
    mainApp: mockApps[1],
    openApps: [mockApps[2]],
    closeApps: [mockApps[4]],
  },
];

