import { describe, expect, it } from "vitest";

import { detectWindowsLocale, resolveLocale } from "@/lib/i18n";

describe("RID locale selection", () => {
  it.each(["zh-CN", "zh_CN", "zh-Hans-CN"])(
    "uses Simplified Chinese for mainland China locale %s",
    (language) => {
      expect(detectWindowsLocale([language, "en-US"])).toBe("zh-CN");
    },
  );

  it.each(["en-US", "zh-HK", "zh-TW", "ja-JP", "de-DE"])(
    "uses English outside mainland China for %s",
    (language) => {
      expect(detectWindowsLocale([language])).toBe("en");
    },
  );

  it("uses only the first Windows preferred language", () => {
    expect(detectWindowsLocale(["en-US", "zh-CN"])).toBe("en");
  });

  it("keeps an explicit user choice", () => {
    expect(resolveLocale("zh-CN", ["en-US"])).toBe("zh-CN");
    expect(resolveLocale("en", ["zh-CN"])).toBe("en");
  });
});
