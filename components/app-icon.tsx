"use client";

import {
  AppWindow,
  Code,
  FolderOpen,
  RocketLaunch,
} from "@phosphor-icons/react";
import type { AppInfo } from "@/lib/types";

const fallbackIcons = {
  codex: Code,
  vscode: Code,
  folder: FolderOpen,
  rocket: RocketLaunch,
  app: AppWindow,
  cyberpunk: AppWindow,
  obsidian: AppWindow,
  accelerator: AppWindow,
  screenshot: AppWindow,
};

export function AppIcon({
  app,
  size = "normal",
}: {
  app: AppInfo;
  size?: "normal" | "small" | "nav";
}) {
  const Icon = fallbackIcons[app.iconKey ?? "app"];
  const tone =
    app.iconKey === "codex"
      ? "dark"
      : app.iconKey === "folder"
        ? "gold"
        : "blue";

  return (
    <span className={`app-icon app-icon--${size} app-icon--${tone}`}>
      {app.iconUrl ? <img src={app.iconUrl} alt="" /> : <Icon weight="duotone" aria-hidden />}
    </span>
  );
}

