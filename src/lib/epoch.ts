export function epochToDate(epoch: number): Date {
  if (Number.isNaN(epoch)) throw new Error("NaN epoch");
  if (epoch >= 2 ** 31) {
    return new Date(epoch);
  } else {
    return new Date(epoch * 1000);
  }
}

export function parseEpochText(text: string): number | null {
  const n = Number(text.trim());
  if (Number.isNaN(n)) return null;
  return n;
}

export function epochToString(epoch: number): string {
  return epochToDate(epoch).toString();
}

export function isValidEpochText(text: string): boolean {
  return parseEpochText(text) !== null;
}
