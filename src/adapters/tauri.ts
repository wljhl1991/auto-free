export const isTauri = (): boolean => {
  return typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;
};

export const invoke = async <T = unknown>(cmd: string, args?: Record<string, unknown>): Promise<T> => {
  if (isTauri()) {
    const { invoke: tauriInvoke } = await import('@tauri-apps/api/core');
    return tauriInvoke<T>(cmd, args);
  }
  // Web 模式：调用后端 API
  const response = await fetch(`/api/${cmd}`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(args),
  });
  return response.json();
};

export const listen = async (event: string, callback: (event: any) => void): Promise<() => void> => {
  if (isTauri()) {
    const { listen: tauriListen } = await import('@tauri-apps/api/event');
    return tauriListen(event, callback);
  }
  // Web 模式：占位实现
  return () => {};
};

export const convertFileSrc = (path: string): string => {
  if (isTauri()) {
    return `asset://localhost/${encodeURIComponent(path)}`;
  }
  return path;
};
