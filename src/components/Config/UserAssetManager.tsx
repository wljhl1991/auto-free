import { useState, useEffect, useCallback } from 'react';
import { useUserAsset, type UserAssetEntry } from '@/hooks/useUserAsset';
import { invoke } from '@/adapters/tauri';

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

        const name = file.name.replace(/\.[^/.]+$/, '');
        const tags: string[] = [];

        // Read file as Base64 data URL, then extract the Base64 payload
        const base64Data = await new Promise<string>((resolve, reject) => {
          const reader = new FileReader();
          reader.onload = () => {
            const result = reader.result as string;
            // data URL format: "data:<mime>;base64,<payload>"
            const base64 = result.split(',')[1];
            if (base64) {
              resolve(base64);
            } else {
              reject(new Error('Failed to read file data'));
            }
          };
          reader.onerror = () => reject(new Error('Failed to read file'));
          reader.readAsDataURL(file);
        });

        await userAsset.importUserAssetFromData(base64Data, file.name, activeTab, name, tags);
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
        color: '#2d3748',
        marginBottom: '1rem',
      }}>
        资源管理
      </h3>
      <p style={{
        fontSize: '0.85rem',
        color: '#718096',
        marginBottom: '1rem',
      }}>
        导入自定义资源作为全局替换，当 AI 生成失败时将优先使用这些资源
      </p>

      {/* Tabs */}
      <div style={{
        display: 'flex',
        gap: '0.5rem',
        marginBottom: '1rem',
        borderBottom: '1px solid #e8e2d8',
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
              background: activeTab === tab.key ? 'rgba(224, 122, 47, 0.1)' : 'transparent',
              color: activeTab === tab.key ? '#e07a2f' : '#718096',
              border: 'none',
              borderBottom: activeTab === tab.key ? '2px solid #e07a2f' : '2px solid transparent',
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
            border: '1px dashed rgba(224, 122, 47, 0.3)',
            color: importing ? '#718096' : '#718096',
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
          color: '#718096',
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
          color: '#718096',
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
                backgroundColor: 'rgba(255, 255, 255, 0.9)',
                border: '1px solid #e8e2d8',
                borderRadius: '10px',
                transition: 'border-color 0.2s ease',
                position: 'relative',
              }}
              onMouseEnter={e => (e.currentTarget.style.borderColor = '#e07a2f')}
              onMouseLeave={e => (e.currentTarget.style.borderColor = '#e8e2d8')}
            >
              {/* Preview */}
              <div style={{
                width: '100%',
                height: '120px',
                backgroundColor: 'rgba(224, 122, 47, 0.05)',
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
                color: '#2d3748',
                overflow: 'hidden',
                textOverflow: 'ellipsis',
                whiteSpace: 'nowrap',
                marginBottom: '0.25rem',
              }}>
                {asset.name}
              </div>
              <div style={{
                fontSize: '0.75rem',
                color: '#718096',
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
                        backgroundColor: 'rgba(224, 122, 47, 0.08)',
                        border: '1px solid rgba(224, 122, 47, 0.2)',
                        borderRadius: '10px',
                        color: '#e07a2f',
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
  const [assetDataUrl, setAssetDataUrl] = useState<string | null>(null);
  const userAsset = useUserAsset();

  useEffect(() => {
    let cancelled = false;
    userAsset.getUserAssetPath(asset.id).then(async (path) => {
      if (!path || cancelled) return;
      try {
        const dataUrl = await invoke<string>('read_file_as_data_url', { filePath: path });
        if (!cancelled) setAssetDataUrl(dataUrl);
      } catch (e) {
        console.warn('读取资源文件失败:', e);
      }
    }).catch(() => {});
    return () => { cancelled = true; };
  }, [asset.id, userAsset]);

  if (asset.assetType === 'image' && assetDataUrl) {
    return (
      <img
        src={assetDataUrl}
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
