import type { ImpactStyle, NotificationType } from '@capacitor/haptics';
import { isNativePlatform } from '../utils/nativePlatform';

let hapticsModule: Promise<typeof import('@capacitor/haptics') | null> | null = null;

const loadHaptics = () => {
  if (!isNativePlatform()) return Promise.resolve(null);
  hapticsModule ??= import('@capacitor/haptics');
  return hapticsModule;
};

export function useHaptics() {
  const impact = async (style?: ImpactStyle) => {
    const module = await loadHaptics();
    if (!module) return;
    try {
      await module.Haptics.impact({ style: style ?? module.ImpactStyle.Light });
    } catch (e) {
      // Ignore if not on a real device or Capacitor not ready
    }
  };

  const notification = async (type?: NotificationType) => {
    const module = await loadHaptics();
    if (!module) return;
    try {
      await module.Haptics.notification({ type: type ?? module.NotificationType.Success });
    } catch (e) {}
  };

  const selectionStart = async () => {
    const module = await loadHaptics();
    if (!module) return;
    try {
      await module.Haptics.selectionStart();
    } catch (e) {}
  };

  const selectionChanged = async () => {
    const module = await loadHaptics();
    if (!module) return;
    try {
      await module.Haptics.selectionChanged();
    } catch (e) {}
  };

  const selectionEnd = async () => {
    const module = await loadHaptics();
    if (!module) return;
    try {
      await module.Haptics.selectionEnd();
    } catch (e) {}
  };

  const vibrate = async () => {
    const module = await loadHaptics();
    if (!module) return;
    try {
      await module.Haptics.vibrate();
    } catch (e) {}
  };

  return {
    impact,
    notification,
    selectionStart,
    selectionChanged,
    selectionEnd,
    vibrate
  };
}
