import { useState, useEffect, useCallback } from 'react';
import { useUserAsset, type UserAssetEntry } from '@/hooks/useUserAsset';
import { convertFileSrc } from '@/adapters/tauri';

const ASSET_TYPE_TABS: { key: string; label: string; accept: string }[] = [
  { key: 'image', label: '图片', accept: 'image/*' },
  { key: 'music', label: '音乐', accept: 'audio/*' },
  { key: 'video', label: '视频', accept: 'video/*' },
  { key: 'voice', label: '语音', accept: 'audio/*' },
];

function formatFileSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

function formatDate(timestamp: number): string {
  return new Date(timestamp * 1000).toLocaleDateString('zh-CN');
}

export default function UserAssetManager() {
  const userAsset = useUserAsset();
  const [activeTab, setActiveTab] = useState('image');
  const [assets, setAssets] = useState<UserAssetEntry[]>([]);
  const [loading, setLoading] = useState(false);
  const [importing, setImporting] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const loadAssets = useCallback(async () => {
    try {
      setLoading(true);
      setError(null);
      const list = await userAsset.listUserAssets(activeTab);
      setAssets(list || []);
    } catch (err) {
      console.error('Failed to load user assets:', err);
      setError(typeof err === 'string' ? err : '加载资源失败');
    } finally {
      setLoading(false);
    }
  }, [userAsset, activeTab]);

  useEffect(() => {
    loadAssets();
  }, [loadAssets]);

  const handleImport = () => {
    const tab = ASSET_TYPE_TABS.find(t => t.key === activeTab);
    if (!tab) return;

    const input = document.createElement('input');
    input.type = 'file';
    input.accept = tab.accept;
    input.onchange = async (e) => {
      const file = (e.target as HTMLInputElement).files?.[0];
      if (!file) return;

      try {
        setImporting(true);
        setError(null);

        // In Tauri, we need the file path. Use the web API to read the file,
        // then pass the path via a workaround. For Tauri, we use the file dialog approach.
        // Since we can't get the full path from web API, we'll use a Tauri command
        // that accepts the file path directly. The user needs to input the path manually
        // or we use a drag-and-drop approach.

        // For now, we'll use the file name as a hint and read the file as bytes,
        // then save it via a different approach. But the backend expects a source_path.
        // The best approach for Tauri is to use the dialog plugin.
        // Since we don't have the dialog plugin, we'll prompt the user for the path.

        const name = file.name.replace(/\.[^/.]+$/, '');
        const tags: string[] = [];

        // Try to get the file path - in Tauri webview, file inputs give us File objects
        // but not the full path. We need to use a different approach.
        // We'll use the file's name and try to construct a reasonable path,
        // but this is a limitation. The proper solution is tauri-plugin-dialog.
        // For now, we'll use a prompt as a fallback.
        const sourcePath = prompt('请输入文件的完整路径：', file.name);
        if (!sourcePath) {
          setImporting(false);
          return;
        }

        await userAsset.importUserAsset(sourcePath, activeTab, name, tags);
        await loadAssets();
      } catch (err) {
        console.error('Failed to import asset:', err);
        setError(typeof err === 'string' ? err : '导入资源失败');
      } finally {
        setImporting(false);
      }
    };
    input.click();
  };

  const handleDelete = async (assetId: string) => {
    if (!confirm('确定要删除此资源吗？')) return;
    try {
      setError(null);
      await userAsset.deleteUserAsset(assetId);
      await loadAssets();
    } catch (err) {
      console.error('Failed to delete asset:', err);
      setError(typeof err === 'string' ? err : '删除资源失败');
    }
  };

  return (
    <div style={{ marginTop: '2rem' }}>
      <h3 style={{
        fontSize: '1.3rem',
        fontWeight: 600,
        color: '#e8eaed',
        marginBottom: '1rem',
      }}>
        资源管理
      </h3>
      <p style={{
        fontSize: '0.85rem',
        color: '#7a8594',
        marginBottom: '1rem',
      }}>
        导入自定义资源作为全局替换，当 AI 生成失败时将优先使用这些资源
      </p>

      {/* Tabs */}
      <div style={{
        display: 'flex',
        gap: '0.5rem',
        marginBottom: '1rem',
        borderBottom: '1px solid #2a3a4e',
        paddingBottom: '0.5rem',
      }}>
        {ASSET_TYPE_TABS.map(tab => (
          <button
            key={tab.key}
            onClick={() => setActiveTab(tab.key)}
            style={{
              padding: '0.5rem 1.2rem',
              fontSize: '0.9rem',
              fontFamily: 'inherit',
              background: activeTab === tab.key ? 'rgba(201, 169, 98, 0.1)' : 'transparent',
              color: activeTab === tab.key ? '#c9a962' : '#7a8594',
              border: 'none',
              borderBottom: activeTab === tab.key ? '2px solid #c9a962' : '2px solid transparent',
              cursor: 'pointer',
              transition: 'all 0.2s ease',
              borderRadius: '4px 4px 0 0',
            }}
          >
            {tab.label}
          </button>
        ))}
      </div>

      {/* Import button */}
      <div style={{ marginBottom: '1rem' }}>
        <button
          className="btn btn-secondary"
          onClick={handleImport}
          disabled={importing}
          style={{
            padding: '0.6rem 1.5rem',
            fontSize: '0.9rem',
            border: '1px dashed rgba(201, 169, 98, 0.3)',
            color: importing ? '#5a6577' : '#7a8594',
          }}
        >
          {importing ? '导入中...' : '+ 导入资源'}
        </button>
      </div>

      {/* Error */}
      {error && (
        <div style={{
          padding: '0.6rem 1rem',
          marginBottom: '1rem',
          backgroundColor: 'rgba(248, 113, 113, 0.08)',
          border: '1px solid rgba(248, 113, 113, 0.2)',
          borderRadius: '8px',
          color: '#f87171',
          fontSize: '0.85rem',
          display: 'flex',
          justifyContent: 'space-between',
          alignItems: 'center',
        }}>
          <span>{error}</span>
          <button
            onClick={() => setError(null)}
            style={{
              background: 'none',
              border: 'none',
              color: '#f87171',
              cursor: 'pointer',
              fontSize: '0.85rem',
              padding: '0.2rem 0.4rem',
            }}
          >
            ✕
          </button>
        </div>
      )}

      {/* Asset list */}
      {loading ? (
        <div style={{
          display: 'flex',
          justifyContent: 'center',
          padding: '2rem 0',
          color: '#5a6577',
        }}>
          加载中...
        </div>
      ) : assets.length === 0 ? (
        <div style={{
          display: 'flex',
          flexDirection: 'column',
          alignItems: 'center',
          gap: '0.75rem',
          padding: '2.5rem 0',
          color: '#5a6577',
        }}>
          <span style={{ fontSize: '2rem' }}>📂</span>
          <span style={{ fontStyle: 'italic' }}>暂无导入的{ASSET_TYPE_TABS.find(t => t.key === activeTab)?.label}资源</span>
        </div>
      ) : (
        <div style={{
          display: 'grid',
          gridTemplateColumns: 'repeat(auto-fill, minmax(200px, 1fr))',
          gap: '0.75rem',
        }}>
          {assets.map(asset => (
            <div
              key={asset.id}
              style={{
                padding: '0.75rem',
                backgroundColor: 'rgba(26, 35, 50, 0.9)',
                border: '1px solid #2a3a4e',
                borderRadius: '10px',
                transition: 'border-color 0.2s ease',
                position: 'relative',
              }}
              onMouseEnter={e => (e.currentTarget.style.borderColor = '#c9a962')}
              onMouseLeave={e => (e.currentTarget.style.borderColor = '#2a3a4e')}
            >
              {/* Preview */}
              <div style={{
                width: '100%',
                height: '120px',
                backgroundColor: 'rgba(201, 169, 98, 0.05)',
                borderRadius: '6px',
                marginBottom: '0.5rem',
                overflow: 'hidden',
                display: 'flex',
                alignItems: 'center',
                justifyContent: 'center',
              }}>
                <AssetPreview asset={asset} />
              </div>

              {/* Info */}
              <div style={{
                fontSize: '0.85rem',
                fontWeight: 600,
                color: '#e8eaed',
                overflow: 'hidden',
                textOverflow: 'ellipsis',
                whiteSpace: 'nowrap',
                marginBottom: '0.25rem',
              }}>
                {asset.name}
              </div>
              <div style={{
                fontSize: '0.75rem',
                color: '#5a6577',
                display: 'flex',
                justifyContent: 'space-between',
              }}>
                <span>{formatFileSize(asset.fileSize)}</span>
                <span>{formatDate(asset.createdAt)}</span>
              </div>

              {/* Tags */}
              {asset.tags.length > 0 && (
                <div style={{
                  display: 'flex',
                  gap: '0.3rem',
                  flexWrap: 'wrap',
                  marginTop: '0.4rem',
                }}>
                  {asset.tags.map(tag => (
                    <span
                      key={tag}
                      style={{
                        padding: '0.1rem 0.5rem',
                        fontSize: '0.7rem',
                        backgroundColor: 'rgba(201, 169, 98, 0.08)',
                        border: '1px solid rgba(201, 169, 98, 0.2)',
                        borderRadius: '10px',
                        color: '#c9a962',
                      }}
                    >
                      {tag}
                    </span>
                  ))}
                </div>
              )}

              {/* Delete button */}
              <button
                onClick={() => handleDelete(asset.id)}
                style={{
                  position: 'absolute',
                  top: '0.5rem',
                  right: '0.5rem',
                  width: '24px',
                  height: '24px',
                  display: 'flex',
                  alignItems: 'center',
                  justifyContent: 'center',
                  fontSize: '0.75rem',
                  backgroundColor: 'rgba(248, 113, 113, 0.1)',
                  border: '1px solid rgba(248, 113, 113, 0.2)',
                  borderRadius: '50%',
                  color: '#f87171',
                  cursor: 'pointer',
                  transition: 'all 0.2s ease',
                  opacity: 0.6,
                }}
                onMouseEnter={e => {
                  e.currentTarget.style.opacity = '1';
                  e.currentTarget.style.backgroundColor = 'rgba(248, 113, 113, 0.2)';
                }}
                onMouseLeave={e => {
                  e.currentTarget.style.opacity = '0.6';
                  e.currentTarget.style.backgroundColor = 'rgba(248, 113, 113, 0.1)';
                }}
                title="删除资源"
              >
                ✕
              </button>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}

/** Asset preview component */
function AssetPreview({ asset }: { asset: UserAssetEntry }) {
  const [assetPath, setAssetPath] = useState<string | null>(null);
  const userAsset = useUserAsset();

  useEffect(() => {
    userAsset.getUserAssetPath(asset.id).then(setAssetPath).catch(() => setAssetPath(null));
  }, [asset.id, userAsset]);

  if (asset.assetType === 'image' && assetPath) {
    return (
      <img
        src={convertFileSrc(assetPath)}
        alt={asset.name}
        style={{
          width: '100%',
          height: '100%',
          objectFit: 'cover',
        }}
      />
    );
  }

  if (asset.assetType === 'music' || asset.assetType === 'voice') {
    return <span style={{ fontSize: '2.5rem' }}>🎵</span>;
  }

  if (asset.assetType === 'video') {
    return <span style={{ fontSize: '2.5rem' }}>🎬</span>;
  }

  return <span style={{ fontSize: '2.5rem' }}>📄</span>;
}
