PRAGMA foreign_keys = ON;
PRAGMA journal_mode = WAL;

CREATE TABLE IF NOT EXISTS schema_migrations (
  version INTEGER PRIMARY KEY,
  applied_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS sources (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  importer_id TEXT NOT NULL DEFAULT 'local-folder',
  path TEXT NOT NULL UNIQUE,
  display_name TEXT NOT NULL,
  created_at TEXT NOT NULL,
  last_scanned_at TEXT
);

CREATE TABLE IF NOT EXISTS scans (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  source_id INTEGER NOT NULL REFERENCES sources(id) ON DELETE CASCADE,
  status TEXT NOT NULL CHECK(status IN ('running', 'completed', 'cancelled', 'failed')),
  started_at TEXT NOT NULL,
  finished_at TEXT,
  files_count INTEGER NOT NULL DEFAULT 0,
  photos_count INTEGER NOT NULL DEFAULT 0,
  videos_count INTEGER NOT NULL DEFAULT 0,
  unknown_count INTEGER NOT NULL DEFAULT 0,
  duplicates_count INTEGER NOT NULL DEFAULT 0,
  errors_count INTEGER NOT NULL DEFAULT 0,
  bytes_count INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS assets (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  source_id INTEGER NOT NULL REFERENCES sources(id) ON DELETE CASCADE,
  last_scan_id INTEGER NOT NULL REFERENCES scans(id) ON DELETE RESTRICT,
  path TEXT NOT NULL,
  filename TEXT NOT NULL,
  extension TEXT NOT NULL,
  media_kind TEXT NOT NULL CHECK(media_kind IN ('image', 'video', 'unknown')),
  size_bytes INTEGER NOT NULL,
  created_at TEXT,
  modified_at TEXT,
  captured_at TEXT,
  effective_date TEXT,
  content_hash TEXT,
  UNIQUE(source_id, path)
);

CREATE TABLE IF NOT EXISTS metadata (
  asset_id INTEGER PRIMARY KEY REFERENCES assets(id) ON DELETE CASCADE,
  width INTEGER,
  height INTEGER,
  duration_seconds REAL,
  latitude REAL,
  longitude REAL,
  device_make TEXT,
  device_model TEXT,
  extractor TEXT NOT NULL,
  extra_json TEXT NOT NULL DEFAULT '{}'
);

CREATE TABLE IF NOT EXISTS memory_candidates (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  source_id INTEGER NOT NULL REFERENCES sources(id) ON DELETE CASCADE,
  scan_id INTEGER NOT NULL REFERENCES scans(id) ON DELETE CASCADE,
  cluster_key TEXT NOT NULL,
  title TEXT NOT NULL,
  date_label TEXT NOT NULL,
  place_hint TEXT,
  asset_count INTEGER NOT NULL,
  photo_count INTEGER NOT NULL,
  video_count INTEGER NOT NULL,
  evidence_json TEXT NOT NULL,
  created_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS candidate_assets (
  candidate_id INTEGER NOT NULL REFERENCES memory_candidates(id) ON DELETE CASCADE,
  asset_id INTEGER NOT NULL REFERENCES assets(id) ON DELETE CASCADE,
  reason TEXT NOT NULL,
  PRIMARY KEY(candidate_id, asset_id)
);

CREATE TABLE IF NOT EXISTS scan_errors (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  scan_id INTEGER NOT NULL REFERENCES scans(id) ON DELETE CASCADE,
  message TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_assets_effective_date ON assets(effective_date DESC);
CREATE INDEX IF NOT EXISTS idx_assets_content_hash ON assets(content_hash) WHERE content_hash IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_scans_source_started ON scans(source_id, started_at DESC);
CREATE INDEX IF NOT EXISTS idx_candidates_date ON memory_candidates(date_label DESC);

INSERT OR IGNORE INTO schema_migrations(version, applied_at)
VALUES (1, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'));
