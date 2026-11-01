// SPDX-License-Identifier: AGPL-3.0-or-later

import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";

async function bootstrap() {
  // The frontend half of WebDriver is loaded only for explicit E2E builds.
  // Vite removes this branch from ordinary production bundles.
  if (import.meta.env.VITE_WDIO === "1") {
    await import("@wdio/tauri-plugin");
    // Keep the document and native test-window titles aligned after reloads so
    // the macOS WebDriver service can deterministically restore focus.
    document.title = "Amara Unfold E2E";
  }

  ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
    <React.StrictMode>
      <App />
    </React.StrictMode>,
  );
}

void bootstrap();
