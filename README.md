# RID

RID is a Windows-first desktop app for creating **Bind Apps** modules around a main application.

Each module can:

- launch one main application;
- launch zero or more companion applications;
- temporarily close zero or more applications;
- restore only the applications that RID successfully closed and that were running before the module started.

## Architecture

```text
Next.js static export
  └─ lib/tauri.ts
       ├─ Tauri invoke transport (desktop)
       └─ deterministic mock transport (browser development/tests)

Tauri v2 / Rust
  ├─ installed-application discovery
  ├─ JSON binding persistence
  ├─ process launch and graceful close
  └─ main-process monitoring and selective restoration
```

Next.js is configured with `output: "export"` and emits static assets to `out/`, which Tauri serves as its frontend bundle.

## Development

Prerequisites:

- Node.js 24+
- Rust 1.85+
- Windows WebView2 development environment for running the desktop shell

Install dependencies:

```powershell
npm install
```

Run the browser development mode:

```powershell
npm run dev
```

Run the Tauri desktop application:

```powershell
npm run tauri:dev
```

## Validation

```powershell
npm run typecheck
npm run lint
npm run test
npm run test:e2e
npm run test:rust
npm run build
```

## MVP safety behavior

- RID requests normal application shutdown and does not force-kill by default.
- System processes and protected executable paths are rejected.
- An application is restored only when it was running before execution and RID successfully closed it.
- Browser development uses mock data and never launches or closes local applications.

## Project references

- [RID MVP PRD](docs/RID/PRD_RID.md)
- [Tauri Next.js frontend guide](https://v2.tauri.app/start/frontend/nextjs/)
- [Next.js static export guide](https://nextjs.org/docs/app/guides/static-exports)
