// 静态导入 Tauri 模块（在 Tauri 环境时会被正确解析）
import {
  invoke as tauriInvoke,
  convertFileSrc as tauriConvertFileSrc,
} from '@tauri-apps/api/core';
import { listen as tauriListen } from '@tauri-apps/api/event';

export const isTauri = (): boolean => {
  return typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;
};

export const invoke = async <T = unknown>(cmd: string, args?: Record<string, unknown>): Promise<T> => {
  if (isTauri()) {
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
    return tauriListen(event, callback);
  }
  // Web 模式：占位实现
  return () => {};
};

export const convertFileSrc = (path: string): string => {
  if (isTauri()) {
    return tauriConvertFileSrc(path);
  }
  return path;
};
