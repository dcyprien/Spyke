import { isPermissionGranted, requestPermission, sendNotification } from '@tauri-apps/api/notification';

export async function triggerSystemNotification(title: string, body: string) {
  // On s'assure d'être dans l'app Tauri (pas sur un simple navigateur)
  if (typeof window !== 'undefined' && window.__TAURI_IPC__) {
    try {
      let permissionGranted = await isPermissionGranted();
      if (!permissionGranted) {
        const permission = await requestPermission();
        permissionGranted = permission === 'granted';
      }

      if (permissionGranted) {
        sendNotification({ title, body });
      }
    } catch (error) {
      console.error("Erreur notification système:", error);
    }
  }
}