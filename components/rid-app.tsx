"use client";

import {
  BookOpenText,
  Check,
  CheckCircle,
  DotsThree,
  List,
  MagnifyingGlass,
  Play,
  Plus,
  ShieldCheck,
  Trash,
  Translate,
  WarningCircle,
  X,
} from "@phosphor-icons/react";
import {
  createContext,
  type MouseEvent,
  useContext,
  useEffect,
  useMemo,
  useState,
  useSyncExternalStore,
} from "react";
import { AppIcon } from "@/components/app-icon";
import { RidMark } from "@/components/rid-mark";
import { scoreApp } from "@/lib/fuzzy";
import {
  getLocaleSnapshot,
  getServerLocaleSnapshot,
  setLocalePreference,
  subscribeLocale,
  type Locale,
} from "@/lib/i18n";
import { ridBridge } from "@/lib/tauri";
import type {
  AppInfo,
  Binding,
  BindingDraft,
  PickerGroup,
  RunResult,
} from "@/lib/types";

const LocaleContext = createContext<Locale>("en");

function translate(locale: Locale, chinese: string, english: string) {
  return locale === "zh-CN" ? chinese : english;
}

function useText() {
  const locale = useContext(LocaleContext);
  return (chinese: string, english: string) =>
    translate(locale, chinese, english);
}

const createEmptyDraft = (): BindingDraft => ({
  mainApp: null,
  openApps: [],
  closeApps: [],
  forceCloseAppIds: [],
});

function errorMessage(error: unknown, fallback: string) {
  if (error instanceof Error && error.message.trim()) return error.message;
  if (typeof error === "string" && error.trim()) return error;
  if (
    error &&
    typeof error === "object" &&
    "message" in error &&
    typeof error.message === "string" &&
    error.message.trim()
  ) {
    return error.message;
  }
  return fallback;
}

function AppRow({
  app,
  status,
  tone = "green",
  onRemove,
  removeLabel,
  forceClose,
  onToggleForceClose,
}: {
  app: AppInfo;
  status: string;
  tone?: "green" | "blue";
  onRemove: () => void;
  removeLabel: string;
  forceClose?: boolean;
  onToggleForceClose?: () => void;
}) {
  const [menuOpen, setMenuOpen] = useState(false);
  const [menuOpensUp, setMenuOpensUp] = useState(false);
  const text = useText();

  function toggleMenu(event: MouseEvent<HTMLButtonElement>) {
    if (!menuOpen) {
      const button = event.currentTarget.getBoundingClientRect();
      setMenuOpensUp(window.innerHeight - button.bottom < 210);
    }
    setMenuOpen((value) => !value);
  }

  return (
    <div className="app-row">
      <AppIcon app={app} />
      <div className="app-row__copy">
        <strong>{app.name}</strong>
        <span>{app.path}</span>
      </div>
      <span className={`status-chip status-chip--${tone}`}>{status}</span>
      <div className="row-menu">
        <button
          className="icon-button icon-button--quiet"
          type="button"
          aria-label={`${app.name} ${text("更多操作", "more actions")}`}
          aria-expanded={menuOpen}
          onClick={toggleMenu}
        >
          <DotsThree weight="bold" aria-hidden />
        </button>
        {menuOpen && (
          <div
            className={`row-menu__popover${menuOpensUp ? " row-menu__popover--up" : ""}`}
          >
            {onToggleForceClose && (
              <button
                className="force-close-option"
                type="button"
                aria-pressed={forceClose}
                onClick={() => {
                  onToggleForceClose();
                  setMenuOpen(false);
                }}
              >
                <span>{forceClose ? "✓" : ""}</span>
                <span>
                  {text(
                    "关闭失败时强制结束",
                    "Force close if graceful close fails",
                  )}
                  <small>
                    {text(
                      "可能导致未保存内容丢失",
                      "May discard unsaved work",
                    )}
                  </small>
                </span>
              </button>
            )}
            <button type="button" onClick={onRemove}>
              {removeLabel}
            </button>
          </div>
        )}
      </div>
    </div>
  );
}

function AddButton({
  label,
  onClick,
}: {
  label?: string;
  onClick: () => void;
}) {
  const text = useText();
  return (
    <button className="add-app-button" type="button" onClick={onClick}>
      <Plus aria-hidden />
      {label ?? text("添加应用", "Add app")}
    </button>
  );
}

function AppPicker({
  group,
  apps,
  onClose,
  onAdd,
}: {
  group: PickerGroup;
  apps: AppInfo[];
  onClose: () => void;
  onAdd: (app: AppInfo) => void;
}) {
  const [query, setQuery] = useState("");
  const text = useText();
  const labels: Record<PickerGroup, string> = {
    mainApp: text("主应用", "Main app"),
    openApps: text("同时打开", "Open together"),
    closeApps: text("临时关闭", "Temporarily close"),
  };
  const filteredApps = useMemo(
    () =>
      apps
        .map((app) => ({ app, score: scoreApp(app, query) }))
        .filter((item) => Number.isFinite(item.score))
        .sort((left, right) => left.score - right.score)
        .map((item) => item.app),
    [apps, query],
  );

  return (
    <div className="modal-backdrop" role="presentation" onMouseDown={onClose}>
      <section
        className="modal"
        role="dialog"
        aria-modal="true"
        aria-labelledby="picker-title"
        onMouseDown={(event) => event.stopPropagation()}
      >
        <div className="modal__header">
          <div>
            <span className="eyebrow">{labels[group]}</span>
            <h2 id="picker-title">
              {text("选择一个应用", "Choose an application")}
            </h2>
          </div>
          <button
            className="icon-button"
            type="button"
            aria-label={text("关闭", "Close")}
            onClick={onClose}
          >
            <X aria-hidden />
          </button>
        </div>
        <label className="search-field">
          <MagnifyingGlass aria-hidden />
          <input
            value={query}
            onChange={(event) => setQuery(event.target.value)}
            placeholder={text(
              "模糊搜索应用名、路径或别名",
              "Search by app name, path, or alias",
            )}
            autoFocus
          />
        </label>
        <p className="search-hint">
          {text(
            "例如输入 “obs”、“vscd” 或 “jietu”",
            'Try "obs", "vscd", or part of a path',
          )}
        </p>
        <div className="picker-list">
          {filteredApps.map((app) => (
            <button className="picker-row" type="button" key={app.id} onClick={() => onAdd(app)}>
              <AppIcon app={app} size="small" />
              <span>
                <strong>{app.name}</strong>
                <small>{app.path}</small>
              </span>
              <Plus aria-hidden />
            </button>
          ))}
          {filteredApps.length === 0 && (
            <div className="empty-state">
              <strong>
                {text("没有找到匹配的应用", "No matching applications")}
              </strong>
              <span>
                {text(
                  "换一个简称、路径片段或拼音试试。",
                  "Try another alias or part of the application path.",
                )}
              </span>
            </div>
          )}
        </div>
      </section>
    </div>
  );
}

function GuideDialog({ onClose }: { onClose: () => void }) {
  const text = useText();
  useEffect(() => {
    const closeOnEscape = (event: KeyboardEvent) => {
      if (event.key === "Escape") onClose();
    };
    window.addEventListener("keydown", closeOnEscape);
    return () => window.removeEventListener("keydown", closeOnEscape);
  }, [onClose]);

  return (
    <div className="modal-backdrop" role="presentation" onMouseDown={onClose}>
      <section
        className="modal guide-modal"
        role="dialog"
        aria-modal="true"
        aria-labelledby="guide-title"
        onMouseDown={(event) => event.stopPropagation()}
      >
        <div className="modal__header">
          <div className="guide-heading">
            <RidMark size="large" />
            <div>
              <span className="eyebrow">
                {text("使用指南", "Quick guide")}
              </span>
              <h2 id="guide-title">
                {text(
                  "三步创建应用联动",
                  "Create an app binding in three steps",
                )}
              </h2>
            </div>
          </div>
          <button
            className="icon-button"
            type="button"
            aria-label={text("关闭使用指南", "Close guide")}
            onClick={onClose}
            autoFocus
          >
            <X aria-hidden />
          </button>
        </div>
        <ol className="guide-steps">
          <li>
            <span>1</span>
            <div>
              <strong>{text("选择主应用", "Choose a main app")}</strong>
              <p>
                {text(
                  "它是这条 Bind Apps 的启动入口，也会成为侧栏中的模块名称。",
                  "It becomes the shortcut entry point and the binding name in the sidebar.",
                )}
              </p>
            </div>
          </li>
          <li>
            <span>2</span>
            <div>
              <strong>
                {text(
                  "配置打开与临时关闭",
                  "Choose apps to open or close",
                )}
              </strong>
              <p>
                {text(
                  "添加需要一同启动的应用，以及运行期间需要暂时收起的应用。",
                  "Add companion apps and apps that should stay closed while the main app runs.",
                )}
              </p>
            </div>
          </li>
          <li>
            <span>3</span>
            <div>
              <strong>
                {text(
                  "保存并创建快捷方式",
                  "Save and create the shortcut",
                )}
              </strong>
              <p>
                {text(
                  "以后双击快捷方式即可执行；再次保存修改会自动更新原快捷方式。",
                  "Use the shortcut from then on. Future changes update it in place.",
                )}
              </p>
            </div>
          </li>
        </ol>
        <div className="guide-note">
          <ShieldCheck weight="duotone" aria-hidden />
          <span>
            {text(
              "RID 只恢复本次由它成功关闭的应用，强制结束始终需要单独开启。",
              "RID restores only apps it successfully closed. Force close always requires explicit opt-in.",
            )}
          </span>
        </div>
        <button className="primary-button primary-button--wide" type="button" onClick={onClose}>
          {text("开始配置", "Start configuring")}
        </button>
      </section>
    </div>
  );
}

function ResultDialog({
  report,
  onClose,
}: {
  report: RunResult;
  onClose: () => void;
}) {
  const text = useText();
  const localizedReportMessage = (() => {
    const messages: Record<string, string> = {
      "试运行完成，没有更改真实应用状态。":
        "Dry run completed without changing application state.",
      "RID 已开始执行此 Bind Apps。": "RID has started this binding.",
      "浏览器模拟运行完成，没有更改真实应用状态。":
        "Browser dry run completed without changing application state.",
      "浏览器预览已模拟启动；桌面版会在这里执行真实应用联动。":
        "The browser preview simulated the launch. The desktop app performs the real actions.",
    };
    return text(report.message, messages[report.message] ?? report.message);
  })();
  return (
    <div className="modal-backdrop" role="presentation" onMouseDown={onClose}>
      <section
        className="modal modal--run"
        role="dialog"
        aria-modal="true"
        aria-labelledby="run-title"
        onMouseDown={(event) => event.stopPropagation()}
      >
        <div className={`success-mark${report.success ? "" : " success-mark--error"}`}>
          {report.success ? <Check weight="bold" aria-hidden /> : <WarningCircle aria-hidden />}
        </div>
        <span className="eyebrow">
          {text("运行结果", "Run result")}
        </span>
        <h2 id="run-title">
          {report.success
            ? text("Bind Apps 已准备好", "Binding is ready")
            : text("发现需要处理的问题", "Some actions need attention")}
        </h2>
        <p>{localizedReportMessage}</p>
        <div className="run-results">
          {report.steps.map((step, index) => (
            <span key={`${step.appId}-${step.action}-${index}`}>
              {step.status === "failed" ? (
                <WarningCircle weight="fill" aria-hidden />
              ) : (
                <CheckCircle weight="fill" aria-hidden />
              )}
              <span>
                {step.appName} · {step.action}
                {step.message && <small>{step.message}</small>}
              </span>
            </span>
          ))}
        </div>
        <button className="primary-button primary-button--wide" type="button" onClick={onClose}>
          {text("完成", "Done")}
        </button>
      </section>
    </div>
  );
}

export function RidApp() {
  const locale = useSyncExternalStore(
    subscribeLocale,
    getLocaleSnapshot,
    getServerLocaleSnapshot,
  );
  const [apps, setApps] = useState<AppInfo[]>([]);
  const [bindings, setBindings] = useState<Binding[]>([]);
  const [activeId, setActiveId] = useState("new");
  const [draft, setDraft] = useState<BindingDraft>(createEmptyDraft);
  const [pickerGroup, setPickerGroup] = useState<PickerGroup | null>(null);
  const [sidebarOpen, setSidebarOpen] = useState(true);
  const [loading, setLoading] = useState(true);
  const [working, setWorking] = useState(false);
  const [report, setReport] = useState<RunResult | null>(null);
  const [savedBinding, setSavedBinding] = useState<Binding | null>(null);
  const [shortcutPath, setShortcutPath] = useState<string | null>(null);
  const [shortcutWorking, setShortcutWorking] = useState(false);
  const [toast, setToast] = useState("");
  const [guideOpen, setGuideOpen] = useState(false);

  const isNew = activeId === "new";
  const text = (chinese: string, english: string) =>
    translate(locale, chinese, english);

  useEffect(() => {
    document.documentElement.lang = locale === "zh-CN" ? "zh-CN" : "en";
  }, [locale]);

  useEffect(() => {
    if (!ridBridge.isNative()) return;

    function blockDesktopRefresh(event: KeyboardEvent) {
      const isRefreshShortcut =
        event.key === "F5" ||
        (event.ctrlKey && event.key.toLowerCase() === "r");
      if (!isRefreshShortcut) return;

      event.preventDefault();
      event.stopPropagation();
    }

    window.addEventListener("keydown", blockDesktopRefresh, true);
    return () =>
      window.removeEventListener("keydown", blockDesktopRefresh, true);
  }, []);

  useEffect(() => {
    let cancelled = false;
    // Saved bindings are a tiny local file. Load them independently so the main
    // workspace becomes usable while Windows application discovery is still running.
    ridBridge.listBindings()
      .then((nextBindings) => {
        if (cancelled) return;
        setBindings(nextBindings);
      })
      .catch((error: unknown) => {
        if (!cancelled) {
          showToast(
            errorMessage(
              error,
              translate(
                getLocaleSnapshot(),
                "载入失败",
                "Failed to load application data",
              ),
            ),
          );
        }
      })
      .finally(() => {
        if (!cancelled) setLoading(false);
      });
    return () => {
      cancelled = true;
    };
  }, []);

  useEffect(() => {
    let cancelled = false;
    ridBridge.listApps()
      .then((nextApps) => {
        if (!cancelled) setApps(nextApps);
      })
      .catch((error: unknown) => {
        if (!cancelled) {
          showToast(
            errorMessage(
              error,
              translate(
                getLocaleSnapshot(),
                "加载应用列表失败",
                "Failed to load the application list",
              ),
            ),
          );
        }
      })
      .finally(() => {
        if (!cancelled) setLoading(false);
      });
    return () => {
      cancelled = true;
    };
  }, []);

  function showToast(message: string) {
    setToast(message);
    window.setTimeout(() => setToast(""), 2400);
  }

  function changeLocale(nextLocale: Locale) {
    setLocalePreference(nextLocale);
  }

  function selectNew() {
    setActiveId("new");
    setDraft(createEmptyDraft());
    setSavedBinding(null);
    setShortcutPath(null);
  }

  function selectBinding(binding: Binding) {
    setActiveId(binding.id);
    setDraft({
      id: binding.id,
      shortcutPath: binding.shortcutPath,
      mainApp: binding.mainApp,
      openApps: [...binding.openApps],
      closeApps: [...binding.closeApps],
      forceCloseAppIds: [...binding.forceCloseAppIds],
    });
    setSavedBinding(null);
    setShortcutPath(null);
  }

  function removeApp(group: PickerGroup, appId: string) {
    setDraft((current) => {
      if (group === "mainApp") return { ...current, mainApp: null };
      return {
        ...current,
        [group]: current[group].filter((app) => app.id !== appId),
        forceCloseAppIds:
          group === "closeApps"
            ? current.forceCloseAppIds.filter((id) => id !== appId)
            : current.forceCloseAppIds,
      };
    });
  }

  function toggleForceClose(appId: string) {
    setDraft((current) => ({
      ...current,
      forceCloseAppIds: current.forceCloseAppIds.includes(appId)
        ? current.forceCloseAppIds.filter((id) => id !== appId)
        : [...current.forceCloseAppIds, appId],
    }));
  }

  function addApp(app: AppInfo) {
    if (!pickerGroup) return;
    setDraft((current) => {
      if (pickerGroup === "mainApp") {
        return {
          ...current,
          mainApp: app,
          openApps: current.openApps.filter((item) => item.id !== app.id),
          closeApps: current.closeApps.filter((item) => item.id !== app.id),
        };
      }
      const otherGroup = pickerGroup === "openApps" ? "closeApps" : "openApps";
      if (current[pickerGroup].some((item) => item.id === app.id)) return current;
      return {
        ...current,
        [pickerGroup]: [...current[pickerGroup], app],
        [otherGroup]: current[otherGroup].filter((item) => item.id !== app.id),
        forceCloseAppIds:
          pickerGroup === "openApps"
            ? current.forceCloseAppIds.filter((id) => id !== app.id)
            : current.forceCloseAppIds,
      };
    });
    setPickerGroup(null);
    showToast(text("应用已加入当前 Bind Apps", "Application added to this binding"));
  }

  async function saveBinding() {
    if (!draft.mainApp) return;
    setWorking(true);
    try {
      const saved = await ridBridge.saveBinding(draft);
      setBindings((current) => {
        const exists = current.some((binding) => binding.id === saved.id);
        return exists
          ? current.map((binding) => (binding.id === saved.id ? saved : binding))
          : [...current, saved];
      });
      setDraft(saved);
      setActiveId(saved.id);
      setShortcutPath(saved.shortcutPath ?? null);
      if (!isNew && saved.shortcutPath) {
        setSavedBinding(null);
        showToast(
          text(
            "更改已保存，原快捷方式已更新",
            "Changes saved and the shortcut was updated",
          ),
        );
      } else {
        setSavedBinding(saved);
      }
    } catch (error) {
      showToast(errorMessage(error, text("保存失败", "Failed to save")));
    } finally {
      setWorking(false);
    }
  }

  async function createShortcut(binding: Binding) {
    setShortcutWorking(true);
    try {
      const directory = await ridBridge.selectShortcutDirectory(locale);
      if (!directory) return;
      const path = await ridBridge.createBindingShortcut(binding, directory);
      const updated = { ...binding, shortcutPath: path };
      setSavedBinding(updated);
      setDraft((current) => ({ ...current, shortcutPath: path }));
      setBindings((current) =>
        current.map((item) => (item.id === binding.id ? updated : item)),
      );
      setShortcutPath(path);
      showToast(text("快捷方式已创建", "Shortcut created"));
    } catch (error) {
      showToast(
        errorMessage(
          error,
          text("创建快捷方式失败", "Failed to create the shortcut"),
        ),
      );
    } finally {
      setShortcutWorking(false);
    }
  }

  async function runBinding() {
    if (!draft.mainApp) return;
    setWorking(true);
    try {
      setReport(await ridBridge.runBinding(draft));
    } catch (error) {
      showToast(
        errorMessage(error, text("试运行失败", "Dry run failed")),
      );
    } finally {
      setWorking(false);
    }
  }

  async function launchBinding() {
    if (!draft.mainApp || !draft.id) return;
    setWorking(true);
    try {
      setReport(await ridBridge.launchBinding(draft));
    } catch (error) {
      showToast(errorMessage(error, text("运行失败", "Binding failed")));
    } finally {
      setWorking(false);
    }
  }

  async function deleteBinding() {
    if (!draft.id) return;
    setWorking(true);
    try {
      await ridBridge.deleteBinding(draft.id);
      setBindings((current) => current.filter((binding) => binding.id !== draft.id));
      selectNew();
      showToast(text("Bind Apps 已删除", "Binding deleted"));
    } catch (error) {
      showToast(errorMessage(error, text("删除失败", "Failed to delete binding")));
    } finally {
      setWorking(false);
    }
  }

  const unavailableIds = new Set(
    pickerGroup === "mainApp"
      ? []
      : [
          ...(draft.mainApp ? [draft.mainApp.id] : []),
          ...(pickerGroup ? draft[pickerGroup].map((app) => app.id) : []),
        ],
  );

  return (
    <LocaleContext.Provider value={locale}>
      <div className="desktop-shell">
      <header className="window-bar" data-tauri-drag-region>
        <button
          className="icon-button icon-button--quiet"
          type="button"
          aria-label={
            sidebarOpen
              ? text("收起侧栏", "Collapse sidebar")
              : text("展开侧栏", "Expand sidebar")
          }
          onClick={() => setSidebarOpen((value) => !value)}
        >
          <List aria-hidden />
        </button>
        <div className="window-brand" data-tauri-drag-region>
          <RidMark size="small" />
          <span>RID</span>
        </div>
        <div className="window-actions">
          <div className="runtime-badge">
            {ridBridge.isNative()
              ? text("桌面版", "Desktop")
              : text("浏览器预览", "Browser preview")}
          </div>
          <button
            className="icon-button icon-button--quiet guide-trigger"
            type="button"
            aria-label={text("打开使用指南", "Open quick guide")}
            title={text("使用指南", "Quick guide")}
            onClick={() => setGuideOpen(true)}
          >
            <BookOpenText aria-hidden />
          </button>
        </div>
      </header>

      <div className={`app-frame${sidebarOpen ? "" : " sidebar-collapsed"}`}>
        <aside
          className="sidebar"
          aria-label={text("RID 导航", "RID navigation")}
        >
          <div className="sidebar__brand">
            <RidMark size="medium" />
            <strong>RID</strong>
          </div>
          <nav className="scene-nav">
            <button
              className={`scene-nav-item new-bind-button${isNew ? " is-active" : ""}`}
              type="button"
              onClick={selectNew}
              aria-current={isNew ? "page" : undefined}
            >
              <span className="new-bind-icon">
                <Plus weight="bold" aria-hidden />
              </span>
              <span>{text("新增应用", "New app")}</span>
            </button>
            <div className="nav-section-label">Bind Apps</div>
            {bindings.map((binding) => (
              <button
                className={`scene-nav-item bind-nav-item${binding.id === activeId ? " is-active" : ""}`}
                type="button"
                key={binding.id}
                title={binding.mainApp.name}
                onClick={() => selectBinding(binding)}
              >
                <AppIcon app={binding.mainApp} size="nav" />
                <span className="bind-nav-item__label">{binding.mainApp.name}</span>
              </button>
            ))}
          </nav>
          <label className="language-setting">
            <Translate aria-hidden />
            <span>{text("语言", "Language")}</span>
            <select
              value={locale}
              aria-label={text("界面语言", "Interface language")}
              onChange={(event) => changeLocale(event.target.value as Locale)}
            >
              <option value="en">English</option>
              <option value="zh-CN">简体中文</option>
            </select>
          </label>
        </aside>

        <main className="workspace">
          <div className="workspace__scroll">
            <section className="scene-header">
              {!isNew && <span className="workspace-eyebrow">Bind Apps</span>}
              <h1>{isNew ? text("新增选项", "New binding") : draft.mainApp?.name}</h1>
              <p>
                {isNew
                  ? text(
                      "选择一个主应用，并为它配置同时打开和临时关闭的应用。",
                      "Choose a main app, then configure apps to open or temporarily close.",
                    )
                  : text(
                      "这个模块以主应用为入口；启动它时，RID 会执行下面的应用绑定。",
                      "The main app is this binding's entry point. RID runs the actions below when it starts.",
                    )}
              </p>
            </section>
            <div className="section-divider" />

            <RuleSection
              title={text("主应用", "Main app")}
              description={text(
                "主应用是这个 Bind Apps 模块的名称和启动入口。",
                "The main app names this binding and acts as its launch entry point.",
              )}
              buttonLabel={
                draft.mainApp
                  ? text("重新选择", "Choose another")
                  : text("选择应用", "Choose app")
              }
              onAdd={() => setPickerGroup("mainApp")}
            >
              {draft.mainApp ? (
                <div className="app-list">
                  <AppRow
                    app={draft.mainApp}
                    status={text("主应用", "Main app")}
                    onRemove={() => removeApp("mainApp", draft.mainApp!.id)}
                    removeLabel={text("清除主应用", "Clear main app")}
                  />
                </div>
              ) : (
                <button className="selection-empty" type="button" onClick={() => setPickerGroup("mainApp")}>
                  <span className="selection-empty__icon"><Plus aria-hidden /></span>
                  <span>
                    <strong>
                      {loading
                        ? text(
                            "正在查找已安装应用…",
                            "Finding installed applications…",
                          )
                        : text("选择主应用", "Choose a main app")}
                    </strong>
                    <small>
                      {text(
                        "支持按名称、路径或别名模糊搜索",
                        "Search by name, path, or alias",
                      )}
                    </small>
                  </span>
                </button>
              )}
            </RuleSection>

            <RuleSection
              title={text("同时打开应用", "Open together")}
              description={text(
                "启动主应用时，一并打开以下应用。",
                "Open these applications with the main app.",
              )}
              onAdd={() => setPickerGroup("openApps")}
            >
              <AppList
                apps={draft.openApps}
                emptyLabel={text(
                  "还没有需要同时打开的应用",
                  "No companion apps yet",
                )}
                status={text("将打开", "Will open")}
                tone="blue"
                onEmptyClick={() => setPickerGroup("openApps")}
                onRemove={(id) => removeApp("openApps", id)}
              />
            </RuleSection>

            <RuleSection
              title={text("临时关闭应用", "Temporarily close")}
              description={text(
                "启动主应用前正常关闭；主应用退出后，只恢复本次成功关闭的应用。",
                "Close normally before launch, then restore only apps RID successfully closed.",
              )}
              onAdd={() => setPickerGroup("closeApps")}
            >
              <AppList
                apps={draft.closeApps}
                emptyLabel={text(
                  "还没有需要临时关闭的应用",
                  "No apps selected for temporary close",
                )}
                status={text("将临时关闭", "Will close")}
                onEmptyClick={() => setPickerGroup("closeApps")}
                onRemove={(id) => removeApp("closeApps", id)}
                forceCloseAppIds={draft.forceCloseAppIds}
                onToggleForceClose={toggleForceClose}
              />
            </RuleSection>

            <div className="recovery-note">
              <ShieldCheck weight="duotone" aria-hidden />
              <p>
                {text(
                  "RID 默认只请求应用正常退出；仅在你为单个应用明确开启时，关闭失败后才会强制结束。",
                  "RID requests a normal exit by default. Force close runs only when explicitly enabled for an app.",
                )}
              </p>
            </div>
          </div>

          <footer className="action-bar">
            {!isNew && (
              <button
                className="delete-button"
                type="button"
                disabled={working}
                onClick={deleteBinding}
                aria-label={text("删除 Bind Apps", "Delete binding")}
              >
                <Trash aria-hidden />
              </button>
            )}
            <button
              className="secondary-button"
              type="button"
              disabled={!draft.mainApp || working}
              onClick={runBinding}
            >
              <Play weight="fill" aria-hidden />
              {working
                ? text("处理中", "Working…")
                : text("试运行", "Dry run")}
            </button>
            {!isNew && (
              <button
                className="launch-button"
                type="button"
                disabled={!draft.mainApp || working}
                onClick={launchBinding}
              >
                <Play weight="fill" aria-hidden />
                {text("运行 Bind Apps", "Run binding")}
              </button>
            )}
            <button
              className="primary-button"
              type="button"
              disabled={!draft.mainApp || working}
              onClick={saveBinding}
            >
              {working
                ? text("正在保存…", "Saving…")
                : isNew
                  ? text("保存 Bind Apps", "Save binding")
                  : text("保存更改", "Save changes")}
            </button>
          </footer>
        </main>
      </div>

      {pickerGroup && (
        <AppPicker
          group={pickerGroup}
          apps={apps.filter((app) => !unavailableIds.has(app.id))}
          onClose={() => setPickerGroup(null)}
          onAdd={addApp}
        />
      )}
      {guideOpen && <GuideDialog onClose={() => setGuideOpen(false)} />}
      {report && <ResultDialog report={report} onClose={() => setReport(null)} />}
      {savedBinding && (
        <div className="modal-backdrop" role="presentation" onMouseDown={() => setSavedBinding(null)}>
          <section
            className="modal modal--run"
            role="dialog"
            aria-modal="true"
            aria-labelledby="saved-title"
            onMouseDown={(event) => event.stopPropagation()}
          >
            <div className="success-mark"><RidMark size="large" /></div>
            <span className="eyebrow">
              {text("保存成功", "Saved")}
            </span>
            <h2 id="saved-title">
              {shortcutPath
                ? text("快捷方式已创建", "Shortcut created")
                : text("Bind Apps 已保存", "Binding saved")}
            </h2>
            <p>
              {shortcutPath
                ? text(
                    "以后双击这个快捷方式，RID 会在后台执行当前 Bind Apps。",
                    "Use this shortcut to run the binding in the background.",
                  )
                : text(
                    "选择一个文件夹创建启动快捷方式；你也可以先跳过，稍后再次保存来创建。",
                    "Choose a folder for the launcher shortcut, or skip it and create one later.",
                  )}
            </p>
            <div className="shortcut-preview">
              <AppIcon app={savedBinding.mainApp} />
              <span>
                <strong>{savedBinding.mainApp.name} · RID</strong>
                <small>
                  {shortcutPath ??
                    text(
                      "关闭指定应用 → 打开搭配应用 → 启动主应用",
                      "Close selected apps → Open companions → Launch main app",
                    )}
                </small>
              </span>
            </div>
            <div className="saved-actions">
              {!shortcutPath && (
                <button
                  className="primary-button primary-button--wide"
                  type="button"
                  disabled={shortcutWorking}
                  onClick={() => createShortcut(savedBinding)}
                >
                  {shortcutWorking
                    ? text("正在创建…", "Creating…")
                    : text("选择位置并创建", "Choose location and create")}
                </button>
              )}
              <button
                className={shortcutPath ? "primary-button primary-button--wide" : "text-button"}
                type="button"
                disabled={shortcutWorking}
                onClick={() => setSavedBinding(null)}
              >
                {shortcutPath
                  ? text("完成", "Done")
                  : text("暂不创建", "Not now")}
              </button>
            </div>
          </section>
        </div>
      )}
      {toast && <div className="toast" role="status">{toast}</div>}
      </div>
    </LocaleContext.Provider>
  );
}

function RuleSection({
  title,
  description,
  buttonLabel,
  onAdd,
  children,
}: {
  title: string;
  description: string;
  buttonLabel?: string;
  onAdd: () => void;
  children: React.ReactNode;
}) {
  return (
    <section className="rule-section">
      <div className="rule-heading">
        <div><h2>{title}</h2><p>{description}</p></div>
        <AddButton label={buttonLabel} onClick={onAdd} />
      </div>
      {children}
    </section>
  );
}

function AppList({
  apps,
  emptyLabel,
  status,
  tone = "green",
  onEmptyClick,
  onRemove,
  forceCloseAppIds = [],
  onToggleForceClose,
}: {
  apps: AppInfo[];
  emptyLabel: string;
  status: string;
  tone?: "green" | "blue";
  onEmptyClick: () => void;
  onRemove: (id: string) => void;
  forceCloseAppIds?: string[];
  onToggleForceClose?: (id: string) => void;
}) {
  const text = useText();
  if (apps.length === 0) {
    return (
      <button className="inline-empty" type="button" onClick={onEmptyClick}>
        <Plus aria-hidden /> {emptyLabel}
      </button>
    );
  }
  return (
    <div className="app-list">
      {apps.map((app) => (
        <AppRow
          key={app.id}
          app={app}
          status={status}
          tone={tone}
          onRemove={() => onRemove(app.id)}
          removeLabel={text("从 Bind Apps 中移除", "Remove from binding")}
          forceClose={forceCloseAppIds.includes(app.id)}
          onToggleForceClose={
            onToggleForceClose ? () => onToggleForceClose(app.id) : undefined
          }
        />
      ))}
    </div>
  );
}
