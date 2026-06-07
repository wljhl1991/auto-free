import { useState, useEffect, useCallback } from 'react';
import { useNavigate } from 'react-router-dom';
import { useConfig } from '@/hooks/useConfig';
import type { AIProviderConfig, ConfigPreset, AIModality, GlobalSettings } from '@/types';
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

const MODALITY_LABELS: Record<string, string> = {
  text: '文本生成',
  image: '图片生成',
  video: '视频生成',
  music: '音乐生成',
  voice: '语音生成',
};

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
  const [globalSettings, setGlobalSettings] = useState<GlobalSettings | null>(null);
  const [loading, setLoading] = useState(true);
  const [checking, setChecking] = useState(false);

  // Modal state
  const [modalProvider, setModalProvider] = useState<AIProviderConfig | null>(null);
  const [modalOpen, setModalOpen] = useState(false);
  const [modalIsNew, setModalIsNew] = useState(false);

  // Log viewer state
  const [logViewerOpen, setLogViewerOpen] = useState(false);

  // Tab state
  const [activeTab, setActiveTab] = useState<string>('providers');

  const tabs = [
    { id: 'providers', label: '服务商配置' },
    { id: 'assets', label: '资源管理' },
    { id: 'system', label: '系统设置' },
    { id: 'logs', label: '日志' },
  ];

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
      setGlobalSettings(configData?.globalSettings || null);
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
        await config.updateProvider(updatedProvider);
      } else {
        await config.updateProvider(updatedProvider);
      }
      // 重新加载 providers 列表（获取脱敏版本）
      const providersData = await config.getProviders();
      setProviders(providersData || []);
    } catch (err) {
      console.error('Failed to save provider:', err);
      throw err;
    }
  };

  const handleCheckProvider = async (providerId: string, testPrompt?: string, modelId?: string, providerOverride?: any) => {
    try {
      const result = await config.checkProvider(providerId, testPrompt, modelId, providerOverride);
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

  const handleCopyProvider = async (providerId: string) => {
    try {
      await config.copyProvider(providerId);
      // 重新加载 providers 列表
      const providersData = await config.getProviders();
      setProviders(providersData || []);
    } catch (err) {
      console.error('Failed to copy provider:', err);
      alert('复制服务商失败: ' + err);
    }
  };

  const handleDeleteProvider = async (providerId: string) => {
    if (!confirm('确定要删除这个服务商吗？')) return;
    try {
      await config.deleteProvider(providerId);
      // 重新加载 providers 列表
      const providersData = await config.getProviders();
      setProviders(providersData || []);
    } catch (err) {
      console.error('Failed to delete provider:', err);
      alert('删除服务商失败: ' + err);
    }
  };

  const handleResetProvider = async (providerId: string) => {
    if (!confirm('确定要重置这个服务商为默认配置吗？这将清除您的所有自定义配置。')) return;
    try {
      await config.resetProvider(providerId);
      // 重新加载 providers 列表
      const providersData = await config.getProviders();
      setProviders(providersData || []);
    } catch (err) {
      console.error('Failed to reset provider:', err);
      alert('重置服务商失败: ' + err);
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
    if (!confirm('确定要恢复默认配置吗？\n\n这将清空所有已保存的配置（包括 API Key 等凭证和开发配置），恢复为系统内置默认值。')) return;
    try {
      await config.resetConfig();
      await loadData();
    } catch (err) {
      console.error('Failed to reset config:', err);
      alert('恢复默认配置失败: ' + err);
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

  const handlePreferredProviderChange = async (modality: string, providerId: string) => {
    try {
      const configData = await config.getConfig();
      const newPreferred = { ...(configData?.globalSettings?.preferredProviders || {}) };
      if (providerId) {
        newPreferred[modality] = providerId;
      } else {
        delete newPreferred[modality];
      }
      const newGlobalSettings = {
        ...configData?.globalSettings,
        preferredProviders: newPreferred,
      };
      await config.updateConfig({
        ...configData,
        globalSettings: newGlobalSettings,
      });
      setGlobalSettings(newGlobalSettings as GlobalSettings);
    } catch (err) {
      console.error('Failed to update preferred provider:', err);
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
        marginBottom: '1rem',
      }}>
        <div style={{ display: 'flex', alignItems: 'center', gap: '1rem' }}>
          <button className="btn btn-back" onClick={() => navigate('/')}>
            ← 返回
          </button>
          <h2 className="page-title" style={{ marginBottom: 0 }}>
            AI 配置管理
          </h2>
        </div>
      </div>

      {/* Tab Bar */}
      <div className="settings-tabs">
        {tabs.map(tab => (
          <button
            key={tab.id}
            className={`settings-tab ${activeTab === tab.id ? 'active' : ''}`}
            onClick={() => setActiveTab(tab.id)}
          >
            {tab.label}
          </button>
        ))}
      </div>

      {/* Tab Content: 服务商配置 */}
      {activeTab === 'providers' && (
        <>
          <PresetSelector
            presets={presets}
            activePresetId={activePresetId}
            onSelect={handleSelectPreset}
          />

          {/* 首选服务商 */}
          <div style={{
            background: 'rgba(255, 255, 255, 0.9)',
            borderRadius: '14px',
            padding: '1.25rem',
            marginBottom: '1.5rem',
            border: '1px solid #e8e2d8',
            boxShadow: '0 4px 16px rgba(0, 0, 0, 0.08)',
          }}>
            <h3 style={{ margin: '0 0 0.5rem 0', fontSize: '1rem', color: '#2d3748', fontWeight: 600 }}>
              首选服务商
            </h3>
            <p style={{ margin: '0 0 1rem 0', fontSize: '0.8rem', color: '#718096' }}>
              为每种模态指定优先使用的服务商。生成游戏时，系统会优先使用您指定的服务商。
            </p>
            {['text', 'image', 'video', 'music', 'voice'].map((modality) => {
              const modalityProviders = providers.filter((p) => p.modality.includes(modality as AIModality));
              const currentPreferred = globalSettings?.preferredProviders?.[modality] || '';
              return (
                <div key={modality} style={{
                  display: 'flex',
                  alignItems: 'center',
                  gap: '0.75rem',
                  marginBottom: '0.5rem',
                }}>
                  <span style={{
                    minWidth: '80px',
                    fontSize: '0.85rem',
                    color: '#4a5568',
                  }}>
                    {MODALITY_LABELS[modality]}
                  </span>
                  <select
                    value={currentPreferred}
                    onChange={(e) => handlePreferredProviderChange(modality, e.target.value)}
                    style={{
                      flex: 1,
                      padding: '0.5rem 0.7rem',
                      background: '#ffffff',
                      border: '1px solid #d4cdc2',
                      borderRadius: '10px',
                      color: '#2d3748',
                      fontSize: '0.85rem',
                      outline: 'none',
                    }}
                  >
                    <option value="">自动选择</option>
                    {modalityProviders.map((p) => (
                      <option key={p.id} value={p.id}>
                        {p.name || p.id} {p.status === 'connected' ? '(已连接)' : p.status === 'auth_failed' ? '(认证失败)' : ''}
                      </option>
                    ))}
                  </select>
                </div>
              );
            })}
          </div>

          {MODALITY_SECTIONS.map(({ modality, title }) => (
        <ModalitySection
          key={modality}
          modality={modality}
          title={title}
          providers={getProvidersByModality(modality)}
          onConfigure={handleConfigure}
          onCheck={handleCheckProvider}
          onCopy={handleCopyProvider}
          onDelete={handleDeleteProvider}
          onReset={handleResetProvider}
        />
      ))}

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
              + 添加服务商
            </button>
          </div>

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
          </div>
        </>
      )}

      {/* Tab Content: 资源管理 */}
      {activeTab === 'assets' && (
        <UserAssetManager />
      )}

      {/* Tab Content: 系统设置 */}
      {activeTab === 'system' && (
        <div style={{
          display: 'flex',
          gap: '0.75rem',
          justifyContent: 'center',
          marginTop: '1rem',
          flexWrap: 'wrap',
        }}>
          <button
            className="btn btn-secondary"
            onClick={handleExport}
            style={{ padding: '0.75rem 2rem' }}
          >
            导出配置
          </button>
          <button
            className="btn btn-secondary"
            onClick={handleImport}
            style={{ padding: '0.75rem 2rem' }}
          >
            导入配置
          </button>
          <button
            className="btn btn-secondary"
            onClick={handleSaveDevConfig}
            style={{ padding: '0.75rem 2rem', fontSize: '0.85rem' }}
            title="将当前配置保存到项目目录下的 dev-config.json，开发模式下自动加载"
          >
            保存开发配置
          </button>
          <button
            className="btn btn-secondary"
            onClick={handleReset}
            style={{ padding: '0.75rem 2rem' }}
          >
            恢复默认
          </button>
        </div>
      )}

      {/* Tab Content: 日志 */}
      {activeTab === 'logs' && (
        <div style={{ marginTop: '0.5rem' }}>
          <button
            className="btn btn-secondary"
            onClick={() => setLogViewerOpen(true)}
            style={{ marginBottom: '1rem' }}
          >
            查看日志
          </button>
        </div>
      )}

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

      {/* Log Viewer Modal */}
      <LogViewer
        isOpen={logViewerOpen}
        onClose={() => setLogViewerOpen(false)}
      />
    </div>
  );
}
