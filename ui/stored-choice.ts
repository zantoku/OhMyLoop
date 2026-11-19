// Per-computer UI preferences kept in local storage. A stored value is
// validated against the allowed choices; an unknown or unreadable value falls
// back to the default without being overwritten, so a newer version's choice
// survives running an older one.
export function readChoice<T extends string>(key: string, choices: readonly T[], fallback: T): T {
  try {
    const value = localStorage.getItem(key);
    return choices.find(choice => choice === value) ?? fallback;
  } catch {
    return fallback;
  }
}

/** Saves a choice; false when storage is unavailable, so callers can say so. */
export function writeChoice(key: string, value: string): boolean {
  try {
    localStorage.setItem(key, value);
    return true;
  } catch {
    return false;
  }
}
