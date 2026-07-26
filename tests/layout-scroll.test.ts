import { readFileSync } from "node:fs";
import { resolve } from "node:path";

import { describe, expect, it } from "vitest";

const css = readFileSync(resolve(process.cwd(), "app/globals.css"), "utf8");

function rule(selector: string) {
  const escaped = selector.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
  return css.match(new RegExp(`${escaped}\\s*\\{([^}]+)\\}`))?.[1] ?? "";
}

describe("桌面布局滚动约束", () => {
  it("主 Grid 和工作区允许子项收缩", () => {
    expect(rule(".app-frame")).toContain("min-height: 0");
    expect(rule(".workspace")).toContain("min-height: 0");
    expect(rule(".workspace")).toContain("overflow: hidden");
  });

  it("内容区和侧栏拥有各自的滚动容器", () => {
    expect(rule(".workspace__scroll")).toContain("min-height: 0");
    expect(rule(".workspace__scroll")).toContain("overflow: auto");
    expect(rule(".scene-nav")).toContain("overflow-y: auto");
  });

  it("桌面外壳不再用固定最小高度裁掉小窗口内容", () => {
    expect(rule(".desktop-shell")).toContain("min-height: 0");
    expect(rule(".desktop-shell")).toContain("height: 100dvh");
  });
});
