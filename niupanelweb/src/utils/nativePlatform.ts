type CapacitorBridge = {
  getPlatform?: () => string;
  isNativePlatform?: () => boolean;
};

/**
 * Capacitor injects this small bridge before the app starts.  Reading it
 * avoids importing the Capacitor runtime (and every native plugin) in a
 * normal browser build.
 */
export const isNativePlatform = () => {
  if (typeof window === "undefined") return false;

  const bridge = (window as Window & { Capacitor?: CapacitorBridge }).Capacitor;
  if (bridge?.isNativePlatform) return bridge.isNativePlatform();

  const platform = bridge?.getPlatform?.();
  return platform === "android" || platform === "ios";
};

