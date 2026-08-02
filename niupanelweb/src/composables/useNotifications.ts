import { isNativePlatform } from '../utils/nativePlatform';

let notificationsModule: Promise<typeof import('@capacitor/local-notifications') | null> | null = null;

const loadNotifications = () => {
  if (!isNativePlatform()) return Promise.resolve(null);
  notificationsModule ??= import('@capacitor/local-notifications');
  return notificationsModule;
};

export function useNotifications() {
  const requestPermissions = async () => {
    const module = await loadNotifications();
    if (!module) return false;
    try {
      const status = await module.LocalNotifications.requestPermissions();
      return status.display === 'granted';
    } catch (e) {
      return false;
    }
  };

  const schedule = async (title: string, body: string, id: number = Math.floor(Math.random() * 100000)) => {
    const hasPermission = await requestPermissions();
    if (!hasPermission) return;

    try {
      const module = await loadNotifications();
      if (!module) return;
      await module.LocalNotifications.schedule({
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
