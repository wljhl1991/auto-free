// 预加载 Tauri 模块的 Promise，避免在非 Tauri 环境下导入错误
let tauriCoreModule: any = null;
let tauriEventModule: any = null;
let modulesLoaded: Promise<void> | null = null;

// 预加载 Tauri 模块（如果是 Tauri 环境
export const loadTauriModules = async (): Promise<void> => {
  if (modulesLoaded) return modulesLoaded;
  if (!isTauri()) {
    modulesLoaded = Promise.resolve();
    return modulesLoaded;
  }
  modulesLoaded = (async () => {
    try {
      tauriCoreModule = await import('@tauri-apps/api/core');
      tauriEventModule = await import('@tauri-apps/api/event');
    } catch (e) {
      console.warn('Failed to load Tauri modules:', e);
    }
  })();
  return modulesLoaded;
};

export const isTauri = (): boolean => {
  return typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;
};

export const invoke = async <T = unknown>(cmd: string, args?: Record<string, unknown>): Promise<T> => {
  if (isTauri()) {
    await loadTauriModules();
    if (tauriCoreModule?.invoke) {
      return tauriCoreModule.invoke(cmd, args);
    }
    // 兜底：如果加载失败，尝试直接动态导入
    const { invoke: fallbackInvoke } = await import('@tauri-apps/api/core');
    return fallbackInvoke(cmd, args);
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
    await loadTauriModules();
    if (tauriEventModule?.listen) {
      return tauriEventModule.listen(event, callback);
    }
    const { listen: fallbackListen } = await import('@tauri-apps/api/event');
    return fallbackListen(event, callback);
  }
  // Web 模式：占位实现
  return () => {};
};

export const convertFileSrc = (path: string): string => {
  if (isTauri()) {
    // 如果模块已经加载，直接用官方API
    if (tauriCoreModule?.convertFileSrc) {
      return tauriCoreModule.convertFileSrc(path);
    }
    // 如果模块没加载，先用临时回退方案，之后再通过asset-ready等更新
    try {
      const encodedPath = encodeURIComponent(path);
      return `asset://localhost/${encodedPath}`;
    } catch {
      return path;
    }
  }
  return path;
};
