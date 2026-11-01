// SPDX-License-Identifier: AGPL-3.0-or-later
import { useCallback, useEffect, useRef, useState } from "react";

export interface ExtractionConsent {
  description: string;
  sourcePath: string;
  destinationPath: string;
}

export function useExtractionConsent() {
  const [consent, setConsent] = useState<ExtractionConsent | null>(null);
  const pending = useRef<((accepted: boolean) => void) | null>(null);
  const requestConsent = useCallback((request: ExtractionConsent) => {
    // A second action must never replace or inherit an outstanding decision.
    if (pending.current) return Promise.resolve(false);
    return new Promise<boolean>((resolve) => {
      pending.current = resolve;
      setConsent(request);
    });
  }, []);
  const decideConsent = useCallback((accepted: boolean) => {
    const resolve = pending.current;
    pending.current = null;
    setConsent(null);
    resolve?.(accepted);
  }, []);
  useEffect(
    () => () => {
      pending.current?.(false);
      pending.current = null;
    },
    [],
  );
  return { consent, requestConsent, decideConsent };
}
