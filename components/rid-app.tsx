"use client";

import {
  Check,
  CheckCircle,
  DotsThree,
  GearSix,
  LinkSimple,
  List,
  MagnifyingGlass,
  Play,
  Plus,
  ShieldCheck,
  Trash,
  WarningCircle,
  X,
} from "@phosphor-icons/react";
import { useEffect, useMemo, useState } from "react";
import { AppIcon } from "@/components/app-icon";
import { scoreApp } from "@/lib/fuzzy";
import { ridBridge } from "@/lib/tauri";
import type {
  AppInfo,
  Binding,
  BindingDraft,
  PickerGroup,
  RunResult,
} from "@/lib/types";

const createEmptyDraft = (): BindingDraft => ({
  mainApp: null,
  openApps: [],
  closeApps: [],
  forceCloseAppIds: [],
});

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
          aria-label={`${app.name} 更多操作`}
          aria-expanded={menuOpen}
          onClick={() => setMenuOpen((value) => !value)}
        >
          <DotsThree weight="bold" aria-hidden />
        </button>
        {menuOpen && (
          <div className="row-menu__popover">
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
                  关闭失败时强制结束
                  <small>可能导致未保存内容丢失</small>
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
  label = "添加应用",
  onClick,
}: {
  label?: string;
  onClick: () => void;
}) {
  return (
    <button className="add-app-button" type="button" onClick={onClick}>
      <Plus aria-hidden />
      {label}
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
  const labels: Record<PickerGroup, string> = {
    mainApp: "主应用",
    openApps: "同时打开",
    closeApps: "临时关闭",
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
            <h2 id="picker-title">选择一个应用</h2>
          </div>
          <button className="icon-button" type="button" aria-label="关闭" onClick={onClose}>
            <X aria-hidden />
          </button>
        </div>
        <label className="search-field">
          <MagnifyingGlass aria-hidden />
          <input
            value={query}
            onChange={(event) => setQuery(event.target.value)}
            placeholder="模糊搜索应用名、路径或别名"
            autoFocus
          />
        </label>
        <p className="search-hint">例如输入 “obs”、“vscd” 或 “jietu”</p>
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
              <strong>没有找到匹配的应用</strong>
              <span>换一个简称、路径片段或拼音试试。</span>
            </div>
          )}
        </div>
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
        <span className="eyebrow">试运行结果</span>
        <h2 id="run-title">{report.success ? "Bind Apps 已准备好" : "发现需要处理的问题"}</h2>
        <p>{report.message}</p>
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
          完成
        </button>
      </section>
    </div>
  );
}

export function RidApp() {
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

  const isNew = activeId === "new";

  useEffect(() => {
    let cancelled = false;
    Promise.all([ridBridge.listApps(), ridBridge.listBindings()])
      .then(([nextApps, nextBindings]) => {
        if (cancelled) return;
        setApps(nextApps);
        setBindings(nextBindings);
      })
      .catch((error: unknown) => {
        if (!cancelled) showToast(error instanceof Error ? error.message : "载入失败");
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
    showToast("应用已加入当前 Bind Apps");
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
        showToast("更改已保存，原快捷方式已更新");
      } else {
        setSavedBinding(saved);
      }
    } catch (error) {
      showToast(error instanceof Error ? error.message : "保存失败");
    } finally {
      setWorking(false);
    }
  }

  async function createShortcut(binding: Binding) {
    setShortcutWorking(true);
    try {
      const directory = await ridBridge.selectShortcutDirectory();
      if (!directory) return;
      const path = await ridBridge.createBindingShortcut(binding, directory);
      const updated = { ...binding, shortcutPath: path };
      setSavedBinding(updated);
      setDraft((current) => ({ ...current, shortcutPath: path }));
      setBindings((current) =>
        current.map((item) => (item.id === binding.id ? updated : item)),
      );
      setShortcutPath(path);
      showToast("快捷方式已创建");
    } catch (error) {
      showToast(error instanceof Error ? error.message : "创建快捷方式失败");
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
      showToast(error instanceof Error ? error.message : "试运行失败");
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
      showToast(error instanceof Error ? error.message : "运行失败");
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
      showToast("Bind Apps 已删除");
    } catch (error) {
      showToast(error instanceof Error ? error.message : "删除失败");
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
    <div className="desktop-shell">
      <header className="window-bar" data-tauri-drag-region>
        <button
          className="icon-button icon-button--quiet"
          type="button"
          aria-label={sidebarOpen ? "收起侧栏" : "展开侧栏"}
          onClick={() => setSidebarOpen((value) => !value)}
        >
          <List aria-hidden />
        </button>
        <div className="window-brand" data-tauri-drag-region>
          <LinkSimple weight="bold" aria-hidden />
          <span>RID</span>
        </div>
        <div className="runtime-badge">{ridBridge.isNative() ? "Desktop" : "Browser preview"}</div>
      </header>

      <div className={`app-frame${sidebarOpen ? "" : " sidebar-collapsed"}`}>
        <aside className="sidebar" aria-label="RID 导航">
          <div className="sidebar__brand">
            <span className="brand-mark">
              <LinkSimple weight="bold" aria-hidden />
            </span>
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
              <span>新增应用</span>
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
          <button className="settings-button" type="button" onClick={() => showToast("设置将在后续版本开放")}>
            <GearSix aria-hidden />
            设置
          </button>
        </aside>

        <main className="workspace">
          <div className="workspace__scroll">
            <section className="scene-header">
              {!isNew && <span className="workspace-eyebrow">Bind Apps</span>}
              <h1>{isNew ? "新增选项" : draft.mainApp?.name}</h1>
              <p>
                {isNew
                  ? "选择一个主应用，并为它配置同时打开和临时关闭的应用。"
                  : "这个模块以主应用为入口；启动它时，RID 会执行下面的应用绑定。"}
              </p>
            </section>
            <div className="section-divider" />

            <RuleSection
              title="主应用"
              description="主应用是这个 Bind Apps 模块的名称和启动入口。"
              buttonLabel={draft.mainApp ? "重新选择" : "选择应用"}
              onAdd={() => setPickerGroup("mainApp")}
            >
              {draft.mainApp ? (
                <div className="app-list">
                  <AppRow
                    app={draft.mainApp}
                    status="主应用"
                    onRemove={() => removeApp("mainApp", draft.mainApp!.id)}
                    removeLabel="清除主应用"
                  />
                </div>
              ) : (
                <button className="selection-empty" type="button" onClick={() => setPickerGroup("mainApp")}>
                  <span className="selection-empty__icon"><Plus aria-hidden /></span>
                  <span>
                    <strong>{loading ? "正在查找已安装应用…" : "选择主应用"}</strong>
                    <small>支持按名称、路径或别名模糊搜索</small>
                  </span>
                </button>
              )}
            </RuleSection>

            <RuleSection
              title="同时打开应用"
              description="启动主应用时，一并打开以下应用。"
              onAdd={() => setPickerGroup("openApps")}
            >
              <AppList
                apps={draft.openApps}
                emptyLabel="还没有需要同时打开的应用"
                status="将打开"
                tone="blue"
                onEmptyClick={() => setPickerGroup("openApps")}
                onRemove={(id) => removeApp("openApps", id)}
              />
            </RuleSection>

            <RuleSection
              title="临时关闭应用"
              description="启动主应用前正常关闭；主应用退出后，只恢复本次成功关闭的应用。"
              onAdd={() => setPickerGroup("closeApps")}
            >
              <AppList
                apps={draft.closeApps}
                emptyLabel="还没有需要临时关闭的应用"
                status="将临时关闭"
                onEmptyClick={() => setPickerGroup("closeApps")}
                onRemove={(id) => removeApp("closeApps", id)}
                forceCloseAppIds={draft.forceCloseAppIds}
                onToggleForceClose={toggleForceClose}
              />
            </RuleSection>

            <div className="recovery-note">
              <ShieldCheck weight="duotone" aria-hidden />
              <p>RID 默认只请求应用正常退出；仅在你为单个应用明确开启时，关闭失败后才会强制结束。</p>
            </div>
          </div>

          <footer className="action-bar">
            {!isNew && (
              <button
                className="delete-button"
                type="button"
                disabled={working}
                onClick={deleteBinding}
                aria-label="删除 Bind Apps"
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
              {working ? "处理中" : "试运行"}
            </button>
            {!isNew && (
              <button
                className="launch-button"
                type="button"
                disabled={!draft.mainApp || working}
                onClick={launchBinding}
              >
                <Play weight="fill" aria-hidden />
                运行 Bind Apps
              </button>
            )}
            <button
              className="primary-button"
              type="button"
              disabled={!draft.mainApp || working}
              onClick={saveBinding}
            >
              {working ? "正在保存…" : isNew ? "保存 Bind Apps" : "保存更改"}
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
            <div className="success-mark"><LinkSimple weight="bold" aria-hidden /></div>
            <span className="eyebrow">保存成功</span>
            <h2 id="saved-title">{shortcutPath ? "快捷方式已创建" : "Bind Apps 已保存"}</h2>
            <p>
              {shortcutPath
                ? "以后双击这个快捷方式，RID 会在后台执行当前 Bind Apps。"
                : "选择一个文件夹创建启动快捷方式；你也可以先跳过，稍后再次保存来创建。"}
            </p>
            <div className="shortcut-preview">
              <AppIcon app={savedBinding.mainApp} />
              <span>
                <strong>{savedBinding.mainApp.name} · RID</strong>
                <small>{shortcutPath ?? "关闭指定应用 → 打开搭配应用 → 启动主应用"}</small>
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
                  {shortcutWorking ? "正在创建…" : "选择位置并创建"}
                </button>
              )}
              <button
                className={shortcutPath ? "primary-button primary-button--wide" : "text-button"}
                type="button"
                disabled={shortcutWorking}
                onClick={() => setSavedBinding(null)}
              >
                {shortcutPath ? "完成" : "暂不创建"}
              </button>
            </div>
          </section>
        </div>
      )}
      {toast && <div className="toast" role="status">{toast}</div>}
    </div>
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
          removeLabel="从 Bind Apps 中移除"
          forceClose={forceCloseAppIds.includes(app.id)}
          onToggleForceClose={
            onToggleForceClose ? () => onToggleForceClose(app.id) : undefined
          }
        />
      ))}
    </div>
  );
}
