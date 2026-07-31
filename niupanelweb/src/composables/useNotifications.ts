import { LocalNotifications } from '@capacitor/local-notifications';
import { useAppStore } from '../stores/app';

export function useNotifications() {
  const appStore = useAppStore();

  const requestPermissions = async () => {
    if (!appStore.isMobile) return false;
    try {
      const status = await LocalNotifications.requestPermissions();
      return status.display === 'granted';
    } catch (e) {
      return false;
    }
  };

  const schedule = async (title: string, body: string, id: number = Math.floor(Math.random() * 100000)) => {
    if (!appStore.isMobile) return;

    const hasPermission = await requestPermissions();
    if (!hasPermission) return;

    try {
      await LocalNotifications.schedule({
        notifications: [
          {
            title,
            body,
            id,
            schedule: { at: new Date(Date.now() + 100) }, // Nearly immediate
            sound: 'default',
            attachments: [],
            actionTypeId: '',
            extra: null
          }
        ]
      });
    } catch (e) {
      console.error('Failed to schedule notification', e);
    }
  };

  return {
    requestPermissions,
    schedule
  };
}
