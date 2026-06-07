import { convertFileSrc, invoke } from '../adapters/tauri';
import type { AssetRef } from '../../shared/types/game-script';

export class AssetLoader {
  private cache: Map<string, string> = new Map(); // assetRefId -> data URL or accessible URL

  // 根据 AssetRef 加载本地资源
  async loadAsset(assetRef: AssetRef): Promise<string | undefined> {
    if (this.cache.has(assetRef.id)) {
      return this.cache.get(assetRef.id);
    }

    if (assetRef.url) {
      // 尝试用 data URL 方式（最可靠）
      try {
        const dataUrl = await invoke<string>('read_file_as_data_url', {
          filePath: assetRef.url
        });
        this.cache.set(assetRef.id, dataUrl);
        return dataUrl;
      } catch (e) {
        // 如果失败，回退到 convertFileSrc
        console.warn('Failed to load as data URL, falling back to convertFileSrc:', e);
        const resolvedUrl = this.resolveAssetUrl(assetRef.url);
        this.cache.set(assetRef.id, resolvedUrl);
        return resolvedUrl;
      }
    }

    return undefined;
  }

  // 批量预加载
  async preloadAssets(assetRefs: AssetRef[]): Promise<void> {
    await Promise.all(assetRefs.map(ref => this.loadAsset(ref)));
  }

  // 获取已缓存的资源 URL
  getCachedUrl(assetRefId: string): string | undefined {
    return this.cache.get(assetRefId);
  }

  // 外部设置缓存 URL（用于 asset-ready 事件更新）
  setCachedUrl(assetRefId: string, url: string): void {
    this.cache.set(assetRefId, url);
  }

  // 清除缓存
  clearCache(): void {
    this.cache.clear();
  }

  // 将本地路径转换为可访问的 URL（Tauri convertFileSrc）
  private resolveAssetUrl(localPath: string): string {
    try {
      return convertFileSrc(localPath);
    } catch {
      // 非 Tauri 环境降级为直接使用路径
      return localPath;
    }
  }
}
