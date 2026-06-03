import { convertFileSrc } from '../adapters/tauri';
import type { AssetRef } from '../../shared/types/game-script';

export class AssetLoader {
  private cache: Map<string, string> = new Map(); // assetRefId -> localUrl

  // 根据 AssetRef 加载本地资源
  async loadAsset(assetRef: AssetRef): Promise<string | undefined> {
    if (this.cache.has(assetRef.id)) {
      return this.cache.get(assetRef.id);
    }

    if (assetRef.url) {
      const resolvedUrl = this.resolveAssetUrl(assetRef.url);
      this.cache.set(assetRef.id, resolvedUrl);
      return resolvedUrl;
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
