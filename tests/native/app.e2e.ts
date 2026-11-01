// SPDX-License-Identifier: AGPL-3.0-or-later

import { $, $$, browser, expect } from "@wdio/globals";
import {
  copyFile,
  mkdir,
  mkdtemp,
  readFile,
  readdir,
  rm,
  stat,
  utimes,
  writeFile,
} from "node:fs/promises";
import { tmpdir } from "node:os";
import path from "node:path";

describe("Amara Unfold native desktop acceptance", () => {
  let fixtureRoot: string;
  let archive: string;
  let openMarkdown: string;
  let adapterSource: string;
  let adapterOutput: string;
  let airbnbSource: string;
  let airbnbOutput: string;

  beforeAll(async () => {
    // A fresh owned directory avoids deleting a previous run's data.
    fixtureRoot = await mkdtemp(
      path.join(tmpdir(), "amara-unfold-native-e2e-"),
    );
    archive = path.join(fixtureRoot, "archive");
    openMarkdown = path.join(fixtureRoot, "open-markdown");
    adapterSource = path.join(fixtureRoot, "adapter-source");
    adapterOutput = path.join(fixtureRoot, "adapter-output");
    airbnbSource = path.join(fixtureRoot, "airbnb-source");
    airbnbOutput = path.join(fixtureRoot, "airbnb-output");
    await mkdir(archive, { recursive: true });
    await mkdir(path.join(openMarkdown, "library"), { recursive: true });
    await mkdir(adapterSource, { recursive: true });
    await mkdir(adapterOutput, { recursive: true });
    await mkdir(path.join(airbnbSource, "json"), { recursive: true });
    await mkdir(path.join(airbnbSource, "images", "profile"), {
      recursive: true,
    });
    await mkdir(airbnbOutput, { recursive: true });
    await writeFile(
      path.join(adapterSource, "subscriptions.csv"),
      "Channel Id,Channel Url,Channel Title\nabc,https://youtube.example/abc,Archive Channel\n",
    );
    await writeFile(
      path.join(airbnbSource, "json", "reservations.json"),
      '[{"reservations":[{"startDate":"2024-06-01","nights":3,"numberOfGuests":2,"status":"accepted"},{"startDate":"2024-07-01","nights":2,"status":"cancelled"}],"bookingSessions":[{"homeInquiries":[{"startDate":"2024-08-01","nights":5}]}]}]',
    );
    await writeFile(
      path.join(airbnbSource, "json", "messages.json"),
      '[{"messageThreads":[{"id":42,"messagesAndContents":[{"message":{"accountType":"guest","createdAt":"2024-05-01T12:30:00Z"},"messageContent":{"textContent":{"body":"Looking forward to the stay."}}}]}]}]',
    );
    await writeFile(
      path.join(airbnbSource, "json", "reviews.json"),
      '[{"reviewsProvided":[{"review":{"submittedAt":"2024-06-05T10:00:00Z","comment":"A welcoming home.","rating":5,"revieweeId":10}}],"reviewsReceived":[]}]',
    );
    await writeFile(
      path.join(airbnbSource, "images", "profile", "original.jpg"),
      "profile image fixture",
    );
    await writeFile(
      path.join(openMarkdown, "manifest.yaml"),
      "name: photos\nschema_version: 1\n",
    );
    await writeFile(path.join(openMarkdown, "README.md"), "# Photos\n");
    await mkdir(path.join(archive, "Jersey City"));
    const sourceIcon = path.resolve("apps/desktop/src-tauri/icons/32x32.png");
    await copyFile(
      sourceIcon,
      path.join(archive, "Jersey City", "photo-a.png"),
    );
    await copyFile(
      sourceIcon,
      path.join(archive, "Jersey City", "photo-a-copy.png"),
    );
    await writeFile(
      path.join(archive, "Jersey City", "clip.mov"),
      "synthetic video fixture",
    );
    await writeFile(
      path.join(archive, "README.txt"),
      "synthetic unknown fixture",
    );
    const fixtureDate = new Date("2019-08-30T12:00:00Z");
    await Promise.all(
      ["photo-a.png", "photo-a-copy.png", "clip.mov"].map((name) =>
        utimes(
          path.join(archive, "Jersey City", name),
          fixtureDate,
          fixtureDate,
        ),
      ),
    );
  });

  afterAll(async () => {
    if (fixtureRoot) await rm(fixtureRoot, { recursive: true, force: true });
  });

  it("launches the real Tauri window in local-only mode", async () => {
    await expect($("h1")).toHaveText(
      "Discover your digital traces. Understand what matters. Conserve your memories in open files.",
    );
    await expect(
      $(
        '//*[normalize-space(.)="Discover your digital traces. Understand what matters. Conserve your memories in open files."]',
      ),
    ).toBeDisplayed();
    await expect(
      $(
        '//*[normalize-space(.)="Because an export is not yet a memory you can use."]',
      ),
    ).toBeDisplayed();
    await expect($('//*[normalize-space(.)="Ask it—locally"]')).toBeDisplayed();
    await expect(
      $('//*[normalize-space(.)="Not metadata extractor number 27."]'),
    ).toBeDisplayed();
    const privacy = await browser.tauri.execute(({ core }) =>
      core.invoke("privacy_status"),
    );
    expect(privacy).toEqual(
      jasmine.objectContaining({
        mode: "local_only",
        networkEnabled: false,
        telemetryEnabled: false,
      }),
    );
  });

  it("renders the complete end-user Help module in the native Tauri window", async () => {
    await $('//button[normalize-space(.)="Help"]').click();
    await expect($("h1")).toHaveText("Help");
    await expect(
      $('//*[normalize-space(.)="Four steps from export to timeline"]'),
    ).toBeDisplayed();
    await expect(
      $('//*[normalize-space(.)="Sources you can use"]'),
    ).toBeDisplayed();
    await expect(
      $('//*[normalize-space(.)="Privacy and safe use"]'),
    ).toBeDisplayed();
    await expect(
      $('//*[normalize-space(.)="Why Unfold exists"]'),
    ).toBeDisplayed();
    await expect(
      $('//*[normalize-space(.)="Fully local. Zero outbound traffic."]'),
    ).toBeDisplayed();
    await expect(
      $('//*[normalize-space(.)="Discovery without surveillance"]'),
    ).toBeDisplayed();
    await expect(
      $('//*[normalize-space(.)="Why use Amara Unfold?"]'),
    ).toBeDisplayed();
    await expect(
      $('//*[normalize-space(.)="This is not metadata extractor number 27"]'),
    ).toBeDisplayed();
    await expect(
      $('//*[normalize-space(.)="What can you do with the information?"]'),
    ).toBeDisplayed();
    await expect(
      $(
        '//*[normalize-space(.)="Amara\'s suggestion: try it with a truly local LLM"]',
      ),
    ).toBeDisplayed();
    await expect(
      $(
        '//*[normalize-space(.)="Why Amara Memory is more than Markdown plus an LLM"]',
      ),
    ).toBeDisplayed();
    await expect(
      $('//*[normalize-space(.)="Connections stay evidence-based"]'),
    ).toBeDisplayed();
    await expect(
      $('//*[normalize-space(.)="For each successfully converted source"]'),
    ).toBeDisplayed();
    await expect(
      $(
        '//*[contains(normalize-space(.), "entity indexes for people, places, merchants, artists, and channels")]',
      ),
    ).toBeDisplayed();
    await expect(
      $('//*[normalize-space(.)="Future modules to build"]'),
    ).toBeDisplayed();
    await expect($('//*[normalize-space(.)="Podcasts"]')).toBeDisplayed();
    await expect($('//*[normalize-space(.)="Build with us"]')).toBeDisplayed();
    await expect(
      $('//*[normalize-space(.)="info@amara.gmbh"]'),
    ).toBeDisplayed();
    await expect(
      $('//*[normalize-space(.)="Troubleshooting"]'),
    ).toBeDisplayed();
    await expect(
      $(
        '//*[contains(normalize-space(.), "Apple Messages, Apple Notes, Apple Journal")]',
      ),
    ).toBeDisplayed();
    await expect(
      $('//button[normalize-space(.)="Start in Scan"]'),
    ).toBeDisplayed();
  });

  it("exposes local-app Scan sources through real Tauri IPC", async () => {
    type LocalAppSource = {
      id: string;
      label: string;
      adapterId: string | null;
      path: string | null;
      status: string;
      detail: string;
      privacyNote: string;
      evidence: string[];
    };
    const sources = (await browser.tauri.execute(({ core }) =>
      core.invoke("local_app_sources"),
    )) as LocalAppSource[];

    expect(sources.map(({ label }) => label)).toEqual([
      "Apple Photos",
      "Apple Calendar",
      "Apple Contacts",
      "Apple Mail",
      "Apple Music",
      "Immich",
      "Apple Messages / iMessage",
      "Apple Notes",
      "Apple Journal",
    ]);

    const byId = new Map(sources.map((source) => [source.id, source]));
    expect(byId.get("apple-mail")?.adapterId).toBe("apple-mail-local");
    expect(byId.get("apple-photos")?.adapterId).toBe("photo-media-folder");
    expect(byId.get("apple-music")?.adapterId).toBe(
      "apple-music-local-library",
    );
    expect(byId.get("immich")?.adapterId).toBeNull();
    expect(byId.get("apple-calendar")?.adapterId).toBe("apple-calendar-local");
    expect(byId.get("apple-contacts")?.adapterId).toBe("apple-contacts-local");

    for (const quietOnLaunch of [
      "apple-photos",
      "apple-calendar",
      "apple-mail",
      "immich",
      "apple-messages",
      "apple-notes",
      "apple-journal",
    ]) {
      expect(byId.get(quietOnLaunch)?.path).toBeNull();
      expect(
        byId
          .get(quietOnLaunch)
          ?.evidence.some((item) => item.startsWith("found ")),
      ).toBeFalse();
    }

    for (const roadmapOnly of [
      "immich",
      "apple-messages",
      "apple-notes",
      "apple-journal",
    ]) {
      expect(byId.get(roadmapOnly)?.adapterId).toBeNull();
      expect(byId.get(roadmapOnly)?.privacyNote).toMatch(
        /(does not inspect|not available|never reads|never auto-run|local-only)/,
      );
    }
    for (const implemented of [
      "apple-photos",
      "apple-mail",
      "apple-music",
      "apple-calendar",
      "apple-contacts",
    ]) {
      expect(["ready", "needs_permission", "needs_preparation"]).toContain(
        byId.get(implemented)?.status,
      );
      expect(byId.get(implemented)?.detail).toMatch(
        /does not inspect|does not contact|until you choose|readable|export/i,
      );
    }
  });

  it("renders the local-app Scan panel in the native Tauri window", async () => {
    await $('//button[normalize-space(.)="Scan"]').click();
    await expect($("h1")).toHaveText("Create open Markdown.");
    await expect(
      $('//*[normalize-space(.)="Choose raw data"]'),
    ).toBeDisplayed();
    await expect(
      $('//button[normalize-space(.)="Choose raw file"]'),
    ).toBeDisplayed();
    await expect(
      $('//button[normalize-space(.)="Choose raw folder"]'),
    ).toBeDisplayed();
    const applicationRoutes = $(
      '//details[contains(@class, "local-app-intake")]/summary[.//*[normalize-space(.)="Choose from an application"]]',
    );
    await expect(applicationRoutes).toBeDisplayed();
    await applicationRoutes.click();
    await expect(
      $('//button[.//*[normalize-space(.)="Available now"]]'),
    ).toBeDisplayed();
    await expect(
      $('//button[normalize-space(.)="Refresh app exports"]'),
    ).toBeDisplayed();
    await expect(
      $('//button[.//*[normalize-space(.)="Not available yet"]]'),
    ).toBeDisplayed();

    for (const label of ["Apple Photos", "Apple Mail", "Apple Music"]) {
      await expect($(`//h3[normalize-space(.)="${label}"]`)).toBeDisplayed();
    }
    await $(
      '//section[contains(@class, "local-app-card")][.//h3[normalize-space(.)="Apple Mail"]]//summary[normalize-space(.)="How access works"]',
    ).click();
    await expect(
      $(
        '//section[contains(@class, "local-app-card")][.//h3[normalize-space(.)="Apple Mail"]]//*[normalize-space(.)="1. Export in Mail"]',
      ),
    ).toBeDisplayed();
    await expect(
      $(
        '//section[contains(@class, "local-app-card")][.//h3[normalize-space(.)="Apple Mail"]]//button[normalize-space(.)="Prepare Mail export"]',
      ),
    ).toBeDisplayed();

    for (const label of ["Apple Calendar", "Apple Contacts"]) {
      await expect($(`//h3[normalize-space(.)="${label}"]`)).toBeDisplayed();
    }

    for (const implemented of [
      "Apple Photos",
      "Apple Mail",
      "Apple Music",
      "Apple Calendar",
      "Apple Contacts",
    ]) {
      const action = await $(
        `//section[contains(@class, "local-app-card")][.//h3[normalize-space(.)="${implemented}"]]//button[normalize-space(.)="Choose Photos export" or normalize-space(.)="Advanced: default Mail data" or normalize-space(.)="Use Music library" or normalize-space(.)="Use Immich folder" or normalize-space(.)="Choose Calendar export" or normalize-space(.)="Choose Contacts export" or normalize-space(.)="Choose data folder" or normalize-space(.)="Choose Mail export" or normalize-space(.)="Use found Mail export"]`,
      );
      await expect(action).toBeDisplayed();
      await expect(action).toBeEnabled();
    }

    await expect(
      $('//button[normalize-space(.)="Adapter pending"]'),
    ).not.toBeDisplayed();

    await $('//button[.//*[normalize-space(.)="Not available yet"]]').click();
    await expect(
      $('//*[contains(@class, "future-app-list")][contains(., "Immich")]'),
    ).toBeDisplayed();

    const screenshotDir = path.resolve("test-results/native/uat-screenshots");
    await mkdir(screenshotDir, { recursive: true });
    await browser.saveScreenshot(
      path.join(screenshotDir, "scan-local-apps-2026-7-27.png"),
    );
  });

  it("recognizes existing open Markdown and offers no extraction command", async () => {
    const inspection = (await browser.tauri.execute(
      ({ core }, sourcePath) =>
        core.invoke("inspect_dataset", { path: sourcePath }),
      openMarkdown,
    )) as {
      outputContract: string;
      detections: Array<{
        adapterId: string;
        readiness: string;
        commandPreview: string | null;
      }>;
    };

    expect(inspection.outputContract).toContain("Markdown");
    expect(inspection.detections[0]).toEqual(
      jasmine.objectContaining({
        adapterId: "foreverlane-photo-markdown-v1",
        readiness: "already_open",
        commandPreview: null,
      }),
    );
  });

  it("runs an offline adapter through real Tauri IPC and writes portable Markdown", async () => {
    const result = (await browser.tauri.execute(
      ({ core }, [adapterId, source, destination]) =>
        core.invoke("extract_dataset", { adapterId, source, destination }),
      ["youtube-takeout", adapterSource, adapterOutput],
    )) as { folder: string; eventsWritten: number; errors: number };
    expect(result.eventsWritten).toBe(1);
    expect(result.errors).toBe(0);
    const eventFiles = await readdir(
      path.join(result.folder, "events", "0000", "00"),
    );
    const event = await readFile(
      path.join(result.folder, "events", "0000", "00", eventFiles[0]),
      "utf8",
    );
    expect(event).toContain("schema_version: 1");
    expect(event).toContain("module: youtube");
    expect(event).toContain('date: "0000-00-00"');
    expect(event).toContain("# Archive Channel");
    expect(event).toContain("raw_hash: sha256:");
    const manifest = await readFile(
      path.join(result.folder, "manifest.yaml"),
      "utf8",
    );
    expect(manifest).toContain("networkUsed: false");

    const timeline = (await browser.tauri.execute(
      ({ core }, destination) =>
        core.invoke("build_timeline_index", { destination }),
      adapterOutput,
    )) as {
      folder: string;
      eventsIndexed: number;
      undatedEvents: number;
      strictPortable: boolean;
    };
    expect(timeline.eventsIndexed).toBe(1);
    expect(timeline.undatedEvents).toBe(1);
    expect(timeline.strictPortable).toBeTrue();
    expect(
      (await stat(path.join(timeline.folder, "index.md"))).isFile(),
    ).toBeTrue();
    expect(
      (await stat(path.join(timeline.folder, "catalog.csv"))).isFile(),
    ).toBeTrue();
  });

  it("prioritizes and extracts a standard Airbnb export over embedded photos", async () => {
    const inspection = (await browser.tauri.execute(
      ({ core }, sourcePath) =>
        core.invoke("inspect_dataset", { path: sourcePath }),
      airbnbSource,
    )) as { detections: Array<{ adapterId: string }> };
    expect(inspection.detections[0].adapterId).toBe("airbnb-json-takeout");
    expect(inspection.detections.map(({ adapterId }) => adapterId)).toContain(
      "photo-media-folder",
    );

    const result = (await browser.tauri.execute(
      ({ core }, [adapterId, source, destination]) =>
        core.invoke("extract_dataset", { adapterId, source, destination }),
      ["airbnb-json-takeout", airbnbSource, airbnbOutput],
    )) as { folder: string; eventsWritten: number; errors: number };
    expect(result.eventsWritten).toBe(4);
    expect(result.errors).toBe(0);
    const files = await readdir(path.join(result.folder, "events"), {
      recursive: true,
    });
    const eventFiles = files.filter((file) => file.endsWith(".md"));
    expect(eventFiles).toHaveSize(4);
    expect(
      eventFiles.some((file) => file.includes("cancelled-reservation")),
    ).toBeTrue();
    const manifest = await readFile(
      path.join(result.folder, "manifest.yaml"),
      "utf8",
    );
    expect(manifest).toContain("adapterId: airbnb-json-takeout");
  });

  it("scans synthetic media through real Rust IPC and persists SQLite results", async () => {
    const scan = await browser.tauri.execute(
      ({ core }, folder) => core.invoke("scan_folder", { folder }),
      archive,
    );
    expect(scan).toEqual(
      jasmine.objectContaining({
        cancelled: false,
        counts: jasmine.objectContaining({
          files: 4,
          photos: 2,
          videos: 1,
          unknown: 1,
          duplicates: 1,
          errors: 0,
        }),
      }),
    );

    await browser.refresh();
    await $('//button[normalize-space(.)="Home"]').click();
    await expect($("h1")).toHaveText(
      "Discover your digital traces. Understand what matters. Conserve your memories in open files.",
    );
    const dashboard = await browser.tauri.execute(({ core }) =>
      core.invoke("get_dashboard"),
    );
    expect(dashboard).toEqual(
      jasmine.objectContaining({
        counts: jasmine.objectContaining({ files: 4, duplicates: 1 }),
        timeline: jasmine.arrayContaining([
          jasmine.objectContaining({ key: "2019-08", photos: 2, videos: 1 }),
        ]),
      }),
    );
  });

  it("renders the persisted timeline and traceable candidate evidence", async () => {
    await $('//button[normalize-space(.)="Photo drafts"]').click();
    await expect($('//h2[normalize-space(.)="August 2019"]')).toBeDisplayed();
    const memoryToggle = await $(".candidate-summary");
    await expect(memoryToggle).toHaveAttribute("aria-expanded", "false");
    await memoryToggle.click();
    await expect(memoryToggle).toHaveAttribute("aria-expanded", "true");
    await expect(
      $('//*[normalize-space(.)="Observed from file metadata"]'),
    ).toBeDisplayed();
    await expect($$(".memory-gallery")).toBeElementsArrayOfSize(0);
    await expect(
      $('//button[normalize-space(.)="Export open Markdown"]'),
    ).toBeDisplayed();
    await expect(
      $('//strong[normalize-space(.)="photo-a.png"]'),
    ).toBeDisplayed();
    await expect(
      $('//*[contains(normalize-space(.), "BLAKE3")]'),
    ).toBeDisplayed();
  });

  it("exports open schema-compatible Markdown without copying media", async () => {
    const snapshot = (await browser.tauri.execute(({ core }) =>
      core.invoke("get_dashboard"),
    )) as { candidates: Array<{ id: number }> };
    const destination = path.join(fixtureRoot, "native-export");
    await mkdir(destination);
    const result = (await browser.tauri.execute(
      ({ core }, [candidateId, folder]) =>
        core.invoke("export_memory_markdown", { candidateId, folder }),
      [snapshot.candidates[0].id, destination],
    )) as { folder: string; eventsWritten: number };

    expect(result.eventsWritten).toBe(3);
    const readme = await readFile(
      path.join(result.folder, "README.md"),
      "utf8",
    );
    expect(readme).toContain("open, local Markdown archive");
    const eventFolder = path.join(
      result.folder,
      "photos",
      "library",
      "2019",
      "08",
      "30",
    );
    const eventFiles = await readdir(eventFolder);
    expect(eventFiles).toHaveSize(3);
    const event = await readFile(path.join(eventFolder, eventFiles[0]), "utf8");
    expect(event).toContain("schema_version: 1");
    expect(event).toContain("provenance:\n  raw_path:");
    expect(event).toContain('raw_hash: "sha256:');
    expect(event).toContain("module: amara-unfold/0.1");
    expect(event).toContain("BLAKE3:");
  });

  it("removes only the local index and leaves the source folder untouched", async () => {
    const snapshot = await browser.tauri.execute(({ core }) =>
      core.invoke("get_dashboard"),
    );
    const sourceId = (snapshot as { sources: Array<{ id: number }> }).sources[0]
      .id;
    const afterRemoval = await browser.tauri.execute(
      ({ core }, id) => core.invoke("remove_source", { sourceId: id }),
      sourceId,
    );

    expect(afterRemoval).toEqual(
      jasmine.objectContaining({
        sources: [],
        counts: jasmine.objectContaining({ files: 0 }),
        timeline: [],
        candidates: [],
      }),
    );
    expect(
      (await stat(path.join(archive, "Jersey City", "photo-a.png"))).isFile(),
    ).toBeTrue();
  });
});
