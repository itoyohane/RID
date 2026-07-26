"use client";

import {
  AppWindow,
  Code,
  FolderOpen,
  RocketLaunch,
} from "@phosphor-icons/react";
import Image from "next/image";
import { useState } from "react";
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
  const [failedIconUrl, setFailedIconUrl] = useState<string | null>(null);
  const Icon = fallbackIcons[app.iconKey ?? "app"];
  const tone =
    app.iconKey === "codex"
      ? "dark"
      : app.iconKey === "folder"
        ? "gold"
        : "blue";

  return (
    <span className={`app-icon app-icon--${size} app-icon--${tone}`}>
      {app.iconUrl && failedIconUrl !== app.iconUrl ? (
        <Image
          src={app.iconUrl}
          alt=""
          width={128}
          height={128}
          unoptimized
          onError={() => setFailedIconUrl(app.iconUrl ?? null)}
        />
      ) : (
        <Icon weight="duotone" aria-hidden />
      )}
    </span>
  );
}
