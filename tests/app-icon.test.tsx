import { fireEvent, render } from "@testing-library/react";
import { describe, expect, it } from "vitest";

import { AppIcon } from "@/components/app-icon";
import type { AppInfo } from "@/lib/types";

const app: AppInfo = {
  id: "broken-icon",
  name: "Broken icon app",
  path: "C:\\Apps\\Broken.exe",
  aliases: [],
  iconKey: "app",
  iconUrl: "data:image/png;base64,broken",
};

describe("AppIcon", () => {
  it("原生图标加载失败时回退为应用图标", () => {
    const { container } = render(<AppIcon app={app} />);
    const image = container.querySelector("img");
    expect(image).not.toBeNull();

    fireEvent.error(image!);

    expect(container.querySelector("img")).toBeNull();
    expect(container.querySelector("svg")).not.toBeNull();
  });
});
