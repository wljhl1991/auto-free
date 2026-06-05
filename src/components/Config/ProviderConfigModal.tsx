import { useState, useEffect } from 'react';
import type { AIProviderConfig, AIModelConfig, ProviderStatus, AIModality } from '@/types';

interface ProviderConfigModalProps {
  provider: AIProviderConfig | null;
  isOpen: boolean;
  onClose: () => void;
  onSave: (provider: AIProviderConfig) => Promise<void>;
  onCheck: (providerId: string, testPrompt?: string) => Promise<any>;
  isNew?: boolean;
}

const CUSTOM_MODEL_ID = '__custom__';

const MODALITY_OPTIONS: { value: AIModality; label: string }[] = [
  { value: 'text', label: '文本' },
  { value: 'image', label: '图片' },
  { value: 'video', label: '视频' },
  { value: 'music', label: '音乐' },
  { value: 'voice', label: '语音' },
];

export default function ProviderConfigModal({
  provider,
  isOpen,
  onClose,
  onSave,
  onCheck,
  isNew = false,
}: ProviderConfigModalProps) {
  const [editedProvider, setEditedProvider] = useState<AIProviderConfig | null>(null);
  const [showApiKey, setShowApiKey] = useState(false);
  const [checking, setChecking] = useState(false);
  const [checkResult, setCheckResult] = useState<{ status: string; message?: string; latency?: number; responsePreview?: string; testPrompt?: string } | null>(null);
  const [saving, setSaving] = useState(false);
  const [saveResult, setSaveResult] = useState<'success' | 'error' | null>(null);
  const [selectedModelId, setSelectedModelId] = useState<string>('');
  const [customModelInput, setCustomModelInput] = useState('');
  const [isCustomModel, setIsCustomModel] = useState(false);
  const [testPrompt, setTestPrompt] = useState('hi');

  useEffect(() => {
    if (provider) {
      setEditedProvider({ ...provider });
      const defaultModel = provider.models.find(m => m.isDefault) || provider.models[0];
      setSelectedModelId(defaultModel?.id || '');
      setIsCustomModel(false);
      setCustomModelInput('');
      setCheckResult(null);
      setSaveResult(null);
    }
  }, [provider]);

  if (!isOpen || !editedProvider) return null;

  const defaultModel = editedProvider.models.find(m => m.isDefault) || editedProvider.models[0];
  const selectedModel = editedProvider.models.find(m => m.id === selectedModelId);
  const currentEndpoint = selectedModel?.endpoint || defaultModel?.endpoint || '';
  const isMultimodal = editedProvider.modality.length > 1;

  // --- Handlers ---

  const handleNameChange = (value: string) => {
    setEditedProvider(prev => prev ? { ...prev, name: value } : prev);
  };

  const handleDescriptionChange = (value: string) => {
    setEditedProvider(prev => prev ? { ...prev, description: value } : prev);
  };

  const handleMultimodalChange = (checked: boolean) => {
    setEditedProvider(prev => {
      if (!prev) return prev;
      // If toggling multimodal on and currently only one modality, keep it
      // If toggling off, keep only the first modality
      if (!checked && prev.modality.length > 1) {
        return { ...prev, modality: [prev.modality[0]] };
      }
      return prev;
    });
  };

  const handleModalityToggle = (mod: AIModality, checked: boolean) => {
    setEditedProvider(prev => {
      if (!prev) return prev;
      if (checked) {
        return { ...prev, modality: [...prev.modality, mod] };
      } else {
        const filtered = prev.modality.filter(m => m !== mod);
        return { ...prev, modality: filtered.length > 0 ? filtered : prev.modality };
      }
    });
  };

  const handleApiKeyChange = (value: string) => {
    setEditedProvider(prev => {
      if (!prev) return prev;
      return {
        ...prev,
        authConfig: {
          ...prev.authConfig,
          apiKey: prev.authConfig.apiKey
            ? { ...prev.authConfig.apiKey, value }
            : { value, label: 'API Key', placeholder: '输入 API Key', helpUrl: '' },
        },
        status: value.trim() ? 'configured' : 'unconfigured',
      };
    });
    setCheckResult(null);
  };

  const handleEndpointChange = (value: string) => {
    setEditedProvider(prev => {
      if (!prev) return prev;
      // 如果是多模态且选中了特定模型，只修改该模型的 endpoint
      if (isMultimodal && selectedModelId && selectedModelId !== CUSTOM_MODEL_ID) {
        return {
          ...prev,
          models: prev.models.map(m =>
            m.id === selectedModelId ? { ...m, endpoint: value } : m
          ),
        };
      }
      // 单模态或未选中特定模型，修改所有模型的 endpoint
      return {
        ...prev,
        models: prev.models.map(m => ({ ...m, endpoint: value })),
      };
    });
    setCheckResult(null);
  };

  const handleModelSelectChange = (value: string) => {
    if (value === CUSTOM_MODEL_ID) {
      setIsCustomModel(true);
      setSelectedModelId(CUSTOM_MODEL_ID);
      setCustomModelInput('');
    } else {
      setIsCustomModel(false);
      setSelectedModelId(value);
      setEditedProvider(prev => {
        if (!prev) return prev;
        return {
          ...prev,
          models: prev.models.map(m => ({
            ...m,
            isDefault: m.id === value,
          })),
        };
      });
    }
    setCheckResult(null);
  };

  const handleCustomModelConfirm = () => {
    if (!customModelInput.trim()) return;
    const newModel: AIModelConfig = {
      id: customModelInput.trim(),
      name: customModelInput.trim(),
      modality: editedProvider.modality[0] || 'text',
      isDefault: true,
      endpoint: currentEndpoint,
      quality: 'standard',
    };
    setEditedProvider(prev => {
      if (!prev) return prev;
      return {
        ...prev,
        models: [
          ...prev.models.map(m => ({ ...m, isDefault: false })),
          newModel,
        ],
      };
    });
    setSelectedModelId(newModel.id);
    setIsCustomModel(false);
    setCustomModelInput('');
    setCheckResult(null);
  };

  const handleExtraParamChange = (key: string, value: string) => {
    setEditedProvider(prev => {
      if (!prev) return prev;
      return {
        ...prev,
        authConfig: {
          ...prev.authConfig,
          extraParams: {
            ...(prev.authConfig.extraParams || {}),
            [key]: {
              ...(prev.authConfig.extraParams?.[key] || { label: key, placeholder: '', required: false, secret: false }),
              value,
            },
          },
        },
        status: 'configured',
      };
    });
    setCheckResult(null);
  };

  const handleSave = async () => {
    setSaving(true);
    setSaveResult(null);
    try {
      await onSave(editedProvider);
      setSaveResult('success');
      setTimeout(() => setSaveResult(null), 3000);
    } catch (err) {
      setSaveResult('error');
    } finally {
      setSaving(false);
    }
  };

  const handleCheck = async () => {
    // 先保存再测试
    setSaving(true);
    try {
      await onSave(editedProvider);
    } finally {
      setSaving(false);
    }

    setChecking(true);
    setCheckResult(null);
    try {
      const result = await onCheck(editedProvider.id, testPrompt || undefined);
      setCheckResult({
        status: result?.status || 'ok',
        message: result?.errorMessage,
        latency: result?.latency,
        responsePreview: result?.responsePreview,
        testPrompt: result?.testPrompt,
      });
      if (result) {
        setEditedProvider(prev => prev ? {
          ...prev,
          status: result.status === 'ok' ? 'connected' : mapCheckStatus(result.status),
          lastChecked: result.timestamp,
          errorMessage: result.errorMessage,
        } : prev);
      }
    } catch (err: any) {
      const msg = typeof err === 'string' ? err : (err?.message || err?.toString() || '检测失败');
      setCheckResult({
        status: 'error',
        message: msg,
      });
    } finally {
      setChecking(false);
    }
  };

  const mapCheckStatus = (status: string): ProviderStatus => {
    const map: Record<string, ProviderStatus> = {
      'ok': 'connected',
      'auth_failed': 'auth_failed',
      'network_error': 'network_error',
      'quota_exceeded': 'quota_exceeded',
      'unconfigured': 'unconfigured',
    };
    return map[status] || 'error';
  };

  const getStatusLabel = (status: ProviderStatus): string => {
    const map: Record<ProviderStatus, string> = {
      connected: '已连接',
      configured: '已配置',
      unconfigured: '未配置',
      auth_failed: '认证失败',
      quota_exceeded: '额度不足',
      network_error: '网络错误',
      error: '错误',
    };
    return map[status] || '未知';
  };

  const getStatusColor = (status: ProviderStatus): string => {
    const map: Record<ProviderStatus, string> = {
      connected: '#4caf50',
      configured: '#ff9800',
      unconfigured: '#666680',
      auth_failed: '#e06060',
      quota_exceeded: '#ff9800',
      network_error: '#e06060',
      error: '#e06060',
    };
    return map[status] || '#666680';
  };

  // --- Styles ---
  const inputStyle: React.CSSProperties = {
    width: '100%', padding: '0.6rem 0.8rem', fontSize: '0.9rem',
    fontFamily: 'monospace', backgroundColor: '#0a0a1a', color: '#e0e0e0',
    border: '1px solid #2a2a3a', borderRadius: '8px', outline: 'none',
    boxSizing: 'border-box',
  };

  const selectStyle: React.CSSProperties = {
    width: '100%', padding: '0.6rem 0.8rem', fontSize: '0.9rem',
    backgroundColor: '#0a0a1a', color: '#e0e0e0',
    border: '1px solid #2a2a3a', borderRadius: '8px', outline: 'none',
    boxSizing: 'border-box',
  };

  const labelStyle: React.CSSProperties = {
    display: 'block', fontSize: '0.85rem', color: '#9999bb', marginBottom: '0.4rem', fontWeight: 500,
  };

  const sectionHeaderStyle: React.CSSProperties = {
    fontSize: '0.8rem', fontWeight: 600, color: '#6a6a8a', textTransform: 'uppercase',
    letterSpacing: '0.05em', marginBottom: '0.75rem', paddingBottom: '0.4rem',
    borderBottom: '1px solid #1e1e30',
  };

  const readOnlyStyle: React.CSSProperties = {
    ...inputStyle, color: '#666680', backgroundColor: '#0e0e1e', cursor: 'default',
  };

  return (
    <div style={{
      position: 'fixed', inset: 0,
      backgroundColor: 'rgba(0,0,0,0.7)',
      display: 'flex', alignItems: 'center', justifyContent: 'center',
      zIndex: 1000,
    }} onClick={e => { if (e.target === e.currentTarget) onClose(); }}>
      <div style={{
        backgroundColor: '#16162a', border: '1px solid #2a2a3a',
        borderRadius: '12px', padding: '2rem',
        width: '90%', maxWidth: '600px', maxHeight: '85vh', overflowY: 'auto',
      }} onClick={e => e.stopPropagation()}>
        {/* Header */}
        <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'flex-start', marginBottom: '1.5rem' }}>
          <div>
            <h3 style={{ fontSize: '1.2rem', fontWeight: 600, color: '#e0e0f0', marginBottom: '0.25rem' }}>
              {isNew ? '添加自定义模型' : editedProvider.name}
            </h3>
            {!isNew && (
              <span style={{
                display: 'inline-flex', alignItems: 'center', gap: '6px',
                fontSize: '0.8rem', color: '#aaaacc',
              }}>
                <span style={{
                  width: '8px', height: '8px', borderRadius: '50%',
                  backgroundColor: getStatusColor(editedProvider.status),
                }} />
                {getStatusLabel(editedProvider.status)}
              </span>
            )}
          </div>
          <button
            onClick={onClose}
            style={{
              background: 'none', border: 'none', color: '#666680', fontSize: '1.2rem',
              cursor: 'pointer', padding: '0.25rem',
            }}
          >
            ✕
          </button>
        </div>

        {/* ===== Section: 基本信息 ===== */}
        <div style={{ marginBottom: '1.5rem' }}>
          <div style={sectionHeaderStyle}>基本信息</div>

          {/* 提供商/名称 */}
          <div style={{ marginBottom: '1rem' }}>
            <label style={labelStyle}>提供商 / 名称</label>
            <input
              type="text"
              value={editedProvider.name}
              onChange={e => handleNameChange(e.target.value)}
              placeholder="输入提供商名称"
              style={inputStyle}
            />
          </div>

          {/* 描述 */}
          <div style={{ marginBottom: '1rem' }}>
            <label style={labelStyle}>描述</label>
            <input
              type="text"
              value={editedProvider.description}
              onChange={e => handleDescriptionChange(e.target.value)}
              placeholder="输入描述信息"
              style={inputStyle}
            />
          </div>

          {/* 是否多模态 */}
          <div style={{ marginBottom: '0.5rem' }}>
            <label style={{
              display: 'flex', alignItems: 'center', gap: '0.5rem',
              fontSize: '0.9rem', color: '#c0c0d0', cursor: 'pointer',
            }}>
              <input
                type="checkbox"
                checked={isMultimodal}
                onChange={e => handleMultimodalChange(e.target.checked)}
                style={{ width: '16px', height: '16px', cursor: 'pointer' }}
              />
              是否多模态
            </label>
          </div>
          {/* 多模态选择 */}
          {isMultimodal && (
            <div style={{ marginBottom: '1rem', paddingLeft: '1.5rem', display: 'flex', flexWrap: 'wrap', gap: '0.75rem' }}>
              {MODALITY_OPTIONS.map(opt => (
                <label key={opt.value} style={{
                  display: 'flex', alignItems: 'center', gap: '0.35rem',
                  fontSize: '0.85rem', color: '#9999bb', cursor: 'pointer',
                }}>
                  <input
                    type="checkbox"
                    checked={editedProvider.modality.includes(opt.value)}
                    onChange={e => handleModalityToggle(opt.value, e.target.checked)}
                    style={{ cursor: 'pointer' }}
                  />
                  {opt.label}
                </label>
              ))}
            </div>
          )}
        </div>

        {/* ===== Section: 接口配置 ===== */}
        <div style={{ marginBottom: '1.5rem' }}>
          <div style={sectionHeaderStyle}>接口配置</div>

          {/* 接口格式 */}
          <div style={{ marginBottom: '1rem' }}>
            <label style={labelStyle}>接口格式</label>
            <input
              type="text"
              value="OPENAI"
              readOnly
              style={readOnlyStyle}
            />
          </div>

          {/* 请求地址 */}
          <div style={{ marginBottom: '1rem' }}>
            <label style={labelStyle}>请求地址</label>
            <input
              type="text"
              value={currentEndpoint}
              onChange={e => handleEndpointChange(e.target.value)}
              placeholder="https://api.example.com/v1"
              style={inputStyle}
            />
            <span style={{ fontSize: '0.75rem', color: '#666680', marginTop: '0.25rem', display: 'block' }}>
              {currentEndpoint.includes('/chat/completions') || currentEndpoint.includes('/completions')
                ? '当前为完整 URL，调用时将直接使用此地址'
                : '当前为基础 URL，调用时将自动追加 /chat/completions'}
            </span>
            {isMultimodal && selectedModelId && selectedModelId !== CUSTOM_MODEL_ID && (
              <span style={{ fontSize: '0.75rem', color: '#e0a040', marginTop: '0.25rem', display: 'block' }}>
                多模态模式下，切换模型 ID 会显示该模型的独立接口地址
              </span>
            )}
          </div>

          {/* 模型ID */}
          <div style={{ marginBottom: '1rem' }}>
            <label style={labelStyle}>模型 ID</label>
            {editedProvider.models.length > 0 && !isCustomModel ? (
              <div>
                <select
                  value={selectedModelId}
                  onChange={e => handleModelSelectChange(e.target.value)}
                  style={selectStyle}
                >
                  {editedProvider.models.map(model => (
                    <option key={model.id} value={model.id}>
                      {model.id}
                    </option>
                  ))}
                  <option value={CUSTOM_MODEL_ID}>自定义...</option>
                </select>
              </div>
            ) : (
              <div style={{ display: 'flex', gap: '0.5rem' }}>
                <input
                  type="text"
                  value={customModelInput}
                  onChange={e => setCustomModelInput(e.target.value)}
                  placeholder="输入自定义模型 ID"
                  style={{ ...inputStyle, flex: 1 }}
                  onKeyDown={e => { if (e.key === 'Enter') handleCustomModelConfirm(); }}
                />
                <button
                  className="btn btn-secondary"
                  style={{ padding: '0.6rem 0.8rem', fontSize: '0.8rem', whiteSpace: 'nowrap' }}
                  onClick={handleCustomModelConfirm}
                  disabled={!customModelInput.trim()}
                >
                  确认
                </button>
                {editedProvider.models.length > 0 && (
                  <button
                    className="btn btn-secondary"
                    style={{ padding: '0.6rem 0.8rem', fontSize: '0.8rem', whiteSpace: 'nowrap' }}
                    onClick={() => { setIsCustomModel(false); setSelectedModelId(defaultModel?.id || ''); }}
                  >
                    取消
                  </button>
                )}
              </div>
            )}
            {/* 模型说明信息 */}
            {selectedModelId && !isCustomModel && selectedModelId !== CUSTOM_MODEL_ID && (() => {
              const selectedModel = editedProvider.models.find(m => m.id === selectedModelId);
              if (!selectedModel) return null;
              return (
                <div style={{
                  marginTop: '0.5rem', padding: '0.6rem 0.8rem',
                  backgroundColor: '#0a0a1a', borderRadius: '6px',
                  fontSize: '0.8rem', color: '#8888aa',
                  border: '1px solid #1e1e30',
                }}>
                  {selectedModel.name !== selectedModel.id && (
                    <div>名称: {selectedModel.name}</div>
                  )}
                  <div>模态: {selectedModel.modality === 'text' ? '文本' : selectedModel.modality === 'image' ? '图片' : selectedModel.modality === 'video' ? '视频' : selectedModel.modality === 'music' ? '音乐' : '语音'}</div>
                  {selectedModel.quality && <div>质量: {selectedModel.quality === 'fast' ? '快速' : selectedModel.quality === 'standard' ? '标准' : '高质量'}</div>}
                  {selectedModel.freeQuota && <div style={{ color: '#a5d6a7' }}>免费额度: {selectedModel.freeQuota}</div>}
                  {selectedModel.maxTokens && <div>最大 Token: {selectedModel.maxTokens}</div>}
                  {selectedModel.endpoint && selectedModel.endpoint !== currentEndpoint && (
                    <div style={{ color: '#e0a040' }}>独立接口: {selectedModel.endpoint}</div>
                  )}
                  <div style={{ marginTop: '0.3rem', fontSize: '0.75rem', color: '#555570', fontStyle: 'italic' }}>
                    * 优化信息仅供参考，以服务商官方文档为准
                  </div>
                </div>
              );
            })()}
          </div>
        </div>

        {/* ===== Section: 认证配置 ===== */}
        <div style={{ marginBottom: '1.5rem' }}>
          <div style={sectionHeaderStyle}>认证配置</div>

          {/* API Key */}
          {editedProvider.authType === 'api_key' && editedProvider.authConfig.apiKey && (
            <div style={{ marginBottom: '1rem' }}>
              <label style={labelStyle}>
                {editedProvider.authConfig.apiKey.label || 'API 秘钥'}
              </label>
              <div style={{ display: 'flex', gap: '0.5rem' }}>
                <input
                  type={showApiKey ? 'text' : 'password'}
                  value={editedProvider.authConfig.apiKey.value || ''}
                  onChange={e => handleApiKeyChange(e.target.value)}
                  placeholder={editedProvider.authConfig.apiKey.placeholder || '输入 API Key'}
                  style={{ ...inputStyle, flex: 1 }}
                />
                <button className="btn btn-secondary" style={{ padding: '0.6rem 0.8rem', fontSize: '0.8rem', whiteSpace: 'nowrap' }}
                  onClick={() => setShowApiKey(!showApiKey)}>
                  {showApiKey ? '隐藏' : '显示'}
                </button>
              </div>
              {editedProvider.authConfig.apiKey.helpUrl && (
                <a href={editedProvider.authConfig.apiKey.helpUrl} target="_blank" rel="noopener noreferrer"
                  style={{ display: 'inline-block', marginTop: '0.4rem', fontSize: '0.8rem', color: '#4a90d9', textDecoration: 'none' }}>
                  获取 API Key →
                </a>
              )}
            </div>
          )}

          {/* Extra Params */}
          {editedProvider.authConfig.extraParams && Object.entries(editedProvider.authConfig.extraParams).map(([key, field]) => (
            <div key={key} style={{ marginBottom: '1rem' }}>
              <label style={labelStyle}>{field.label}</label>
              <input
                type={field.secret ? 'password' : 'text'}
                value={field.value || ''}
                onChange={e => handleExtraParamChange(key, e.target.value)}
                placeholder={field.placeholder}
                style={inputStyle}
              />
            </div>
          ))}

          {/* Test Prompt */}
          <div style={{ marginBottom: '1rem' }}>
            <label style={labelStyle}>测试提示词</label>
            <input
              type="text"
              value={testPrompt}
              onChange={e => setTestPrompt(e.target.value)}
              placeholder="输入测试提示词（默认: hi）"
              style={inputStyle}
            />
            <span style={{ fontSize: '0.75rem', color: '#666680', marginTop: '0.25rem', display: 'block' }}>
              连接测试时发送给 AI 的提示词，仅对文本类服务商有效
            </span>
          </div>
        </div>

        {/* ===== Check Result ===== */}
        {checkResult && (
          <div style={{
            marginBottom: '1.25rem', padding: '0.75rem',
            backgroundColor: checkResult.status === 'ok' ? 'rgba(46,125,50,0.1)' : 'rgba(224,96,96,0.1)',
            border: `1px solid ${checkResult.status === 'ok' ? 'rgba(46,125,50,0.3)' : 'rgba(224,96,96,0.3)'}`,
            borderRadius: '8px', fontSize: '0.85rem',
            color: checkResult.status === 'ok' ? '#a5d6a7' : '#e06060',
          }}>
            <div>
              {checkResult.status === 'ok' ? '连接成功！' : `连接失败：${checkResult.message || '未知错误'}`}
              {checkResult.latency != null && (
                <span style={{ marginLeft: '0.5rem', color: '#9999bb', fontSize: '0.8rem' }}>
                  延迟: {checkResult.latency}ms
                </span>
              )}
            </div>
            {checkResult.responsePreview && (
              <div style={{
                marginTop: '0.5rem', padding: '0.5rem',
                backgroundColor: 'rgba(0,0,0,0.2)', borderRadius: '4px',
                fontSize: '0.8rem', color: '#c0c0d0',
                maxHeight: '120px', overflowY: 'auto',
                whiteSpace: 'pre-wrap', wordBreak: 'break-word',
              }}>
                <div style={{ color: '#6a6a8a', marginBottom: '0.25rem' }}>AI 响应:</div>
                {checkResult.responsePreview}
              </div>
            )}
          </div>
        )}

        {/* ===== Free Quota Info ===== */}
        {defaultModel?.freeQuota && (
          <div style={{
            marginBottom: '1.25rem', padding: '0.75rem',
            backgroundColor: 'rgba(46,125,50,0.1)', border: '1px solid rgba(46,125,50,0.3)',
            borderRadius: '8px', fontSize: '0.85rem', color: '#a5d6a7',
          }}>
            免费额度：{defaultModel.freeQuota}
          </div>
        )}

        {/* ===== Registration Guide (only for built-in) ===== */}
        {!isNew && (editedProvider.officialUrl || editedProvider.registerUrl || editedProvider.docsUrl) && (
          <div style={{
            marginBottom: '1.5rem', padding: '0.75rem',
            backgroundColor: '#0a0a1a', borderRadius: '8px', fontSize: '0.85rem',
          }}>
            <div style={{ color: '#9999bb', marginBottom: '0.5rem' }}>注册指引</div>
            <div style={{ display: 'flex', flexDirection: 'column', gap: '0.3rem' }}>
              {editedProvider.officialUrl && (
                <a href={editedProvider.officialUrl} target="_blank" rel="noopener noreferrer"
                  style={{ color: '#4a90d9', textDecoration: 'none', fontSize: '0.8rem' }}>官方网站 →</a>
              )}
              {editedProvider.registerUrl && (
                <a href={editedProvider.registerUrl} target="_blank" rel="noopener noreferrer"
                  style={{ color: '#4a90d9', textDecoration: 'none', fontSize: '0.8rem' }}>注册账号 →</a>
              )}
              {editedProvider.docsUrl && (
                <a href={editedProvider.docsUrl} target="_blank" rel="noopener noreferrer"
                  style={{ color: '#4a90d9', textDecoration: 'none', fontSize: '0.8rem' }}>API 文档 →</a>
              )}
            </div>
          </div>
        )}

        {/* ===== Save Result ===== */}
        {saveResult && (
          <div style={{
            marginBottom: '1rem', padding: '0.6rem 0.8rem',
            backgroundColor: saveResult === 'success' ? 'rgba(46,125,50,0.1)' : 'rgba(224,96,96,0.1)',
            border: `1px solid ${saveResult === 'success' ? 'rgba(46,125,50,0.3)' : 'rgba(224,96,96,0.3)'}`,
            borderRadius: '8px', fontSize: '0.85rem',
            color: saveResult === 'success' ? '#a5d6a7' : '#e06060',
          }}>
            {saveResult === 'success' ? '保存成功！' : '保存失败，请重试'}
          </div>
        )}

        {/* ===== Actions ===== */}
        <div style={{ display: 'flex', gap: '0.75rem', justifyContent: 'flex-end' }}>
          <button className="btn btn-secondary" onClick={onClose}
            style={{ padding: '0.6rem 1.2rem', fontSize: '0.9rem' }}>
            取消
          </button>
          <button className="btn btn-secondary" onClick={handleCheck}
            disabled={checking || saving}
            style={{ padding: '0.6rem 1.2rem', fontSize: '0.9rem' }}>
            {checking ? '检测中...' : saving ? '保存中...' : '测试连接'}
          </button>
          <button className="btn btn-primary" onClick={handleSave}
            disabled={saving}
            style={{ padding: '0.6rem 1.2rem', fontSize: '0.9rem' }}>
            {saving ? '保存中...' : '保存'}
          </button>
        </div>
      </div>
    </div>
  );
}
