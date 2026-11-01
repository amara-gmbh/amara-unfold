// @vitest-environment jsdom
// SPDX-License-Identifier: AGPL-3.0-or-later
import {
  act,
  cleanup,
  fireEvent,
  render,
  renderHook,
  screen,
} from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { useExtractionConsent } from "../lib/useExtractionConsent";
import { ExtractionConsentDialog } from "./ExtractionConsentDialog";
const request = {
  description: "Read this exported Calendar once. No upload.",
  sourcePath: "/synthetic/calendar",
  destinationPath: "/synthetic/output",
};
afterEach(cleanup);
beforeEach(() => {
  HTMLDialogElement.prototype.showModal = function () {
    this.setAttribute("open", "");
  };
  HTMLDialogElement.prototype.close = function () {
    this.removeAttribute("open");
  };
});
describe("local extraction consent", () => {
  it("does not authorize until Continue; refuses concurrent requests", async () => {
    const { result } = renderHook(useExtractionConsent);
    let decision!: Promise<boolean>;
    act(() => {
      decision = result.current.requestConsent(request);
    });
    const settled = vi.fn();
    void decision.then(settled);
    await act(async () => {
      await Promise.resolve();
    });
    expect(settled).not.toHaveBeenCalled();
    expect(
      await result.current.requestConsent({ ...request, sourcePath: "/other" }),
    ).toBe(false);
    expect(result.current.consent?.sourcePath).toBe(request.sourcePath);
    act(() => result.current.decideConsent(true));
    expect(await decision).toBe(true);
    expect(result.current.consent).toBeNull();
  });
  it("resolves refusal and unmount without granting consent", async () => {
    const { result, unmount } = renderHook(useExtractionConsent);
    let decision!: Promise<boolean>;
    act(() => {
      decision = result.current.requestConsent(request);
    });
    act(() => result.current.decideConsent(false));
    expect(await decision).toBe(false);
    act(() => {
      decision = result.current.requestConsent(request);
    });
    unmount();
    expect(await decision).toBe(false);
  });
  it("shows source, destination and privacy, focuses Cancel and restores focus", () => {
    const opener = document.createElement("button");
    document.body.append(opener);
    opener.focus();
    const decide = vi.fn();
    const { rerender } = render(
      <ExtractionConsentDialog consent={request} onDecision={decide} />,
    );
    expect(
      screen.getByRole("dialog", { name: "Confirm local extraction" }),
    ).toBeTruthy();
    expect(screen.getByText(request.sourcePath)).toBeTruthy();
    expect(screen.getByText(request.destinationPath)).toBeTruthy();
    expect(screen.getByText(request.description)).toBeTruthy();
    expect(document.activeElement).toBe(
      screen.getByRole("button", { name: "Cancel" }),
    );
    fireEvent.click(screen.getByRole("button", { name: "Cancel" }));
    expect(decide).toHaveBeenCalledWith(false);
    rerender(<ExtractionConsentDialog consent={null} onDecision={decide} />);
    expect(document.activeElement).toBe(opener);
    opener.remove();
  });
  it("Escape refuses and Continue is an explicit separate action", () => {
    const decide = vi.fn();
    render(<ExtractionConsentDialog consent={request} onDecision={decide} />);
    fireEvent(
      screen.getByRole("dialog"),
      new Event("cancel", { cancelable: true }),
    );
    expect(decide).toHaveBeenLastCalledWith(false);
    fireEvent.click(
      screen.getByRole("button", { name: "Continue extraction" }),
    );
    expect(decide).toHaveBeenLastCalledWith(true);
  });
});
