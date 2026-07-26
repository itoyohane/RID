import { expect, test, type Locator, type Page } from "@playwright/test";

function ruleSection(page: Page, name: string) {
  return page.locator("section.rule-section").filter({
    has: page.getByRole("heading", { name, exact: true }),
  });
}

async function pickApp(
  page: Page,
  trigger: Locator,
  query: string,
  appName: string,
) {
  await trigger.click();
  const dialog = page.getByRole("dialog", { name: "选择一个应用" });
  await expect(dialog).toBeVisible();
  await dialog.getByPlaceholder("模糊搜索应用名、路径或别名").fill(query);
  await dialog.getByRole("button", { name: new RegExp(appName) }).click();
}

test.beforeEach(async ({ page }) => {
  await page.addInitScript(() => window.localStorage.clear());
});

test("在浏览器 mock 中完成 Bind Apps 新增、试运行与保存", async ({ page }) => {
  await page.goto("/");

  await expect(page.getByRole("heading", { name: "新增选项" })).toBeVisible();
  await expect(page.getByText("Browser preview")).toBeVisible();
  await expect(page.getByRole("button", { name: "试运行" })).toBeDisabled();
  await expect(page.getByRole("button", { name: "保存 Bind Apps" })).toBeDisabled();

  await pickApp(
    page,
    ruleSection(page, "主应用").getByRole("button", { name: "选择应用" }),
    "vscd",
    "Visual Studio Code",
  );
  await pickApp(
    page,
    ruleSection(page, "同时打开应用").getByRole("button", { name: "添加应用" }),
    "code",
    "Codex",
  );
  await pickApp(
    page,
    ruleSection(page, "临时关闭应用").getByRole("button", { name: "添加应用" }),
    "jt",
    "截图工具",
  );

  await expect(ruleSection(page, "主应用").getByText("Visual Studio Code")).toBeVisible();
  await expect(
    ruleSection(page, "同时打开应用").getByText("Codex", { exact: true }),
  ).toBeVisible();
  await expect(
    ruleSection(page, "临时关闭应用").getByText("截图工具", { exact: true }),
  ).toBeVisible();

  await page.getByRole("button", { name: "试运行" }).click();
  const runDialog = page.getByRole("dialog", { name: "Bind Apps 已准备好" });
  await expect(runDialog).toContainText("没有更改真实应用状态");
  await expect(runDialog).toContainText("Visual Studio Code · launch-main");
  await runDialog.getByRole("button", { name: "完成" }).click();

  await page.getByRole("button", { name: "保存 Bind Apps" }).click();
  const savedDialog = page.getByRole("dialog", { name: "Bind Apps 已保存" });
  await expect(savedDialog).toContainText("Visual Studio Code");
  await savedDialog.getByRole("button", { name: "好的" }).click();

  const navigation = page.getByRole("complementary", { name: "RID 导航" });
  await expect(
    navigation.getByRole("button", { name: "Visual Studio Code" }),
  ).toBeVisible();
  await expect(page.getByRole("heading", { name: "Visual Studio Code" })).toBeVisible();
});
