import { storageKey } from "./storage";

export function getPersistedOrderKey(scope: string): string {
  return storageKey(`ordered_ids_${scope}`);
}

export function loadPersistedOrder(scope: string): number[] {
  try {
    const raw = localStorage.getItem(getPersistedOrderKey(scope));
    if (!raw) return [];
    const parsed = JSON.parse(raw);
    if (!Array.isArray(parsed)) return [];
    return parsed.map((id) => Number(id)).filter((id) => Number.isFinite(id));
  } catch {
    return [];
  }
}

export function savePersistedOrder(scope: string, ids: number[]) {
  localStorage.setItem(getPersistedOrderKey(scope), JSON.stringify(ids));
}

export function applyPersistedOrder<T extends { id: number | null | undefined }>(
  items: T[],
  storedIds: number[],
): T[] {
  if (!storedIds.length) return [...items];

  const orderMap = new Map(storedIds.map((id, index) => [id, index]));
  return [...items].sort((a, b) => {
    const aIndex = a.id == null ? Number.MAX_SAFE_INTEGER : (orderMap.get(a.id) ?? Number.MAX_SAFE_INTEGER);
    const bIndex = b.id == null ? Number.MAX_SAFE_INTEGER : (orderMap.get(b.id) ?? Number.MAX_SAFE_INTEGER);

    if (aIndex !== bIndex) return aIndex - bIndex;
    return 0;
  });
}

export function moveItem<T>(items: T[], fromIndex: number, toIndex: number): T[] {
  const list = [...items];
  const [item] = list.splice(fromIndex, 1);
  list.splice(toIndex, 0, item);
  return list;
}
