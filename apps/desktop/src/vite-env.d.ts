// SPDX-License-Identifier: AGPL-3.0-or-later

/// <reference types="vite/client" />

interface ImportMetaEnv {
  readonly VITE_E2E?: string;
  readonly VITE_WDIO?: string;
}

interface ImportMeta {
  readonly env: ImportMetaEnv;
}
