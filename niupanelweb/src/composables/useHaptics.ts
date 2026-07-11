import { Haptics, ImpactStyle, NotificationType } from '@capacitor/haptics';
import { useAppStore } from '../stores/app';

export function useHaptics() {
  const appStore = useAppStore();

  const impact = async (style: ImpactStyle = ImpactStyle.Light) => {
    if (!appStore.isMobile) return;
    try {
      await Haptics.impact({ style });
    } catch (e) {
      // Ignore if not on a real device or Capacitor not ready
    }
  };

  const notification = async (type: NotificationType = NotificationType.Success) => {
    if (!appStore.isMobile) return;
    try {
      await Haptics.notification({ type });
    } catch (e) {}
  };

  const selectionStart = async () => {
    if (!appStore.isMobile) return;
    try {
      await Haptics.selectionStart();
    } catch (e) {}
  };

  const selectionChanged = async () => {
    if (!appStore.isMobile) return;
    try {
      await Haptics.selectionChanged();
    } catch (e) {}
  };

  const selectionEnd = async () => {
    if (!appStore.isMobile) return;
    try {
      await Haptics.selectionEnd();
    } catch (e) {}
  };

  const vibrate = async () => {
    if (!appStore.isMobile) return;
    try {
      await Haptics.vibrate();
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
