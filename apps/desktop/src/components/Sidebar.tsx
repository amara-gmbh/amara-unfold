// SPDX-License-Identifier: AGPL-3.0-or-later

import {
  CalendarRange,
  CircleHelp,
  Database,
  FolderSearch2,
  House,
  Settings,
  Sparkles,
} from "lucide-react";
import type { Section } from "../types";

const items = [
  { id: "home" as const, label: "Home", icon: House },
  { id: "scan" as const, label: "Scan", icon: FolderSearch2 },
  { id: "timeline" as const, label: "All events", icon: CalendarRange },
  { id: "memories" as const, label: "Photo drafts", icon: Sparkles },
  { id: "settings" as const, label: "Settings", icon: Settings },
  { id: "help" as const, label: "Help", icon: CircleHelp },
];

interface SidebarProps {
  active: Section;
  onNavigate: (section: Section) => void;
}

export function Sidebar({ active, onNavigate }: SidebarProps) {
  return (
    <aside className="sidebar">
      <div className="window-drag-region" data-tauri-drag-region="deep" />
      <div className="brand-lockup">
        <img
          className="brand-logo-light"
          alt="Amara Unfold"
          src="/brand/amara-unfold-flat-cream.svg"
          width="166"
          height="63"
        />
        <img
          className="brand-logo-dark"
          alt="Amara Unfold"
          src="/brand/amara-unfold-flat-mocca.svg"
          width="166"
          height="63"
        />
      </div>

      <nav className="sidebar-nav" aria-label="Workspace">
        {items.map(({ id, label, icon: Icon }) => (
          <button
            className={active === id ? "nav-item active" : "nav-item"}
            key={id}
            onClick={() => onNavigate(id)}
            type="button"
          >
            <Icon aria-hidden="true" size={17} strokeWidth={1.8} />
            <span>{label}</span>
          </button>
        ))}
      </nav>

      <div className="sidebar-spacer" />
      <div className="local-card">
        <div className="local-card-heading">
          <span className="status-dot" />
          <strong>Local only</strong>
        </div>
        <p>Nothing leaves this device.</p>
        <div className="local-card-storage">
          <Database aria-hidden="true" size={14} />
          Private SQLite index
        </div>
      </div>
    </aside>
  );
}
