import { describe, expect, it } from "vitest";

import { mockApps } from "@/lib/mock-data";
import { scoreApp } from "@/lib/fuzzy";

function search(query: string) {
  return mockApps
    .map((app) => ({ app, score: scoreApp(app, query) }))
    .filter(({ score }) => Number.isFinite(score))
    .sort((left, right) => left.score - right.score)
    .map(({ app }) => app.name);
}

describe("应用模糊搜索", () => {
  it.each([
    ["vscd", "Visual Studio Code"],
    ["jt", "截图工具"],
    ["obs", "Obsidian"],
    ["snipping", "截图工具"],
    ["Program Files Codex", "Codex"],
  ])("可用“%s”找到 %s", (query, expected) => {
    expect(search(query)).toContain(expected);
  });

  it("优先返回名称或别名中的连续匹配", () => {
    expect(search("code")[0]).toBe("Codex");
  });

  it("不返回完全不匹配的应用", () => {
    expect(search("zzzzzz")).toEqual([]);
  });
});
