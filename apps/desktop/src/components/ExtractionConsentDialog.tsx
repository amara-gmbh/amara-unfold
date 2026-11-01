// SPDX-License-Identifier: AGPL-3.0-or-later
import { useEffect, useRef } from "react";
import type { ExtractionConsent } from "../lib/useExtractionConsent";

export function ExtractionConsentDialog({
  consent,
  onDecision,
}: {
  consent: ExtractionConsent | null;
  onDecision: (accepted: boolean) => void;
}) {
  const dialogRef = useRef<HTMLDialogElement>(null);
  const cancelRef = useRef<HTMLButtonElement>(null);
  useEffect(() => {
    const dialog = dialogRef.current;
    if (!dialog || !consent) return;
    const opener = document.activeElement;
    dialog.showModal();
    cancelRef.current?.focus();
    return () => {
      dialog.close();
      if (opener instanceof HTMLElement && opener.isConnected) opener.focus();
    };
  }, [consent]);
  return (
    <dialog
      ref={dialogRef}
      className="extraction-consent"
      aria-labelledby="extraction-consent-title"
      aria-describedby="extraction-consent-description"
      onCancel={(event) => {
        event.preventDefault();
        onDecision(false);
      }}
    >
      <h2 id="extraction-consent-title">Confirm local extraction</h2>
      <p id="extraction-consent-description">{consent?.description}</p>
      <dl>
        <dt>Selected source</dt>
        <dd>{consent?.sourcePath}</dd>
        <dt>Destination</dt>
        <dd>{consent?.destinationPath}</dd>
      </dl>
      <div className="extraction-consent-actions">
        <button
          ref={cancelRef}
          type="button"
          className="secondary-button"
          onClick={() => onDecision(false)}
        >
          Cancel
        </button>
        <button
          type="button"
          className="primary-button"
          onClick={() => onDecision(true)}
        >
          Continue extraction
        </button>
      </div>
    </dialog>
  );
}
