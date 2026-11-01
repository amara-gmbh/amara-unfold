// SPDX-License-Identifier: AGPL-3.0-or-later

import AxeBuilder from "@axe-core/playwright";
import { expect, test, type Page } from "@playwright/test";

const externalRequestsByPage = new WeakMap<Page, string[]>();

test.beforeEach(async ({ page }) => {
  const externalRequests: string[] = [];
  externalRequestsByPage.set(page, externalRequests);
  page.on("request", (request) => {
    const url = new URL(request.url());
    if (url.protocol === "blob:") return;
    if (!["127.0.0.1", "localhost", "[::1]"].includes(url.hostname))
      externalRequests.push(request.url());
  });
  await page.goto("/");
  await expect(
    page.getByRole("heading", {
      name: "Discover your digital traces. Understand what matters. Conserve your memories in open files.",
    }),
  ).toBeVisible();
});

test.afterEach(async ({ page }) => {
  expect(
    externalRequestsByPage.get(page),
    "the renderer must not make external requests during any journey",
  ).toEqual([]);
});

test("exposes usable custom window drag regions", async ({ page }) => {
  await expect(page.locator(".workspace-titlebar")).toHaveAttribute(
    "data-tauri-drag-region",
    "deep",
  );
  await expect(page.locator(".window-drag-region")).toHaveAttribute(
    "data-tauri-drag-region",
    "deep",
  );
  const logo = page.locator(".brand-lockup img:visible");
  await expect(logo).toHaveAttribute(
    "src",
    /amara-unfold-flat-(cream|mocca)\.svg/,
  );
  await expect
    .poll(() => logo.evaluate((image: HTMLImageElement) => image.naturalWidth))
    .toBeGreaterThan(0);
});

test("empty archive exposes all six calm workspace sections", async ({
  page,
}) => {
  await expect(
    page.getByText("Local only", { exact: true }).first(),
  ).toBeVisible();
  const privacyMessage = page.getByText(
    "Your files remain where they are. The index lives privately on this device.",
    { exact: true },
  );
  await expect(privacyMessage).toBeVisible();
  await expect
    .poll(() =>
      privacyMessage.evaluate((element) =>
        Number.parseFloat(getComputedStyle(element).fontSize),
      ),
    )
    .toBeGreaterThanOrEqual(13);
  await expect(page.getByText("No folders indexed")).toBeVisible();
  await expect(
    page.getByRole("heading", {
      name: "Discover your digital traces. Understand what matters. Conserve your memories in open files.",
    }),
  ).toBeVisible();
  await expect(
    page.getByText("Unfold is not surveillance, scoring, or a hidden dossier", {
      exact: false,
    }),
  ).toBeVisible();
  await expect(
    page.getByRole("heading", {
      name: "Because an export is not yet a memory you can use.",
    }),
  ).toBeVisible();
  await expect(page.getByText("Ask it—locally", { exact: true })).toBeVisible();
  await expect(
    page.getByText("Not metadata extractor number 27.", { exact: true }),
  ).toBeVisible();
  await expect(
    page.getByText("This build contains no LLM or live Amara Memory bridge", {
      exact: false,
    }),
  ).toBeVisible();

  await page.getByRole("button", { name: "Scan", exact: true }).click();
  await expect(
    page.getByRole("heading", { name: "Create open Markdown." }),
  ).toBeVisible();

  await page.getByRole("button", { name: "Photo drafts", exact: true }).click();
  await expect(
    page.getByRole("heading", { name: "No photo event drafts yet" }),
  ).toBeVisible();

  await page.getByRole("button", { name: "All events", exact: true }).click();
  await expect(
    page.getByRole("heading", { name: "No exported events yet" }),
  ).toBeVisible();

  await page.getByRole("button", { name: "Settings", exact: true }).click();
  await expect(page.getByText("Network access")).toBeVisible();
  await expect(page.getByText("Analytics and telemetry")).toBeVisible();
  await expect(
    page.getByText("amara information security GmbH."),
  ).toBeVisible();
  await expect(page.getByText("Off", { exact: true })).toHaveCount(2);

  await page.getByRole("button", { name: "Help", exact: true }).click();
  await expect(page.getByRole("heading", { name: "Help" })).toBeVisible();
  await expect(
    page.getByRole("heading", { name: "Four steps from export to timeline" }),
  ).toBeVisible();
});

test("recognizes existing Digi-Pics Markdown without re-extracting it", async ({
  page,
}) => {
  await page.goto("/?scenario=open-markdown");
  await page.getByRole("button", { name: "Choose a folder" }).click();

  await expect(
    page.getByRole("heading", { name: "Photo metadata Markdown" }),
  ).toBeVisible();
  await expect(
    page.getByRole("heading", {
      name: "This folder is already open Markdown",
    }),
  ).toBeVisible();
  await expect(page.getByText(/55,517 files/)).toBeVisible();
  await expect(page.getByLabel("Takeout to Markdown workflow")).toContainText(
    "Choose sourceDoneConfirm formatDoneChoose destinationSkippedCreate MarkdownDone",
  );
  await expect(page.getByText("No conversion needed")).toBeVisible();
  await expect(
    page.getByRole("button", { name: "Create Markdown files" }),
  ).toHaveCount(0);
  await page.getByText("Recognition details").click();
  await expect(
    page.getByText("do not extract it again", { exact: false }),
  ).toBeVisible();
});

test("matches Amara Memory light and dark theme tokens without changing navigation", async ({
  page,
}) => {
  await expect(page.locator("html")).toHaveAttribute("data-theme", "dark");
  await page.getByRole("button", { name: "Settings", exact: true }).click();
  await page.getByRole("button", { name: "Dark", exact: true }).click();
  await expect(page.locator("html")).toHaveAttribute("data-theme", "dark");
  await expect(
    page.getByRole("button", { name: "Dark", exact: true }),
  ).toHaveAttribute("aria-pressed", "true");
  expect(
    await page.evaluate(() => ({
      background: getComputedStyle(document.documentElement).backgroundColor,
      color: getComputedStyle(document.documentElement).color,
      font: getComputedStyle(document.documentElement).fontFamily,
      semanticTokens: ["--sage", "--sage-dark", "--danger", "--focus"].map(
        (token) =>
          getComputedStyle(document.documentElement)
            .getPropertyValue(token)
            .trim(),
      ),
      saved: window.localStorage.getItem("amara-unfold-theme"),
    })),
  ).toEqual(
    expect.objectContaining({
      background: "rgb(24, 23, 19)",
      color: "rgb(222, 216, 204)",
      semanticTokens: Array(4).fill("#ded8cc"),
      saved: "dark",
    }),
  );
  await expect(
    page.getByRole("button", { name: "Settings", exact: true }),
  ).toHaveCSS("box-shadow", "none");
  expect(
    await page.evaluate(() =>
      getComputedStyle(document.documentElement).fontFamily.includes(
        "Segoe UI",
      ),
    ),
  ).toBe(true);
  await page.reload();
  await expect(page.locator("html")).toHaveAttribute("data-theme", "dark");

  await expect(
    page.getByRole("navigation", { name: "Workspace" }),
  ).toContainText("HomeScanAll eventsPhoto draftsSettingsHelp");
  await page.getByRole("button", { name: "Settings", exact: true }).click();
  await page.getByRole("button", { name: "Light", exact: true }).click();
  await expect(page.locator("html")).toHaveAttribute("data-theme", "light");
  expect(
    await page.evaluate(() => ({
      background: getComputedStyle(document.documentElement).backgroundColor,
      color: getComputedStyle(document.documentElement).color,
    })),
  ).toEqual({
    background: "rgb(250, 249, 245)",
    color: "rgb(36, 34, 31)",
  });
});

test("dark mode keeps the warmer coffee surface throughout", async ({
  page,
}) => {
  await page.goto("/?scenario=populated");
  await page.getByRole("button", { name: "Settings", exact: true }).click();
  await page.getByRole("button", { name: "Dark", exact: true }).click();

  const disconnected = page.getByRole("button", {
    name: "Not included in this release",
  });
  await expect(disconnected).toBeVisible();
  expect(
    await disconnected.evaluate(
      (element) => getComputedStyle(element).backgroundColor,
    ),
  ).not.toBe("rgb(240, 241, 238)");

  await page.getByRole("button", { name: "Home", exact: true }).click();
  const scanButton = page.getByRole("button", { name: "Scan another folder" });
  await expect(scanButton).toBeVisible();
  expect(
    await scanButton.evaluate(
      (element) => getComputedStyle(element).backgroundColor,
    ),
  ).toBe("rgb(24, 23, 19)");
  await expect(scanButton).toHaveCSS("color", "rgb(222, 216, 204)");
  const conserveLine = page.locator(".home-conserve-line");
  await expect(conserveLine).toBeVisible();
  expect(
    await conserveLine.evaluate((element) => element.getClientRects().length),
  ).toBe(1);

  const darkTextHierarchy = await page.evaluate(() => ({
    pageTitle: getComputedStyle(
      document.querySelector<HTMLElement>(".page-header h1")!,
    ).color,
    normalText: getComputedStyle(
      document.querySelector<HTMLElement>(".trust-strip strong")!,
    ).color,
    secondaryText: getComputedStyle(
      document.querySelector<HTMLElement>(".page-header p")!,
    ).color,
  }));
  expect(darkTextHierarchy).toEqual({
    pageTitle: "rgb(250, 249, 245)",
    normalText: "rgb(222, 216, 204)",
    secondaryText: "rgb(184, 178, 164)",
  });

  await page.getByRole("button", { name: "Photo drafts", exact: true }).click();
  const candidateCard = page.locator(".candidate-card").first();
  await expect(candidateCard).toBeVisible();
  expect(
    await candidateCard.evaluate(
      (element) => getComputedStyle(element).backgroundColor,
    ),
  ).toBe("rgb(24, 23, 19)");
  await expect(page.locator(".result-count")).toHaveCSS(
    "background-color",
    "rgba(222, 216, 204, 0.055)",
  );
  for (const pill of await page.locator(".media-pills span").all()) {
    await expect(pill).toHaveCSS(
      "background-color",
      "rgba(222, 216, 204, 0.055)",
    );
  }
});

test("Help documents the complete end-user contract", async ({ page }) => {
  await page.getByRole("button", { name: "Help", exact: true }).click();

  await expect(
    page.getByText(
      "Everything you need to turn a personal export or local archive into readable Markdown",
      { exact: false },
    ),
  ).toBeVisible();
  await expect(
    page.getByRole("heading", { name: "Sources you can use" }),
  ).toBeVisible();
  await expect(
    page.getByRole("heading", { name: "Why Unfold exists" }),
  ).toBeVisible();
  await expect(
    page.getByText("digital fenced gardens", { exact: false }),
  ).toBeVisible();
  await expect(
    page.getByText("Fully local. Zero outbound traffic.", { exact: true }),
  ).toBeVisible();
  await expect(
    page.getByRole("heading", {
      name: "Discover your digital traces. Understand what matters. Conserve your memories in open files.",
    }),
  ).toBeVisible();
  await expect(
    page.getByText("Discovery without surveillance", { exact: true }),
  ).toBeVisible();
  await expect(
    page.getByText("Seeing a trace never means you must conserve it", {
      exact: false,
    }),
  ).toBeVisible();
  await expect(
    page.getByRole("heading", { name: "Why use Amara Unfold?" }),
  ).toBeVisible();
  await expect(
    page.getByText("Find what you forgot was there", { exact: true }),
  ).toBeVisible();
  await expect(
    page.getByText("This is not metadata extractor number 27", {
      exact: true,
    }),
  ).toBeVisible();
  await expect(
    page.getByText("The goal is usable personal memory", { exact: false }),
  ).toBeVisible();
  await expect(
    page.getByRole("heading", {
      name: "What can you do with the information?",
    }),
  ).toBeVisible();
  await expect(
    page.getByText("Amara's suggestion: try it with a truly local LLM", {
      exact: true,
    }),
  ).toBeVisible();
  await expect(
    page.getByRole("heading", {
      name: "Why Amara Memory is more than Markdown plus an LLM",
    }),
  ).toBeVisible();
  await expect(
    page.getByText("Life becomes visible in layers", { exact: true }),
  ).toBeVisible();
  await expect(
    page.getByText("Connections stay evidence-based", { exact: true }),
  ).toBeVisible();
  await expect(
    page.getByText("Continue with Amara Memory", {
      exact: true,
    }),
  ).toBeVisible();
  await expect(
    page.getByText("Unfold is intended to prepare the way into Amara Memory", {
      exact: false,
    }),
  ).toBeVisible();
  await expect(
    page.getByRole("heading", {
      name: "For each successfully converted source",
    }),
  ).toBeVisible();
  await expect(
    page.getByText(
      "entity indexes for people, places, merchants, artists, and channels",
      { exact: false },
    ),
  ).toBeVisible();
  await expect(
    page.getByText("Apple Mail exported .mbox packages"),
  ).toBeVisible();
  await expect(
    page.getByText("Not available in this build", { exact: true }),
  ).toBeVisible();
  await expect(
    page.getByText("Apple Messages, Apple Notes, Apple Journal", {
      exact: false,
    }),
  ).toBeVisible();
  await expect(
    page.getByRole("heading", { name: "Future modules to build" }),
  ).toBeVisible();
  await expect(
    page.getByRole("heading", { name: "Podcasts", exact: true }),
  ).toBeVisible();
  await expect(
    page.getByText("podcast subscriptions and listening history", {
      exact: false,
    }),
  ).toBeVisible();
  await expect(
    page.getByRole("heading", { name: "Privacy and safe use" }),
  ).toBeVisible();
  await expect(
    page.getByRole("heading", { name: "Troubleshooting" }),
  ).toBeVisible();
  await expect(
    page.getByRole("heading", { name: "Frequently asked questions" }),
  ).toBeVisible();
  await expect(
    page.getByRole("heading", { name: "Build with us" }),
  ).toBeVisible();
  await expect(
    page.getByText("GNU Affero General Public License v3.0 or later", {
      exact: false,
    }),
  ).toBeVisible();
  await expect(
    page.getByText("New submodules are welcome", { exact: true }),
  ).toBeVisible();
  await expect(
    page.getByRole("link", { name: "info@amara.gmbh" }),
  ).toHaveAttribute("href", "mailto:info@amara.gmbh");

  const faq = page.locator("details").filter({
    hasText: "Does Unfold need internet access?",
  });
  await faq
    .getByText("Does Unfold need internet access?", { exact: true })
    .click();
  await expect(
    faq.getByText("Supported extraction, indexing, timeline generation", {
      exact: false,
    }),
  ).toBeVisible();

  await assertNoSeriousAccessibilityViolations(page);
  await page.getByRole("button", { name: "Start in Scan" }).click();
  await expect(
    page.getByRole("heading", { name: "Create open Markdown." }),
  ).toBeVisible();
});

test("complete user journey scans, persists, and exposes evidence", async ({
  page,
}) => {
  await page.goto("/?scenario=empty");
  await page.getByRole("button", { name: "Choose a folder" }).click();
  await expect(
    page.getByText("/Synthetic Archive/Jersey City", { exact: true }),
  ).toBeVisible();
  await expect(
    page.getByText(
      "Photo and video folder — Image and video files recognized locally.",
      { exact: false },
    ),
  ).toBeVisible();
  await page.getByRole("button", { name: "Choose output folder" }).click();
  await expect(
    page.getByText("/Synthetic/output", { exact: true }),
  ).toBeVisible();
  await page.getByRole("button", { name: "Create Markdown files" }).click();

  await expect(
    page.getByRole("status").filter({
      hasText: "Saved 333 open Markdown events",
    }),
  ).toBeVisible();
  await expect(
    page.getByRole("heading", { name: "Your Markdown is ready" }),
  ).toBeVisible();
  await expect(
    page.getByRole("region", { name: "Export quality" }),
  ).toBeVisible();
  await page.getByRole("button", { name: "Home", exact: true }).click();
  await expect(
    page.getByRole("heading", {
      name: "Discover your digital traces. Understand what matters. Conserve your memories in open files.",
    }),
  ).toBeVisible();
  await expect(page.getByText("333", { exact: true }).first()).toBeVisible();

  await page.getByRole("button", { name: "All events", exact: true }).click();
  await expect(
    page.getByRole("heading", { name: "All events", exact: true }),
  ).toBeVisible();
  await expect(page.getByText("333", { exact: true }).first()).toBeVisible();
  await page.getByRole("button", { name: /^2019 / }).click();
  await page.getByRole("button", { name: "August 2 events" }).click();
  const exportedEvent = page
    .locator(".timeline-event-card")
    .filter({ hasText: "Jersey City photo event" });
  await expect(exportedEvent).toBeVisible();
  await expect(
    exportedEvent.getByText(
      "A Jersey City photo recorded on August 30, 2019.",
      {
        exact: false,
      },
    ),
  ).toBeVisible();
  const previewFact = exportedEvent.locator(".event-preview .event-fact");
  await expect(previewFact).toHaveText("Jersey City photo");
  expect(
    await exportedEvent.locator(".event-preview").evaluate((element) => ({
      copy: getComputedStyle(element).color,
      fact: getComputedStyle(element.querySelector(".event-fact")!).color,
    })),
  ).toEqual({
    copy: "rgb(222, 216, 204)",
    fact: "rgb(250, 249, 245)",
  });
  await exportedEvent.getByText("Read complete event").click();
  await expect(
    exportedEvent.getByText("The complete event body stays readable here", {
      exact: false,
    }),
  ).toBeVisible();
  await expect(exportedEvent.locator(".event-content .event-fact")).toHaveText(
    "Jersey City photo",
  );
  await expect(
    exportedEvent.getByTitle("/Synthetic Archive/Jersey City/IMG_4021.jpg"),
  ).toBeHidden();
  await exportedEvent.getByText("Evidence", { exact: true }).click();
  await expect(
    exportedEvent.getByTitle("/Synthetic Archive/Jersey City/IMG_4021.jpg"),
  ).toBeVisible();

  await page.getByRole("button", { name: "Photo drafts", exact: true }).click();
  await expect(
    page.getByRole("heading", { name: "August 2019" }),
  ).toBeVisible();
  await expect(
    page.getByRole("heading", { name: "Photo event drafts" }),
  ).toBeVisible();
  const memoryToggle = page.getByRole("button", {
    name: /Expand memory.*Jersey City/,
  });
  await expect(memoryToggle).toHaveAttribute("aria-expanded", "false");
  await expect(page.getByText("What Unfold can say")).toBeHidden();
  await memoryToggle.click();
  await expect(
    page.getByRole("button", {
      name: /Collapse memory.*Jersey City/,
    }),
  ).toHaveAttribute("aria-expanded", "true");
  await expect(page.getByText("What Unfold can say")).toBeVisible();
  await expect(page.getByText("Observed from file metadata")).toBeVisible();
  await expect(
    page.getByText("Apple iPhone 11", { exact: true }),
  ).toBeVisible();
  await expect(page.getByText("One GPS area").first()).toBeVisible();
  await expect(page.locator(".memory-gallery")).toHaveCount(0);
  await expect(
    page.getByText("schema version", { exact: false }),
  ).toBeVisible();
  await expect(page.getByText(/BLAKE3 synthetic-/)).toBeVisible();
  await expect(
    page
      .getByRole("article")
      .locator(".evidence-list")
      .getByText("IMG_4021.jpg", { exact: true }),
  ).toBeVisible();
  await page.getByRole("button", { name: "Export open Markdown" }).click();
  await expect(
    page.getByRole("status").filter({
      hasText: "Saved 2 open Markdown event files",
    }),
  ).toBeVisible();

  await page.getByRole("button", { name: "Home", exact: true }).click();
  await expect(
    page.getByRole("heading", {
      name: "Discover your digital traces. Understand what matters. Conserve your memories in open files.",
    }),
  ).toBeVisible();
  await expect(page.getByText("Scan more")).toBeVisible();
});

test("Airbnb completion discloses full category accounting and skip reasons", async ({
  page,
}) => {
  await page.goto("/?scenario=airbnb-coverage");
  await page.getByRole("button", { name: "Choose a folder" }).click();
  await expect(
    page.getByRole("heading", { name: "Airbnb data export" }),
  ).toBeVisible();
  await expect(
    page.getByText("Airbnb JSON directory", { exact: true }),
  ).toBeVisible();
  await page.getByRole("button", { name: "Choose output folder" }).click();
  await page.getByRole("button", { name: "Create Markdown files" }).click();

  await expect(
    page.getByRole("status").filter({
      hasText:
        "149 open Markdown events; 1,510 records were inspected but not added to Timeline",
    }),
  ).toBeVisible();
  const quality = page.getByRole("region", { name: "Export quality" });
  await expect(quality).toBeVisible();
  await quality.getByText("Airbnb Json Takeout").click();
  await expect(quality).toContainText("Skipped with reason1,510");
  await expect(quality).toContainText("Why records were not written");
  await expect(quality).toContainText("Booking Session Supporting Metadata49");
  await expect(quality).toContainText(
    "Not Timeline Material: Search History751",
  );
  await expect(quality).toContainText("Accounting balances");
  await expect(quality).toContainText("skip reasons balance");
});

test("extracts every structured route from a multi-service takeout", async ({
  page,
}) => {
  await page.goto("/?scenario=multi-takeout");
  await page.getByRole("button", { name: "Choose a folder" }).click();
  await page.getByRole("button", { name: "Choose output folder" }).click();
  const batch = page.getByRole("button", { name: "Create all 3 exports" });
  await expect(batch).toBeVisible();
  await batch.click();
  await expect(
    page.getByRole("status").filter({
      hasText:
        "Done: saved 999 new Markdown events across 3 recognized exports to",
    }),
  ).toBeVisible();
});

test("All events restores sibling Diary and Airbnb archives with provider-first details", async ({
  page,
}) => {
  await page.goto("/?scenario=multi-archive");
  await page.getByRole("button", { name: "Choose a folder" }).click();
  await page.getByRole("button", { name: "Choose output folder" }).click();
  await page.getByRole("button", { name: "Create Markdown files" }).click();
  await page.getByRole("button", { name: "All events", exact: true }).click();

  const layers = page.getByText("2 active archive layers · 3 providers");
  await expect(layers).toBeVisible();
  const filterGeometry = await page
    .locator(".event-filters")
    .evaluate((toolbar) => {
      const columns = [
        ...toolbar.querySelectorAll<HTMLElement>(".event-filter"),
      ];
      const controls = [
        toolbar.querySelector<HTMLElement>(".event-search"),
        ...toolbar.querySelectorAll<HTMLElement>(".event-filter select"),
      ].filter((element): element is HTMLElement => Boolean(element));
      const toolbarRect = toolbar.getBoundingClientRect();
      const columnRects = columns.map((column) =>
        column.getBoundingClientRect(),
      );
      const controlRects = controls.map((control) =>
        control.getBoundingClientRect(),
      );
      return {
        topSpread:
          Math.max(...columnRects.map((rect) => rect.top)) -
          Math.min(...columnRects.map((rect) => rect.top)),
        bottomSpread:
          Math.max(...controlRects.map((rect) => rect.bottom)) -
          Math.min(...controlRects.map((rect) => rect.bottom)),
        outerBalance: Math.abs(
          columnRects[0].top -
            toolbarRect.top -
            (toolbarRect.bottom - controlRects[0].bottom),
        ),
      };
    });
  expect(filterGeometry.topSpread).toBeLessThanOrEqual(1);
  expect(filterGeometry.bottomSpread).toBeLessThanOrEqual(1);
  expect(filterGeometry.outerBalance).toBeLessThanOrEqual(1);
  await expect(page.locator(".event-year-number").first()).toBeVisible();
  expect(
    await page
      .locator(".event-year-number")
      .first()
      .evaluate((element) => getComputedStyle(element).fontFamily),
  ).toContain("Segoe UI");
  await expect(
    page.getByRole("option", { name: "Airbnb", exact: true }),
  ).toHaveCount(1);
  await expect(
    page.getByRole("option", { name: "Diary", exact: true }),
  ).toHaveCount(1);

  await page.getByRole("button", { name: /^2019 / }).click();
  await page.getByRole("button", { name: "August 3 events" }).click();
  const airbnb = page
    .locator(".timeline-event-card")
    .filter({ hasText: "Airbnb stay in Jersey City" });
  const diary = page
    .locator(".timeline-event-card")
    .filter({ hasText: "A day worth remembering" });
  await expect(airbnb).toBeVisible();
  await expect(diary).toBeVisible();
  await expect(airbnb.locator(".candidate-kicker")).toHaveText(
    "Airbnb · Stay · Location",
  );
  await expect(diary.locator(".candidate-kicker")).toHaveText(
    "Diary · Diary Entry",
  );
  await diary.getByText("Read complete event").click();
  await expect(diary.locator(".event-content pre")).toBeVisible();
  await expect(diary.locator(".event-content pre")).toContainText(
    "The complete diary entry remains readable inside the chronological archive.",
  );
  const evidence = diary.locator(".event-evidence summary");
  const completeEvent = diary.locator(".event-content summary");
  await expect(evidence).toBeVisible();
  expect(
    await evidence.evaluate((element) => getComputedStyle(element).fontSize),
  ).toBe(
    await completeEvent.evaluate(
      (element) => getComputedStyle(element).fontSize,
    ),
  );

  const referenceMetric = page.locator(".metric-card").filter({
    hasText: "Reference notes",
  });
  await expect(referenceMetric).toContainText("1");
  await expect(
    page.locator(".metric-card").filter({
      has: page.locator(".metric-label", { hasText: /^All events$/ }),
    }),
  ).toContainText("5");
  await expect(
    page.locator(".metric-card").filter({
      has: page.locator(".metric-label", { hasText: /^Dated$/ }),
    }),
  ).toContainText("3");
  await expect(
    page.locator(".metric-card").filter({ hasText: "Undated" }),
  ).toContainText("1");
  await expect(
    page.locator(".metric-card").filter({ hasText: "Months" }),
  ).toContainText("1");
  await page.getByRole("button", { name: /Timeless reference notes/ }).click();
  const reference = page
    .locator(".timeline-event-card")
    .filter({ hasText: "A timeless diary reference" });
  await expect(reference).toContainText("Timeless");
  await expect(reference).toContainText("Reference note");

  await layers.click();
  await expect(page.getByText("Diary archive", { exact: true })).toBeVisible();
  await expect(page.getByText("Airbnb archive", { exact: true })).toBeVisible();
  await page
    .getByRole("button", { name: "Remove Diary archive from All events" })
    .click();
  await expect(
    page.getByRole("status").filter({
      hasText:
        "Archive layer removed from All events. Its Markdown files and original source were not deleted.",
    }),
  ).toBeVisible();
  await expect(
    page.getByRole("option", { name: "Diary", exact: true }),
  ).toHaveCount(0);
  await expect(
    page.getByRole("option", { name: "Airbnb", exact: true }),
  ).toHaveCount(1);
  await expect(
    page.getByText("1 active archive layer · 2 providers"),
  ).toBeVisible();

  await page.getByRole("button", { name: "Home", exact: true }).click();
  await expect(
    page.getByRole("heading", {
      name: "Discover your digital traces. Understand what matters. Conserve your memories in open files.",
    }),
  ).toBeVisible();
  await expect(page.getByText(/open events ready/)).toHaveCount(0);
  await expect(page.locator(".timeline-home-preview code")).toHaveCount(0);
  const homeCardGeometry = await page
    .locator(".home-columns")
    .evaluate((grid) => {
      const cards = [...grid.querySelectorAll<HTMLElement>(":scope > .panel")];
      const rects = cards.map((card) => card.getBoundingClientRect());
      const preview = grid
        .querySelector<HTMLElement>(".timeline-home-preview")
        ?.getBoundingClientRect();
      return {
        widthSpread: Math.abs(rects[0].width - rects[1].width),
        heightSpread: Math.abs(rects[0].height - rects[1].height),
        tallestCard: Math.max(...rects.map((rect) => rect.height)),
        previewHeight: preview?.height ?? 0,
      };
    });
  expect(homeCardGeometry.widthSpread).toBeLessThanOrEqual(1);
  expect(homeCardGeometry.heightSpread).toBeLessThanOrEqual(1);
  expect(homeCardGeometry.tallestCard).toBeLessThan(200);
  expect(homeCardGeometry.previewHeight).toBeLessThan(80);
});

test("Timeline starts collapsed and keeps a single-open year accordion", async ({
  page,
}) => {
  await page.goto("/?scenario=year-accordion");
  await page.getByRole("button", { name: "Choose a folder" }).click();
  await page.getByRole("button", { name: "Choose output folder" }).click();
  await page.getByRole("button", { name: "Create Markdown files" }).click();
  await page.getByRole("button", { name: "All events", exact: true }).click();

  const newest = page.getByRole("button", { name: /^2026 / });
  const middle = page.getByRole("button", { name: /^2025 / });
  const oldest = page.getByRole("button", { name: /^2024 / });
  await expect(newest).toHaveAttribute("aria-expanded", "false");
  await expect(middle).toHaveAttribute("aria-expanded", "false");
  await expect(oldest).toHaveAttribute("aria-expanded", "false");

  await newest.click();
  await expect(newest).toHaveAttribute("aria-expanded", "true");
  await expect(middle).toHaveAttribute("aria-expanded", "false");
  await expect(oldest).toHaveAttribute("aria-expanded", "false");

  await middle.click();
  await expect(newest).toHaveAttribute("aria-expanded", "false");
  await expect(middle).toHaveAttribute("aria-expanded", "true");
  await expect(oldest).toHaveAttribute("aria-expanded", "false");

  await middle.click();
  await expect(newest).toHaveAttribute("aria-expanded", "false");
  await expect(middle).toHaveAttribute("aria-expanded", "false");
  await expect(oldest).toHaveAttribute("aria-expanded", "false");
});

test("manually dates, corrects, and undoes a genuinely undated event", async ({
  page,
}) => {
  await page.goto("/?scenario=empty");
  await page.getByRole("button", { name: "Choose a folder" }).click();
  await page.getByRole("button", { name: "Choose output folder" }).click();
  await page.getByRole("button", { name: "Create Markdown files" }).click();
  await page.getByRole("button", { name: "All events", exact: true }).click();

  await page.getByRole("button", { name: /Undated — review queue/ }).click();
  let event = page.locator(".timeline-event-card").filter({
    has: page.getByRole("heading", {
      name: "Undated archive event",
      exact: true,
    }),
  });
  await event.getByRole("button", { name: "Add a date" }).click();
  await event.getByLabel("Assigned date").fill("2024-05-17");
  await event.getByLabel("Assigned time").fill("08:30");
  await event.getByRole("button", { name: "Add to Timeline" }).click();

  await expect(page.getByRole("status")).toContainText(
    "stable event ID, raw source, and original evidence were preserved",
  );
  await expect(
    page.locator(".metric-card").filter({ hasText: "Undated" }),
  ).toContainText("2");
  await page.getByRole("button", { name: /^2024 / }).click();
  await page.getByRole("button", { name: /^May / }).click();
  event = page.locator(".timeline-event-card").filter({
    has: page.getByRole("heading", {
      name: "Undated archive event",
      exact: true,
    }),
  });
  await expect(event).toContainText("May 17, 2024 · 08:30");
  await expect(event).toContainText("Manually dated");

  await event.getByRole("button", { name: "Edit assigned date" }).click();
  await event.getByLabel("Precision").selectOption("month");
  await event.getByLabel("Assigned date").fill("2023-11");
  await event.getByRole("button", { name: "Save correction" }).click();

  await page.getByRole("button", { name: /^2023 / }).click();
  await page.getByRole("button", { name: /^November / }).click();
  event = page.locator(".timeline-event-card").filter({
    has: page.getByRole("heading", {
      name: "Undated archive event",
      exact: true,
    }),
  });
  await expect(event).toContainText("November 2023");
  await event.getByRole("button", { name: "Edit assigned date" }).click();
  await event.getByRole("button", { name: "Undo assignment" }).click();

  await expect(page.getByRole("status")).toContainText(
    "returned to the undated review queue",
  );
  await expect(
    page.locator(".metric-card").filter({ hasText: "Undated" }),
  ).toContainText("3");
});

test("lists local apps and enables ready local-app export routes", async ({
  page,
}) => {
  await page.goto("/?scenario=local-apps");
  await page.getByRole("button", { name: "Scan", exact: true }).click();

  await expect(
    page.getByRole("heading", { name: "Choose raw data" }),
  ).toBeVisible();
  await expect(
    page.getByText(
      "Choose one raw export file or a folder containing an export.",
    ),
  ).toBeVisible();
  await expect(
    page.getByText(
      "Supported now: Diary DOCX · Banking Takeout · Airbnb Takeout · Google Maps",
    ),
  ).toBeVisible();
  await expect(
    page
      .locator(".next-action-card")
      .getByText("Supported now:", { exact: false }),
  ).toBeVisible();
  await expect(
    page
      .locator(".next-action-card")
      .getByText(
        "Accepted files: DOCX, CSV, spreadsheet, GeoJSON and KMZ. For Airbnb, choose the extracted Takeout folder.",
      ),
  ).toBeVisible();
  const rawSupportLines = page.locator(".raw-support-note p");
  await expect(rawSupportLines).toHaveCount(2);
  const rawSupportStyles = await rawSupportLines.evaluateAll((lines) =>
    lines.map((line) => {
      const style = getComputedStyle(line);
      return {
        color: style.color,
        fontSize: style.fontSize,
        fontWeight: style.fontWeight,
        opacity: style.opacity,
      };
    }),
  );
  expect(rawSupportStyles[1]).toEqual(rawSupportStyles[0]);
  const rawDescriptionStyle = await page
    .getByText("Choose one raw export file or a folder containing an export.")
    .evaluate((line) => {
      const style = getComputedStyle(line);
      return {
        color: style.color,
        fontSize: style.fontSize,
        fontWeight: style.fontWeight,
        opacity: style.opacity,
      };
    });
  expect(rawSupportStyles[0]).toEqual(rawDescriptionStyle);
  const rawFileRoute = page.getByRole("button", { name: "Choose raw file" });
  await expect(rawFileRoute).toBeVisible();
  await expect(
    page.getByRole("button", { name: "Choose raw folder" }),
  ).toBeVisible();
  const rawFolderRoute = page.getByRole("button", {
    name: "Choose raw folder",
  });
  await expect(
    page.getByRole("button", { name: "Choose application" }),
  ).toHaveCount(0);
  await expect(rawFileRoute).toHaveClass(/primary-button/);
  await expect(rawFolderRoute).toHaveClass(/secondary-button/);
  const rawFileBox = await rawFileRoute.boundingBox();
  const rawFolderBox = await rawFolderRoute.boundingBox();
  expect(rawFolderBox?.height).toBe(rawFileBox?.height);
  expect(rawFolderBox?.width).toBe(rawFileBox?.width);
  const rawDataHeading = page.getByRole("heading", {
    name: "Choose raw data",
  });
  const applicationHeading = page.getByRole("heading", {
    name: "Choose from an application",
  });
  await expect(applicationHeading).toBeVisible();
  await expect(page.getByText("Show applications")).toBeVisible();
  const routeGeometry = await page.evaluate(() => {
    const rawCard = document.querySelector<HTMLElement>(".next-action-card");
    const applicationCard =
      document.querySelector<HTMLElement>(".local-app-intake");
    const rawIcon = rawCard?.querySelector<HTMLElement>(".next-action-icon");
    const applicationIcon =
      applicationCard?.querySelector<HTMLElement>(".next-action-icon");
    const rawHeading = rawCard?.querySelector<HTMLElement>("h2");
    const applicationHeading =
      applicationCard?.querySelector<HTMLElement>("h2");
    const rawHeadingStyle = rawHeading && getComputedStyle(rawHeading);
    const applicationHeadingStyle =
      applicationHeading && getComputedStyle(applicationHeading);
    return {
      rawHeight: rawCard?.getBoundingClientRect().height ?? 0,
      applicationHeight: applicationCard?.getBoundingClientRect().height ?? 0,
      cardHeightDifference: Math.abs(
        (rawCard?.getBoundingClientRect().height ?? 0) -
          (applicationCard?.getBoundingClientRect().height ?? 0),
      ),
      rawIconSize: rawIcon
        ? [
            rawIcon.getBoundingClientRect().width,
            rawIcon.getBoundingClientRect().height,
          ]
        : [],
      applicationIconSize: applicationIcon
        ? [
            applicationIcon.getBoundingClientRect().width,
            applicationIcon.getBoundingClientRect().height,
          ]
        : [],
      rawHeading: rawHeadingStyle
        ? [
            rawHeadingStyle.fontFamily,
            rawHeadingStyle.fontSize,
            rawHeadingStyle.fontWeight,
          ]
        : [],
      applicationHeading: applicationHeadingStyle
        ? [
            applicationHeadingStyle.fontFamily,
            applicationHeadingStyle.fontSize,
            applicationHeadingStyle.fontWeight,
          ]
        : [],
    };
  });
  expect(routeGeometry.cardHeightDifference).toBeLessThanOrEqual(1);
  expect(routeGeometry.applicationIconSize).toEqual(routeGeometry.rawIconSize);
  expect(routeGeometry.applicationHeading).toEqual(routeGeometry.rawHeading);
  await applicationHeading.click();
  await expect(page.getByText("Hide applications")).toBeVisible();
  const availableApplications = page.getByRole("button", {
    name: /^Available now/,
  });
  const futureApplications = page.getByRole("button", {
    name: /^Not available yet/,
  });
  await expect(availableApplications).toBeVisible();
  await expect(availableApplications).toHaveAttribute("aria-pressed", "true");
  await expect(futureApplications).toBeVisible();
  await expect(futureApplications).toHaveAttribute("aria-pressed", "false");
  const availableBox = await availableApplications.boundingBox();
  const futureBox = await futureApplications.boundingBox();
  expect(availableBox?.height).toBe(futureBox?.height);
  expect(availableBox?.width).toBe(futureBox?.width);
  await expect(page.locator(".future-app-list")).toHaveCount(0);
  await expect(
    page.getByText("More local sources:", { exact: false }),
  ).toHaveCount(0);
  await expect(
    page.getByRole("button", { name: "Refresh app exports" }),
  ).toBeVisible();
  await expect(
    page.getByRole("heading", { name: "Apple Music" }),
  ).toBeVisible();
  await expect(page.getByRole("heading", { name: "Immich" })).toHaveCount(0);
  await expect(page.getByRole("heading", { name: "Apple Mail" })).toBeVisible();
  const mailCard = page
    .locator(".local-app-card")
    .filter({ has: page.getByRole("heading", { name: "Apple Mail" }) });
  await expect(mailCard.getByText("Export found")).toBeVisible();
  await expect(mailCard.getByText("Needs permission")).not.toBeVisible();
  await expect(mailCard.getByText("1. Export in Mail")).toBeHidden();
  await page
    .locator(".local-app-card")
    .filter({ has: page.getByRole("heading", { name: "Apple Mail" }) })
    .getByText("How access works")
    .click();
  await expect(mailCard.getByText("1. Export in Mail")).toBeVisible();
  await expect(mailCard.getByText("2. Save .mbox")).toBeVisible();
  await expect(mailCard.getByText("3. Choose Mail export")).toBeVisible();
  await expect(
    page.getByText("Click Prepare Mail export to open Mail", {
      exact: false,
    }),
  ).toBeVisible();
  await expect(
    page.getByText("save the exported .mbox folder there", {
      exact: false,
    }),
  ).toBeVisible();
  await expect(
    page.getByText("Return here and click Use found Mail export", {
      exact: false,
    }),
  ).toBeVisible();
  await expect(
    page
      .locator(".local-app-card")
      .filter({ has: page.getByRole("heading", { name: "Apple Mail" }) })
      .getByText("Mailbox › Export Mailbox", { exact: false }),
  ).toHaveCount(1);
  await expect(
    page
      .locator(".local-app-card")
      .filter({ has: page.getByRole("heading", { name: "Apple Mail" }) })
      .getByRole("button", { name: "Use found Mail export" }),
  ).toBeVisible();
  await page.getByRole("button", { name: "Refresh app exports" }).click();
  await expect(
    page.getByRole("status").filter({
      hasText: "Apple Mail export found",
    }),
  ).toBeVisible();
  await expect(
    page
      .locator(".local-app-card")
      .filter({ has: page.getByRole("heading", { name: "Apple Mail" }) })
      .getByRole("button", { name: "Prepare Mail export" }),
  ).toBeVisible();
  await page
    .locator(".local-app-card")
    .filter({ has: page.getByRole("heading", { name: "Apple Mail" }) })
    .getByRole("button", { name: "Prepare Mail export" })
    .click();
  await expect(
    page.getByRole("status").filter({
      hasText: "Amara Unfold Mail Exports",
    }),
  ).toBeVisible();
  await expect(futureApplications).toBeVisible();
  await expect(
    page.getByText("Read Apple Calendar .ics events locally", {
      exact: false,
    }),
  ).toBeVisible();
  await expect(
    page.getByRole("heading", { name: "Apple Calendar" }),
  ).toBeVisible();
  await expect(
    page.getByRole("heading", { name: "Apple Contacts" }),
  ).toBeVisible();

  await page
    .locator(".local-app-card")
    .filter({ has: page.getByRole("heading", { name: "Apple Calendar" }) })
    .getByRole("button", {
      name: /Choose Calendar export|Choose data folder|Use .+|Choose folder/,
    })
    .click();
  await expect(
    page.getByRole("heading", { name: "Apple Calendar local calendars" }),
  ).toBeVisible();
  await expect(
    page
      .locator(".recognition-announcement")
      .filter({ hasText: "Local iCalendar (.ics) files" }),
  ).toBeVisible();
  const changeSource = page.getByRole("button", { name: "Change source" });
  await expect(changeSource).toHaveClass(/secondary-button/);
  await expect(changeSource).toHaveCSS("border-radius", "4px");
  await expect(changeSource).toHaveCSS("height", "38px");
  await changeSource.click();
  await expect(
    page.getByRole("heading", { name: "Choose raw data" }),
  ).toBeVisible();
  await page.getByText("Choose from an application").click();
  await expect(
    page.getByRole("heading", { name: "Apple Calendar" }),
  ).toBeVisible();

  await page.goto("/?scenario=local-apps");
  await page.getByRole("button", { name: "Scan", exact: true }).click();
  await page.getByText("Choose from an application").click();
  await page
    .locator(".local-app-card")
    .filter({ has: page.getByRole("heading", { name: "Apple Contacts" }) })
    .getByRole("button", {
      name: /Choose Contacts export|Choose data folder|Use .+|Choose folder/,
    })
    .click();
  await expect(
    page.getByRole("heading", { name: "Apple Contacts local cards" }),
  ).toBeVisible();
  await expect(
    page
      .locator(".recognition-announcement")
      .filter({ hasText: "Local vCard (.vcf) files" }),
  ).toBeVisible();

  await page.goto("/?scenario=local-apps");
  await page.getByRole("button", { name: "Scan", exact: true }).click();
  await page.getByText("Choose from an application").click();
  await page
    .locator(".local-app-card")
    .filter({ has: page.getByRole("heading", { name: "Apple Music" }) })
    .getByText("How access works")
    .click();
  await expect(
    page.getByText(
      "macOS may ask for Music/Automation access only when you create the export.",
      {
        exact: false,
      },
    ),
  ).toBeVisible();
  await page
    .locator(".local-app-card")
    .filter({ has: page.getByRole("heading", { name: "Apple Music" }) })
    .getByRole("button", { name: /Use Music library|Choose data folder/ })
    .click();
  await expect(
    page.getByRole("heading", { name: "Apple Music local library" }),
  ).toBeVisible();
  await expect(
    page.locator(".recognition-announcement").filter({
      hasText: "Music.app library metadata (one-time read)",
    }),
  ).toBeVisible();

  await page.goto("/?scenario=local-apps");
  await page.getByRole("button", { name: "Scan", exact: true }).click();
  await page.getByText("Choose from an application").click();
  await page
    .locator(".local-app-card")
    .filter({ has: page.getByRole("heading", { name: "Apple Mail" }) })
    .getByRole("button", { name: "Use found Mail export" })
    .click();
  await expect(
    page.getByRole("heading", { name: "Apple Mail exported mailbox" }),
  ).toBeVisible();
  await expect(
    page
      .locator(".recognition-announcement")
      .filter({ hasText: "Exported Apple Mail .mbox mailbox" }),
  ).toBeVisible();
  await page.getByRole("button", { name: "Choose output folder" }).click();
  await page.getByRole("button", { name: "Create Markdown files" }).click();
  await expect(
    page.getByRole("status").filter({
      hasText: "Saved 333 open Markdown events",
    }),
  ).toBeVisible();
  const progressPipeline = page.getByRole("region", {
    name: "Extraction progress",
  });
  await expect(progressPipeline).toHaveCount(0);
  await expect(
    page.getByRole("region", { name: "Export quality" }),
  ).toBeVisible();
});

test("local app access confirmation happens only when creating Markdown", async ({
  page,
}) => {
  await page.goto("/?scenario=local-apps");
  await page.getByRole("button", { name: "Scan", exact: true }).click();
  await page.getByText("Choose from an application").click();
  await page
    .locator(".local-app-card")
    .filter({ has: page.getByRole("heading", { name: "Apple Music" }) })
    .getByRole("button", { name: /Use Music library|Choose data folder/ })
    .click();

  await expect(
    page.getByRole("heading", { name: "Apple Music local library" }),
  ).toBeVisible();
  await expect(
    page.getByText(
      "Track metadata remains unread until you choose Create Markdown.",
      { exact: false },
    ),
  ).toBeVisible();
  await page.getByText("Recognition details").click();
  await expect(
    page.getByText(
      "Standard Music.app library selected; track metadata has not been read yet",
      { exact: false },
    ),
  ).toBeVisible();
  await expect(
    page.getByRole("dialog", { name: "Confirm local extraction" }),
  ).toHaveCount(0);

  await page.getByRole("button", { name: "Choose output folder" }).click();
  await page.evaluate(() => {
    Object.defineProperty(window, "__TAURI_INTERNALS__", {
      configurable: true,
      value: {},
    });
  });
  await page.getByRole("button", { name: "Create Markdown files" }).click();
  const consent = page.getByRole("dialog", {
    name: "Confirm local extraction",
  });
  await expect(consent).toBeVisible();
  await expect(consent).toContainText("one-time, read-only local Music export");
  await expect(consent).toContainText(
    "ask macOS to forget the Music Automation approval",
  );
  await expect(consent.getByRole("button", { name: "Cancel" })).toBeFocused();
  await consent.getByRole("button", { name: "Continue extraction" }).click();
  await expect(
    page.getByRole("status").filter({
      hasText: "Music Automation approval was released after this export",
    }),
  ).toBeVisible();
});

test("Apple Music reruns report retained files instead of a misleading zero-file result", async ({
  page,
}) => {
  await page.goto("/?scenario=music-rerun");
  await page.getByRole("button", { name: "Scan", exact: true }).click();
  await page.getByText("Choose from an application").click();
  await page
    .locator(".local-app-card")
    .filter({ has: page.getByRole("heading", { name: "Apple Music" }) })
    .getByRole("button", { name: /Use Music library|Choose data folder/ })
    .click();
  await page.getByRole("button", { name: "Choose output folder" }).click();
  await page.getByRole("button", { name: "Create Markdown files" }).click();

  await expect(
    page.getByText("No duplicate files were created", { exact: false }),
  ).toBeVisible();
  await expect(
    page.getByRole("status").filter({
      hasText: "17 unchanged daily Date Added Timeline files were retained",
    }),
  ).toBeVisible();
});

test("Apple Mail export route rejects a wrong folder before extraction", async ({
  page,
}) => {
  await page.goto("/?scenario=mail-wrong-export");
  await page.getByRole("button", { name: "Scan", exact: true }).click();
  await page.getByText("Choose from an application").click();

  const mailCard = page
    .locator(".local-app-card")
    .filter({ has: page.getByRole("heading", { name: "Apple Mail" }) });

  await mailCard.getByText("How access works").click();
  await mailCard.getByRole("button", { name: "Prepare Mail export" }).click();
  await expect(mailCard.getByText("Prepared folder")).toBeVisible();
  await expect(
    mailCard.getByRole("button", { name: "Open Mail + folder again" }),
  ).toBeVisible();
  await expect(
    mailCard.getByText("Waiting for a Mail export ending in .mbox."),
  ).toBeVisible();
  await expect(mailCard.getByText("No Full Disk Access needed")).toBeVisible();
  await expect(
    mailCard.getByText("Auto-checking this folder now."),
  ).toBeVisible();

  await mailCard.getByRole("button", { name: "Choose Mail export" }).click();

  await expect(
    page.getByRole("status").filter({
      hasText: "That does not look like an Apple Mail export",
    }),
  ).toBeVisible();
  await expect(
    page.getByRole("status").filter({
      hasText: "Mailbox › Export Mailbox",
    }),
  ).toBeVisible();
  await expect(
    page.getByRole("heading", { name: "Confirm format" }),
  ).not.toBeVisible();
});

test("implemented local apps expose clear folder or export routes", async ({
  page,
}) => {
  const implementedRoutes = [
    {
      card: "Apple Photos",
      heading: "Apple Photos exported media",
      evidence: "Exported Apple Photos originals or media folder",
      action: "Choose Photos export",
    },
    {
      card: "Apple Mail",
      heading: "Apple Mail exported mailbox",
      evidence: "Exported Apple Mail .mbox mailbox",
      action: "Use found Mail export",
    },
    {
      card: "Apple Music",
      heading: "Apple Music local library",
      evidence: "Music.app library metadata (one-time read)",
      action: "Use Music library",
    },
    {
      card: "Apple Calendar",
      heading: "Apple Calendar local calendars",
      evidence: "Local iCalendar (.ics) files",
      action: "Choose Calendar export",
    },
    {
      card: "Apple Contacts",
      heading: "Apple Contacts local cards",
      evidence: "Local vCard (.vcf) files",
      action: "Choose Contacts export",
    },
  ];

  for (const route of implementedRoutes) {
    await page.goto("/?scenario=local-apps");
    await page.getByRole("button", { name: "Scan", exact: true }).click();
    await page.getByText("Choose from an application").click();

    const routeCard = page
      .locator(".local-app-card")
      .filter({ has: page.getByRole("heading", { name: route.card }) });
    const action = routeCard.getByRole("button", { name: route.action });
    await expect(action).toBeEnabled();
    await action.click();
    await expect(
      page.getByRole("heading", { name: route.heading }),
    ).toBeVisible();
    await expect(
      page
        .locator(".recognition-announcement")
        .filter({ hasText: route.evidence }),
    ).toBeVisible();
    await page.getByText("Recognition details").click();
    await expect(
      page.getByText(
        /readable records found|Default app data location:|Selected app data location:|track metadata has not been read yet/,
      ),
    ).toBeVisible();
  }

  await page.goto("/?scenario=local-apps");
  await page.getByRole("button", { name: "Scan", exact: true }).click();
  await page.getByText("Choose from an application").click();
  await expect(page.getByText("Adapter pending")).toHaveCount(0);
  await expect(
    page.getByText("This local source is not available in this build yet"),
  ).toHaveCount(0);
  await expect(page.getByRole("button", { name: "Coming later" })).toHaveCount(
    0,
  );
  await expect(
    page.getByRole("button", { name: "Not in this build" }),
  ).toHaveCount(0);
  await page.getByRole("button", { name: /^Not available yet/ }).click();
  for (const pendingRoute of [
    "Immich",
    "Apple Messages / iMessage",
    "Apple Notes",
    "Apple Journal",
  ]) {
    await expect(page.locator(".future-app-list")).toContainText(pendingRoute);
  }
});

test("cancellation is explicit and preserves the last complete state", async ({
  page,
}) => {
  await page.goto("/?scenario=slow-cancel");
  await page.getByRole("button", { name: "Choose a folder" }).click();
  await page.getByRole("button", { name: "Choose output folder" }).click();
  await page.getByRole("button", { name: "Create Markdown files" }).click();
  await expect(
    page.getByRole("button", { name: "Stop extraction" }),
  ).toBeVisible();
  await page.getByRole("button", { name: "Stop extraction" }).click();
  await expect(
    page.getByRole("status").filter({
      hasText: "No new files were committed; previous exports are unchanged",
    }),
  ).toBeVisible();
  await page.getByRole("button", { name: "Home", exact: true }).click();
  await expect(page.getByText("No folders indexed")).toBeVisible();
});

test("scan failures are visible and locally auditable", async ({ page }) => {
  await page.goto("/?scenario=scan-error");
  await page.getByRole("button", { name: "Choose a folder" }).click();
  await page.getByRole("button", { name: "Choose output folder" }).click();
  await page.getByRole("button", { name: "Create Markdown files" }).click();
  await expect(
    page.getByRole("status").filter({ hasText: "could not be read" }),
  ).toBeVisible();
  await expect(page.getByText("Recent scan issues")).toBeVisible();
  await expect(
    page.getByTitle("/Synthetic Archive/locked: permission denied"),
  ).toBeVisible();
});

test("expanding a lower memory uses the page scroll and keeps its header visible", async ({
  page,
}) => {
  await page.goto("/?scenario=many-memories");
  await page.getByRole("button", { name: "Photo drafts", exact: true }).click();

  const toggle = page.getByRole("button", {
    name: /Expand memory.*Cluster 13/,
  });
  await toggle.evaluate((button: HTMLButtonElement) => button.click());

  const expandedToggle = page.getByRole("button", {
    name: /Collapse memory.*Cluster 13/,
  });
  await expect(expandedToggle).toHaveAttribute("aria-expanded", "true");
  await expect
    .poll(() =>
      page
        .locator(".workspace-scroll")
        .evaluate((element) => element.scrollTop),
    )
    .toBeGreaterThan(0);
  await expect(
    page.getByRole("scrollbar", { name: "Page position" }),
  ).toHaveClass(/visible/);

  const position = await expandedToggle.evaluate((button) => {
    const card = button.closest(".candidate-card")!;
    const scroller = button.closest(".workspace-scroll")!;
    const cardRect = card.getBoundingClientRect();
    const scrollerRect = scroller.getBoundingClientRect();
    return {
      cardTop: cardRect.top,
      scrollerTop: scrollerRect.top,
      overflowY: getComputedStyle(scroller).overflowY,
      nestedScrollableEvidence: getComputedStyle(
        card.querySelector(".evidence-panel")!,
      ).overflowY,
      scrollbarValue: Number(
        document
          .querySelector('[role="scrollbar"]')!
          .getAttribute("aria-valuenow"),
      ),
    };
  });
  expect(position.cardTop).toBeGreaterThanOrEqual(position.scrollerTop);
  expect(position.overflowY).toBe("auto");
  expect(position.nestedScrollableEvidence).not.toBe("auto");
  expect(position.scrollbarValue).toBeGreaterThan(0);
  const scrollbarAppearance = await page
    .getByRole("scrollbar", { name: "Page position" })
    .evaluate((element) => {
      const style = getComputedStyle(element);
      const scroller = document.querySelector(".workspace-scroll")!;
      return {
        width: style.width,
        background: style.backgroundColor,
        nativeWidth: getComputedStyle(scroller, "::-webkit-scrollbar").width,
      };
    });
  expect(scrollbarAppearance.width).toBe("6px");
  expect(scrollbarAppearance.background).toBe("rgba(0, 0, 0, 0)");
  expect(scrollbarAppearance.nativeWidth).toBe("0px");
});

test("all populated sections pass accessibility and overflow gates", async ({
  page,
}) => {
  await page.goto("/?scenario=populated");
  for (const section of [
    "Home",
    "Scan",
    "All events",
    "Photo drafts",
    "Settings",
    "Help",
  ]) {
    await page.getByRole("button", { name: section, exact: true }).click();
    await assertNoSeriousAccessibilityViolations(page);
    const overflow = await page.evaluate(() => ({
      document:
        document.documentElement.scrollWidth -
        document.documentElement.clientWidth,
      body: document.body.scrollWidth - document.body.clientWidth,
    }));
    expect(overflow, `${section} must not overflow horizontally`).toEqual({
      document: 0,
      body: 0,
    });
    const undersizedText = await page.evaluate(() =>
      [...document.querySelectorAll<HTMLElement>(".view *")]
        .filter((element) => {
          const style = getComputedStyle(element);
          return (
            element.childElementCount === 0 &&
            Boolean(element.textContent?.trim()) &&
            style.display !== "none" &&
            style.visibility !== "hidden"
          );
        })
        .map((element) => ({
          text: element.textContent?.trim().slice(0, 80),
          size: Number.parseFloat(getComputedStyle(element).fontSize),
        }))
        .filter(({ size }) => size < 12),
    );
    expect(
      undersizedText,
      `${section} must not contain microtext below 12px`,
    ).toEqual([]);
    if (section === "Home") {
      const candidatePreviewOverflow = await page.evaluate(() =>
        [...document.querySelectorAll<HTMLElement>(".candidate-mini")].flatMap(
          (item) => {
            const panel = item.closest<HTMLElement>(".candidates-preview");
            if (!panel) return [];
            const panelRect = panel.getBoundingClientRect();
            return [...item.querySelectorAll<HTMLElement>("*")]
              .map((element) => {
                const rect = element.getBoundingClientRect();
                return {
                  text: element.textContent?.trim().slice(0, 80),
                  left: rect.left,
                  right: rect.right,
                  panelLeft: panelRect.left,
                  panelRight: panelRect.right,
                };
              })
              .filter(
                ({ left, right, panelLeft, panelRight }) =>
                  left < panelLeft - 0.5 || right > panelRight + 0.5,
              );
          },
        ),
      );
      expect(
        candidatePreviewOverflow,
        "Home memory-draft preview rows must stay inside the card",
      ).toEqual([]);
    }
  }
});

async function assertNoSeriousAccessibilityViolations(
  page: Page,
): Promise<void> {
  const results = await new AxeBuilder({ page })
    .withTags(["wcag2a", "wcag2aa", "wcag21a", "wcag21aa"])
    .analyze();
  const serious = results.violations.filter(
    ({ impact }) => impact === "critical" || impact === "serious",
  );
  expect(
    serious,
    serious.map((item) => `${item.id}: ${item.help}`).join("\n"),
  ).toEqual([]);
}
