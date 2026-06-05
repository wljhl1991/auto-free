import { useState, useEffect, useCallback } from 'react';
import { useNavigate } from 'react-router-dom';
import { useConfig } from '@/hooks/useConfig';
import type { AIProviderConfig, ConfigPreset, AIModality } from '@/types';
import PresetSelector from '@/components/Config/PresetSelector';
import ModalitySection from '@/components/Config/ModalitySection';
import ProviderConfigModal from '@/components/Config/ProviderConfigModal';
import UserAssetManager from '@/components/Config/UserAssetManager';
import LogViewer from '@/components/HUD/LogViewer';

const MODALITY_SECTIONS: { modality: AIModality; title: string }[] = [
  { modality: 'text', title: '文本生成' },
  { modality: 'image', title: '图片生成' },
  { modality: 'video', title: '视频生成' },
  { modality: 'music', title: '音乐生成' },
  { modality: 'voice', title: '语音生成' },
];

function generateId(): string {
  return `custom_${Date.now()}_${Math.random().toString(36).substring(2, 8)}`;
}

function createEmptyCustomProvider(): AIProviderConfig {
  return {
    id: generateId(),
    name: '',
    vendor: 'custom',
    description: '',
    officialUrl: '',
    registerUrl: '',
    docsUrl: '',
    modality: ['text'],
    authType: 'api_key',
    authConfig: {
      apiKey: {
        value: '',
        label: 'API 秘钥',
        placeholder: '输入 API Key',
        helpUrl: '',
      },
    },
    models: [],
    status: 'unconfigured',
  };
}

export default function Settings() {
  const navigate = useNavigate();
  const config = useConfig();

  const [presets, setPresets] = useState<ConfigPreset[]>([]);
  const [activePresetId, setActivePresetId] = useState('');
  const [providers, setProviders] = useState<AIProviderConfig[]>([]);
  const [loading, setLoading] = useState(true);
  const [checking, setChecking] = useState(false);

  // Modal state
  const [modalProvider, setModalProvider] = useState<AIProviderConfig | null>(null);
  const [modalOpen, setModalOpen] = useState(false);
  const [modalIsNew, setModalIsNew] = useState(false);

  // Log viewer state
  const [logViewerOpen, setLogViewerOpen] = useState(false);

  const loadData = useCallback(async () => {
    try {
      setLoading(true);
      const [configData, presetsData, providersData] = await Promise.all([
        config.getConfig(),
        config.getPresets(),
        config.getProviders(),
      ]);
      setActivePresetId(configData?.activePresetId || '');
      setPresets(presetsData || []);
      setProviders(providersData || []);
    } catch (err) {
      console.error('Failed to load config:', err);
    } finally {
      setLoading(false);
    }
  }, [config]);

  useEffect(() => {
    loadData();
  }, [loadData]);

  const handleSelectPreset = async (presetId: string) => {
    try {
      await config.applyPreset(presetId);
      setActivePresetId(presetId);
      // Reload providers after applying preset
      const providersData = await config.getProviders();
      setProviders(providersData || []);
    } catch (err) {
      console.error('Failed to apply preset:', err);
    }
  };

  const handleConfigure = (providerId: string) => {
    const provider = providers.find((p) => p.id === providerId);
    if (provider) {
      setModalProvider({ ...provider });
      setModalIsNew(false);
      setModalOpen(true);
    }
  };

  const handleAddCustomProvider = () => {
    setModalProvider(createEmptyCustomProvider());
    setModalIsNew(true);
    setModalOpen(true);
  };

  const handleSaveProvider = async (updatedProvider: AIProviderConfig) => {
    try {
      if (modalIsNew) {
        // For new custom providers, add to the list and save via updateProviderModels
        const newProviders = [...providers, updatedProvider];
        await config.updateProviderModels(JSON.stringify(newProviders));
        setProviders(newProviders);
      } else {
        await config.updateProvider(updatedProvider);
        setProviders((prev) =>
          prev.map((p) => (p.id === updatedProvider.id ? updatedProvider : p))
        );
        // 保存后自动检测连通性
        handleCheckProvider(updatedProvider.id);
      }
    } catch (err) {
      console.error('Failed to save provider:', err);
    }
  };

  const handleCheckProvider = async (providerId: string) => {
    try {
      const result = await config.checkProvider(providerId);
      const newStatus = result.status === 'ok' ? 'connected' :
                        result.status === 'auth_failed' ? 'auth_failed' :
                        result.status === 'network_error' ? 'network_error' :
                        result.status === 'quota_exceeded' ? 'quota_exceeded' :
                        result.status === 'unconfigured' ? 'unconfigured' : 'error';
      setProviders((prev) =>
        prev.map((p) => (p.id === providerId ? {
          ...p,
          status: newStatus,
          lastChecked: result.timestamp,
          errorMessage: result.errorMessage,
        } : p))
      );
      if (modalProvider?.id === providerId) {
        setModalProvider((prev) => (prev ? {
          ...prev,
          status: newStatus,
          lastChecked: result.timestamp,
          errorMessage: result.errorMessage,
        } : prev));
      }
      return result;
    } catch (err) {
      console.error('Failed to check provider:', err);
      throw err;
    }
  };

  const handleCheckAll = async () => {
    try {
      setChecking(true);
      const results = await config.checkAllProviders();
      if (Array.isArray(results)) {
        setProviders((prev) =>
          prev.map((p) => {
            const result = results.find((r: any) => r.id === p.id);
            return result ? { ...p, ...result } : p;
          })
        );
      }
    } catch (err) {
      console.error('Failed to check all providers:', err);
    } finally {
      setChecking(false);
    }
  };

  const handleExport = async () => {
    try {
      const json = await config.exportConfig();
      const blob = new Blob([json], { type: 'application/json' });
      const url = URL.createObjectURL(blob);
      const a = document.createElement('a');
      a.href = url;
      a.download = 'autofree-config.json';
      a.click();
      URL.revokeObjectURL(url);
    } catch (err) {
      console.error('Failed to export config:', err);
    }
  };

  const handleImport = () => {
    const input = document.createElement('input');
    input.type = 'file';
    input.accept = '.json';
    input.onchange = async (e) => {
      const file = (e.target as HTMLInputElement).files?.[0];
      if (!file) return;
      try {
        const text = await file.text();
        await config.importConfig(text);
        await loadData();
      } catch (err) {
        console.error('Failed to import config:', err);
      }
    };
    input.click();
  };

  const handleReset = async () => {
    try {
      await config.applyPreset('default');
      await loadData();
    } catch (err) {
      console.error('Failed to reset config:', err);
    }
  };

  const handleSaveDevConfig = async () => {
    try {
      await config.saveDevConfig();
      alert('开发配置已保存到项目目录下的 dev-config.json');
    } catch (err: any) {
      const msg = typeof err === 'string' ? err : (err?.message || '保存失败');
      alert(msg);
    }
  };

  const getProvidersByModality = (modality: AIModality): AIProviderConfig[] => {
    return providers.filter((p) => p.modality.includes(modality));
  };

  if (loading) {
    return (
      <div className="page settings">
        <button className="btn btn-back" onClick={() => navigate('/')}>
          ← 返回
        </button>
        <h2 className="page-title">AI 配置管理</h2>
        <p className="placeholder-text">加载中...</p>
      </div>
    );
  }

  return (
    <div className="page settings">
      {/* Header */}
      <div style={{
        display: 'flex',
        justifyContent: 'space-between',
        alignItems: 'center',
        marginBottom: '2rem',
      }}>
        <div style={{ display: 'flex', alignItems: 'center', gap: '1rem' }}>
          <button className="btn btn-back" onClick={() => navigate('/')}>
            ← 返回
          </button>
          <h2 className="page-title" style={{ marginBottom: 0 }}>
            AI 配置管理
          </h2>
        </div>
        <div style={{ display: 'flex', gap: '0.5rem' }}>
          <button
            className="btn btn-secondary"
            style={{ padding: '0.5rem 1rem', fontSize: '0.85rem' }}
            onClick={() => setLogViewerOpen(true)}
          >
            查看日志
          </button>
          <button
            className="btn btn-secondary"
            style={{ padding: '0.5rem 1rem', fontSize: '0.85rem' }}
            onClick={handleExport}
          >
            导出配置
          </button>
          <button
            className="btn btn-secondary"
            style={{ padding: '0.5rem 1rem', fontSize: '0.85rem' }}
            onClick={handleImport}
          >
            导入配置
          </button>
        </div>
      </div>

      {/* Preset Selector */}
      <PresetSelector
        presets={presets}
        activePresetId={activePresetId}
        onSelect={handleSelectPreset}
      />

      {/* Modality Sections */}
      {MODALITY_SECTIONS.map(({ modality, title }) => (
        <ModalitySection
          key={modality}
          modality={modality}
          title={title}
          providers={getProvidersByModality(modality)}
          onConfigure={handleConfigure}
          onCheck={handleCheckProvider}
        />
      ))}

      {/* Add Custom Provider Button */}
      <div style={{
        display: 'flex',
        justifyContent: 'center',
        marginTop: '1.5rem',
      }}>
        <button
          className="btn btn-secondary"
          onClick={handleAddCustomProvider}
          style={{
            padding: '0.75rem 2rem',
            fontSize: '0.9rem',
            border: '1px dashed #3a3a5a',
            color: '#8888aa',
          }}
        >
          + 添加自定义模型
        </button>
      </div>

      {/* User Asset Manager */}
      <div style={{
        marginTop: '2rem',
        paddingTop: '1.5rem',
        borderTop: '1px solid #2a2a3a',
      }}>
        <UserAssetManager />
      </div>

      {/* Bottom Actions */}
      <div style={{
        display: 'flex',
        gap: '0.75rem',
        justifyContent: 'center',
        marginTop: '2rem',
        paddingTop: '1.5rem',
        borderTop: '1px solid #2a2a3a',
      }}>
        <button
          className="btn btn-primary"
          onClick={handleCheckAll}
          disabled={checking}
          style={{ padding: '0.75rem 2rem' }}
        >
          {checking ? '检测中...' : '全部检测'}
        </button>
        <button
          className="btn btn-secondary"
          onClick={handleReset}
          style={{ padding: '0.75rem 2rem' }}
        >
          恢复默认
        </button>
        <button
          className="btn btn-secondary"
          onClick={handleSaveDevConfig}
          style={{ padding: '0.75rem 2rem', fontSize: '0.85rem' }}
          title="将当前配置保存到项目目录下的 dev-config.json，开发模式下自动加载"
        >
          保存开发配置
        </button>
      </div>

      {/* Provider Config Modal */}
      <ProviderConfigModal
        provider={modalProvider}
        isOpen={modalOpen}
        isNew={modalIsNew}
        onClose={() => {
          setModalOpen(false);
          setModalProvider(null);
          setModalIsNew(false);
        }}
        onSave={handleSaveProvider}
        onCheck={handleCheckProvider}
      />

      {/* Log Viewer */}
      <LogViewer
        isOpen={logViewerOpen}
        onClose={() => setLogViewerOpen(false)}
      />
    </div>
  );
}
