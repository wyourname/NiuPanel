export const FRONTEND_VERSION = __APP_VERSION__;

export const formatVersion = (version?: string | null) => {
  const normalized = version?.trim();
  if (!normalized) return "未知";
  return normalized.toLowerCase().startsWith("v") ? normalized : `v${normalized}`;
};
