import { useState, useEffect } from 'react';
import type { AIProviderConfig, AIModelConfig, ProviderStatus, AIModality } from '@/types';
import { convertFileSrc } from '@/adapters/tauri';

interface ProviderConfigModalProps {
  provider: AIProviderConfig | null;
  isOpen: boolean;
  onClose: () => void;
  onSave: (provider: AIProviderConfig) => Promise<void>;
  onCheck: (providerId: string, testPrompt?: string, modelId?: string) => Promise<any>;
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

interface CheckResultData {
  status: string;
  message?: string;
  latency?: number;
  responsePreview?: string;
  testPrompt?: string;
  mediaUrl?: string;
  mediaType?: string;
  requestEndpoint?: string;
  requestModel?: string;
  requestHeaders?: string;
  requestBody?: string;
  responseStatus?: number;
}

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
  const [checkResult, setCheckResult] = useState<CheckResultData | null>(null);
  const [saving, setSaving] = useState(false);
  const [saveResult, setSaveResult] = useState<'success' | 'error' | null>(null);
  const [selectedModelId, setSelectedModelId] = useState<string>('');
  const [customModelInput, setCustomModelInput] = useState('');
  const [customModelModality, setCustomModelModality] = useState<AIModality>('text');
  const [isCustomModel, setIsCustomModel] = useState(false);
  const [testPrompt, setTestPrompt] = useState('hi');

  useEffect(() => {
    if (provider) {
      setEditedProvider({ ...provider });
      const defaultModel = provider.models.find(m => m.isDefault) || provider.models[0];
      setSelectedModelId(defaultModel?.id || '');
      setIsCustomModel(false);
      setCustomModelInput('');
      // 不清除 checkResult，防止一闪而逝
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
  };

  const handleEndpointChange = (value: string) => {
    setEditedProvider(prev => {
      if (!prev) return prev;
      if (isMultimodal && selectedModelId && selectedModelId !== CUSTOM_MODEL_ID) {
        return {
          ...prev,
          models: prev.models.map(m =>
            m.id === selectedModelId ? { ...m, endpoint: value } : m
          ),
        };
      }
      return {
        ...prev,
        models: prev.models.map(m => ({ ...m, endpoint: value })),
      };
    });
  };

  const handleModelSelectChange = (value: string) => {
    if (value === CUSTOM_MODEL_ID) {
      setIsCustomModel(true);
      setSelectedModelId(CUSTOM_MODEL_ID);
      setCustomModelInput('');
      const nonTextModality = editedProvider.modality.find(m => m !== 'text');
      setCustomModelModality(nonTextModality || editedProvider.modality[0] || 'text');
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
  };

  const handleCustomModelConfirm = () => {
    if (!customModelInput.trim()) return;
    const newModel: AIModelConfig = {
      id: customModelInput.trim(),
      name: customModelInput.trim(),
      modality: customModelModality,
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
  };

  const handleDeleteModel = (modelId: string) => {
    setEditedProvider(prev => {
      if (!prev) return prev;
      const remaining = prev.models.filter(m => m.id !== modelId);
      if (remaining.length === 0) return prev;
      const wasSelected = selectedModelId === modelId;
      const wasDefault = remaining.every(m => !m.isDefault);
      const updated = {
        ...prev,
        models: wasDefault
          ? remaining.map((m, i) => ({ ...m, isDefault: i === 0 }))
          : remaining,
      };
      if (wasSelected) {
        const newDefault = updated.models.find(m => m.isDefault) || updated.models[0];
        setSelectedModelId(newDefault?.id || '');
      }
      return updated;
    });
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
  };

  const handleSave = async () => {
    setSaving(true);
    setSaveResult(null);
    try {
      await onSave(editedProvider);
      setSaveResult('success');
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
    // 不清除旧结果，避免一闪而逝
    try {
      const result = await onCheck(editedProvider.id, testPrompt || undefined, selectedModelId !== CUSTOM_MODEL_ID ? selectedModelId : undefined);
      setCheckResult({
        status: result?.status || 'ok',
        message: result?.errorMessage,
        latency: result?.latency,
        responsePreview: result?.responsePreview,
        testPrompt: result?.testPrompt,
        mediaUrl: result?.mediaUrl,
        mediaType: result?.mediaType,
        requestEndpoint: result?.requestEndpoint,
        requestModel: result?.requestModel,
        requestHeaders: result?.requestHeaders,
        requestBody: result?.requestBody,
        responseStatus: result?.responseStatus,
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

  // --- 右侧测试结果面板 ---
  const renderTestPanel = () => {
    if (!checkResult) {
      return (
        <div style={{
          display: 'flex', flexDirection: 'column', alignItems: 'center', justifyContent: 'center',
          height: '100%', color: '#555570', fontSize: '0.9rem', textAlign: 'center', padding: '2rem',
        }}>
          <div style={{ fontSize: '2rem', marginBottom: '1rem', opacity: 0.3 }}>&#9881;</div>
          <div>点击左侧"测试连接"按钮</div>
          <div style={{ fontSize: '0.8rem', marginTop: '0.3rem' }}>测试结果将在此处显示</div>
        </div>
      );
    }

    const isSuccess = checkResult.status === 'ok';

    return (
      <div style={{ padding: '1rem', overflowY: 'auto', height: '100%' }}>
        {/* 状态横幅 */}
        <div style={{
          padding: '0.8rem 1rem', borderRadius: '8px', marginBottom: '1rem',
          backgroundColor: isSuccess ? 'rgba(46,125,50,0.15)' : 'rgba(224,96,96,0.15)',
          border: `2px solid ${isSuccess ? 'rgba(46,125,50,0.5)' : 'rgba(224,96,96,0.5)'}`,
        }}>
          <div style={{ fontWeight: 600, fontSize: '1rem', color: isSuccess ? '#a5d6a7' : '#e06060' }}>
            {isSuccess ? '✓ 连接成功' : '✗ 连接失败'}
          </div>
          {checkResult.latency != null && (
            <div style={{ color: '#9999bb', fontSize: '0.85rem', marginTop: '0.2rem' }}>
              延迟: {checkResult.latency}ms
              {checkResult.latency < 500 ? ' (很快)' : checkResult.latency < 2000 ? ' (正常)' : ' (较慢)'}
            </div>
          )}
          {!isSuccess && checkResult.message && (
            <div style={{ color: '#e08080', fontSize: '0.85rem', marginTop: '0.25rem', wordBreak: 'break-word' }}>
              {checkResult.message}
            </div>
          )}
        </div>

        {/* 请求详情 */}
        <div style={{ marginBottom: '0.75rem' }}>
          <div style={sectionHeaderStyle}>请求详情</div>

          {checkResult.requestEndpoint && (
            <div style={{ marginBottom: '0.5rem' }}>
              <div style={{ fontSize: '0.75rem', color: '#6a6a8a', marginBottom: '0.15rem' }}>接口地址</div>
              <div style={{
                padding: '0.4rem 0.6rem', backgroundColor: '#0a0a1a', borderRadius: '4px',
                fontSize: '0.8rem', color: '#c0c0d0', fontFamily: 'monospace',
                wordBreak: 'break-all', border: '1px solid #1e1e30',
              }}>
                {checkResult.requestEndpoint}
              </div>
            </div>
          )}

          {checkResult.requestModel && (
            <div style={{ marginBottom: '0.5rem' }}>
              <div style={{ fontSize: '0.75rem', color: '#6a6a8a', marginBottom: '0.15rem' }}>模型</div>
              <div style={{
                padding: '0.4rem 0.6rem', backgroundColor: '#0a0a1a', borderRadius: '4px',
                fontSize: '0.8rem', color: '#c0c0d0', fontFamily: 'monospace',
                border: '1px solid #1e1e30',
              }}>
                {checkResult.requestModel}
              </div>
            </div>
          )}

          {checkResult.testPrompt && (
            <div style={{ marginBottom: '0.5rem' }}>
              <div style={{ fontSize: '0.75rem', color: '#6a6a8a', marginBottom: '0.15rem' }}>提示词</div>
              <div style={{
                padding: '0.4rem 0.6rem', backgroundColor: '#0a0a1a', borderRadius: '4px',
                fontSize: '0.8rem', color: '#c0c0d0', fontFamily: 'monospace',
                border: '1px solid #1e1e30', whiteSpace: 'pre-wrap', wordBreak: 'break-word',
              }}>
                {checkResult.testPrompt}
              </div>
            </div>
          )}

          {checkResult.requestHeaders && (
            <div style={{ marginBottom: '0.5rem' }}>
              <div style={{ fontSize: '0.75rem', color: '#6a6a8a', marginBottom: '0.15rem' }}>请求头</div>
              <pre style={{
                padding: '0.5rem 0.6rem', backgroundColor: '#0a0a1a', borderRadius: '4px',
                fontSize: '0.75rem', color: '#8888aa', fontFamily: 'monospace',
                border: '1px solid #1e1e30', margin: 0, whiteSpace: 'pre-wrap', wordBreak: 'break-all',
                maxHeight: '120px', overflowY: 'auto',
              }}>
                {formatJson(checkResult.requestHeaders)}
              </pre>
            </div>
          )}

          {checkResult.requestBody && (
            <div style={{ marginBottom: '0.5rem' }}>
              <div style={{ fontSize: '0.75rem', color: '#6a6a8a', marginBottom: '0.15rem' }}>请求体</div>
              <pre style={{
                padding: '0.5rem 0.6rem', backgroundColor: '#0a0a1a', borderRadius: '4px',
                fontSize: '0.75rem', color: '#8888aa', fontFamily: 'monospace',
                border: '1px solid #1e1e30', margin: 0, whiteSpace: 'pre-wrap', wordBreak: 'break-all',
                maxHeight: '200px', overflowY: 'auto',
              }}>
                {formatJson(checkResult.requestBody)}
              </pre>
            </div>
          )}
        </div>

        {/* 响应详情 */}
        <div style={{ marginBottom: '0.75rem' }}>
          <div style={sectionHeaderStyle}>响应详情</div>

          {checkResult.responseStatus != null && (
            <div style={{ marginBottom: '0.5rem' }}>
              <div style={{ fontSize: '0.75rem', color: '#6a6a8a', marginBottom: '0.15rem' }}>状态码</div>
              <span style={{
                display: 'inline-block', padding: '0.2rem 0.6rem', borderRadius: '4px',
                fontSize: '0.85rem', fontWeight: 600, fontFamily: 'monospace',
                backgroundColor: checkResult.responseStatus >= 200 && checkResult.responseStatus < 300
                  ? 'rgba(46,125,50,0.2)' : 'rgba(224,96,96,0.2)',
                color: checkResult.responseStatus >= 200 && checkResult.responseStatus < 300
                  ? '#a5d6a7' : '#e06060',
              }}>
                {checkResult.responseStatus}
              </span>
            </div>
          )}

          {/* 文本响应 */}
          {checkResult.responsePreview && (
            <div style={{ marginBottom: '0.5rem' }}>
              <div style={{ fontSize: '0.75rem', color: '#6a6a8a', marginBottom: '0.15rem' }}>AI 响应</div>
              <div style={{
                padding: '0.6rem', backgroundColor: '#0a0a1a', borderRadius: '6px',
                fontSize: '0.85rem', color: '#c0c0d0', lineHeight: 1.6,
                border: '1px solid #1e1e30',
                maxHeight: '200px', overflowY: 'auto',
                whiteSpace: 'pre-wrap', wordBreak: 'break-word',
              }}>
                {checkResult.responsePreview}
              </div>
            </div>
          )}

          {/* 图片 */}
          {checkResult.mediaUrl && checkResult.mediaType === 'image' && (
            <div style={{ marginBottom: '0.5rem' }}>
              <div style={{ fontSize: '0.75rem', color: '#6a6a8a', marginBottom: '0.25rem' }}>测试图片</div>
              <img
                src={convertFileSrc(checkResult.mediaUrl)}
                alt="Test result"
                style={{ maxWidth: '100%', maxHeight: '250px', borderRadius: '8px', border: '1px solid #2a2a3a' }}
              />
            </div>
          )}

          {/* 音频 */}
          {checkResult.mediaUrl && checkResult.mediaType === 'audio' && (
            <div style={{ marginBottom: '0.5rem' }}>
              <div style={{ fontSize: '0.75rem', color: '#6a6a8a', marginBottom: '0.25rem' }}>测试音频</div>
              <audio controls src={convertFileSrc(checkResult.mediaUrl)} style={{ width: '100%' }} />
            </div>
          )}

          {/* 视频 */}
          {checkResult.mediaUrl && checkResult.mediaType === 'video' && (
            <div style={{ marginBottom: '0.5rem' }}>
              <div style={{ fontSize: '0.75rem', color: '#6a6a8a', marginBottom: '0.25rem' }}>测试视频</div>
              <video controls src={convertFileSrc(checkResult.mediaUrl)} style={{ maxWidth: '100%', maxHeight: '250px', borderRadius: '8px' }} />
            </div>
          )}
        </div>
      </div>
    );
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
        borderRadius: '12px',
        width: '95%', maxWidth: checkResult ? '1100px' : '600px',
        maxHeight: '88vh',
        display: 'flex',
        transition: 'max-width 0.3s ease',
      }} onClick={e => e.stopPropagation()}>
        {/* ===== 左侧：配置表单 ===== */}
        <div style={{
          flex: '0 0 420px', maxWidth: '420px',
          padding: '1.5rem',
          overflowY: 'auto',
          borderRight: checkResult ? '1px solid #2a2a3a' : 'none',
        }}>
          {/* Header */}
          <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'flex-start', marginBottom: '1.25rem' }}>
            <div>
              <h3 style={{ fontSize: '1.1rem', fontWeight: 600, color: '#e0e0f0', marginBottom: '0.2rem' }}>
                {isNew ? '添加自定义模型' : editedProvider.name}
              </h3>
              {!isNew && (
                <span style={{
                  display: 'inline-flex', alignItems: 'center', gap: '6px',
                  fontSize: '0.75rem', color: '#aaaacc',
                }}>
                  <span style={{
                    width: '7px', height: '7px', borderRadius: '50%',
                    backgroundColor: getStatusColor(editedProvider.status),
                  }} />
                  {getStatusLabel(editedProvider.status)}
                </span>
              )}
            </div>
            <button
              onClick={onClose}
              style={{
                background: 'none', border: 'none', color: '#666680', fontSize: '1.1rem',
                cursor: 'pointer', padding: '0.2rem',
              }}
            >
              ✕
            </button>
          </div>

          {/* ===== Section: 基本信息 ===== */}
          <div style={{ marginBottom: '1.25rem' }}>
            <div style={sectionHeaderStyle}>基本信息</div>

            <div style={{ marginBottom: '0.75rem' }}>
              <label style={labelStyle}>提供商 / 名称</label>
              <input type="text" value={editedProvider.name}
                onChange={e => handleNameChange(e.target.value)}
                placeholder="输入提供商名称" style={inputStyle} />
            </div>

            <div style={{ marginBottom: '0.75rem' }}>
              <label style={labelStyle}>描述</label>
              <input type="text" value={editedProvider.description}
                onChange={e => handleDescriptionChange(e.target.value)}
                placeholder="输入描述信息" style={inputStyle} />
            </div>

            <div style={{ marginBottom: '0.4rem' }}>
              <label style={{ display: 'flex', alignItems: 'center', gap: '0.5rem', fontSize: '0.85rem', color: '#c0c0d0', cursor: 'pointer' }}>
                <input type="checkbox" checked={isMultimodal}
                  onChange={e => handleMultimodalChange(e.target.checked)}
                  style={{ width: '15px', height: '15px', cursor: 'pointer' }} />
                是否多模态
              </label>
            </div>
            {isMultimodal && (
              <div style={{ marginBottom: '0.75rem', paddingLeft: '1.25rem', display: 'flex', flexWrap: 'wrap', gap: '0.6rem' }}>
                {MODALITY_OPTIONS.map(opt => (
                  <label key={opt.value} style={{ display: 'flex', alignItems: 'center', gap: '0.3rem', fontSize: '0.8rem', color: '#9999bb', cursor: 'pointer' }}>
                    <input type="checkbox" checked={editedProvider.modality.includes(opt.value)}
                      onChange={e => handleModalityToggle(opt.value, e.target.checked)} style={{ cursor: 'pointer' }} />
                    {opt.label}
                  </label>
                ))}
              </div>
            )}
          </div>

          {/* ===== Section: 接口配置 ===== */}
          <div style={{ marginBottom: '1.25rem' }}>
            <div style={sectionHeaderStyle}>接口配置</div>

            <div style={{ marginBottom: '0.75rem' }}>
              <label style={labelStyle}>接口格式</label>
              <input type="text" value="OPENAI" readOnly style={readOnlyStyle} />
            </div>

            <div style={{ marginBottom: '0.75rem' }}>
              <label style={labelStyle}>请求地址</label>
              <input type="text" value={currentEndpoint}
                onChange={e => handleEndpointChange(e.target.value)}
                placeholder="https://api.example.com/v1" style={inputStyle} />
              <span style={{ fontSize: '0.7rem', color: '#666680', marginTop: '0.2rem', display: 'block' }}>
                {currentEndpoint.includes('/chat/completions') || currentEndpoint.includes('/completions')
                  ? '当前为完整 URL，调用时将直接使用此地址'
                  : '当前为基础 URL，调用时将自动追加 /chat/completions'}
              </span>
            </div>

            {/* 模型ID */}
            <div style={{ marginBottom: '0.75rem' }}>
              <label style={labelStyle}>模型 ID</label>
              {editedProvider.models.length > 0 && !isCustomModel ? (
                <select value={selectedModelId} onChange={e => handleModelSelectChange(e.target.value)} style={selectStyle}>
                  {editedProvider.models.map(model => (
                    <option key={model.id} value={model.id}>{model.id}</option>
                  ))}
                  <option value={CUSTOM_MODEL_ID}>自定义...</option>
                </select>
              ) : (
                <div>
                  <div style={{ display: 'flex', gap: '0.4rem', marginBottom: '0.4rem' }}>
                    <input type="text" value={customModelInput}
                      onChange={e => setCustomModelInput(e.target.value)}
                      placeholder="输入自定义模型 ID"
                      style={{ ...inputStyle, flex: 1 }}
                      onKeyDown={e => { if (e.key === 'Enter') handleCustomModelConfirm(); }} />
                    <button className="btn btn-secondary"
                      style={{ padding: '0.5rem 0.6rem', fontSize: '0.75rem', whiteSpace: 'nowrap' }}
                      onClick={handleCustomModelConfirm} disabled={!customModelInput.trim()}>
                      确认
                    </button>
                    {editedProvider.models.length > 0 && (
                      <button className="btn btn-secondary"
                        style={{ padding: '0.5rem 0.6rem', fontSize: '0.75rem', whiteSpace: 'nowrap' }}
                        onClick={() => { setIsCustomModel(false); setSelectedModelId(defaultModel?.id || ''); }}>
                        取消
                      </button>
                    )}
                  </div>
                  <div style={{ display: 'flex', alignItems: 'center', gap: '0.4rem' }}>
                    <label style={{ fontSize: '0.8rem', color: '#9999bb', whiteSpace: 'nowrap' }}>模态:</label>
                    <select value={customModelModality}
                      onChange={e => setCustomModelModality(e.target.value as AIModality)}
                      style={{ ...selectStyle, flex: 1 }}>
                      {editedProvider.modality.map(mod => {
                        const opt = MODALITY_OPTIONS.find(o => o.value === mod);
                        return <option key={mod} value={mod}>{opt?.label || mod}</option>;
                      })}
                    </select>
                  </div>
                </div>
              )}
              {/* 模型说明 */}
              {selectedModelId && !isCustomModel && selectedModelId !== CUSTOM_MODEL_ID && (() => {
                const sm = editedProvider.models.find(m => m.id === selectedModelId);
                if (!sm) return null;
                const isBuiltin = provider?.models.some(m => m.id === selectedModelId);
                return (
                  <div style={{
                    marginTop: '0.4rem', padding: '0.5rem 0.6rem',
                    backgroundColor: '#0a0a1a', borderRadius: '6px',
                    fontSize: '0.75rem', color: '#8888aa', border: '1px solid #1e1e30',
                  }}>
                    {sm.name !== sm.id && <div>名称: {sm.name}</div>}
                    <div>模态: {MODALITY_OPTIONS.find(o => o.value === sm.modality)?.label || sm.modality}</div>
                    {sm.quality && <div>质量: {sm.quality === 'fast' ? '快速' : sm.quality === 'standard' ? '标准' : '高质量'}</div>}
                    {sm.freeQuota && <div style={{ color: '#a5d6a7' }}>免费额度: {sm.freeQuota}</div>}
                    {sm.maxTokens && <div>最大 Token: {sm.maxTokens}</div>}
                    {!isBuiltin && editedProvider.models.length > 1 && (
                      <button onClick={() => handleDeleteModel(selectedModelId)}
                        style={{
                          marginTop: '0.3rem', padding: '0.2rem 0.5rem',
                          fontSize: '0.7rem', color: '#e06060',
                          backgroundColor: 'rgba(224,96,96,0.1)',
                          border: '1px solid rgba(224,96,96,0.3)',
                          borderRadius: '4px', cursor: 'pointer',
                        }}>
                        删除此模型
                      </button>
                    )}
                  </div>
                );
              })()}
            </div>
          </div>

          {/* ===== Section: 认证配置 ===== */}
          <div style={{ marginBottom: '1.25rem' }}>
            <div style={sectionHeaderStyle}>认证配置</div>

            {editedProvider.authType === 'api_key' && editedProvider.authConfig.apiKey && (
              <div style={{ marginBottom: '0.75rem' }}>
                <label style={labelStyle}>{editedProvider.authConfig.apiKey.label || 'API 秘钥'}</label>
                <div style={{ display: 'flex', gap: '0.4rem' }}>
                  <input type={showApiKey ? 'text' : 'password'}
                    value={editedProvider.authConfig.apiKey.value || ''}
                    onChange={e => handleApiKeyChange(e.target.value)}
                    placeholder={editedProvider.authConfig.apiKey.placeholder || '输入 API Key'}
                    style={{ ...inputStyle, flex: 1 }} />
                  <button className="btn btn-secondary" style={{ padding: '0.5rem 0.6rem', fontSize: '0.75rem', whiteSpace: 'nowrap' }}
                    onClick={() => setShowApiKey(!showApiKey)}>
                    {showApiKey ? '隐藏' : '显示'}
                  </button>
                </div>
                {editedProvider.authConfig.apiKey.helpUrl && (
                  <a href={editedProvider.authConfig.apiKey.helpUrl} target="_blank" rel="noopener noreferrer"
                    style={{ display: 'inline-block', marginTop: '0.3rem', fontSize: '0.75rem', color: '#4a90d9', textDecoration: 'none' }}>
                    获取 API Key →
                  </a>
                )}
              </div>
            )}

            {editedProvider.authConfig.extraParams && Object.entries(editedProvider.authConfig.extraParams).map(([key, field]) => (
              <div key={key} style={{ marginBottom: '0.75rem' }}>
                <label style={labelStyle}>{field.label}</label>
                <input type={field.secret ? 'password' : 'text'}
                  value={field.value || ''} onChange={e => handleExtraParamChange(key, e.target.value)}
                  placeholder={field.placeholder} style={inputStyle} />
              </div>
            ))}

            <div style={{ marginBottom: '0.75rem' }}>
              <label style={labelStyle}>测试提示词</label>
              <input type="text" value={testPrompt}
                onChange={e => setTestPrompt(e.target.value)}
                placeholder="输入测试提示词（默认: hi）" style={inputStyle} />
              <span style={{ fontSize: '0.7rem', color: '#666680', marginTop: '0.2rem', display: 'block' }}>
                连接测试时发送给 AI 的提示词，仅对文本类服务商有效
              </span>
            </div>
          </div>

          {/* Free Quota */}
          {defaultModel?.freeQuota && (
            <div style={{
              marginBottom: '1rem', padding: '0.6rem',
              backgroundColor: 'rgba(46,125,50,0.1)', border: '1px solid rgba(46,125,50,0.3)',
              borderRadius: '8px', fontSize: '0.8rem', color: '#a5d6a7',
            }}>
              免费额度：{defaultModel.freeQuota}
            </div>
          )}

          {/* Registration Guide */}
          {!isNew && (editedProvider.officialUrl || editedProvider.registerUrl || editedProvider.docsUrl) && (
            <div style={{
              marginBottom: '1rem', padding: '0.6rem',
              backgroundColor: '#0a0a1a', borderRadius: '8px', fontSize: '0.8rem',
            }}>
              <div style={{ color: '#9999bb', marginBottom: '0.3rem' }}>注册指引</div>
              <div style={{ display: 'flex', flexDirection: 'column', gap: '0.2rem' }}>
                {editedProvider.officialUrl && (
                  <a href={editedProvider.officialUrl} target="_blank" rel="noopener noreferrer"
                    style={{ color: '#4a90d9', textDecoration: 'none', fontSize: '0.75rem' }}>官方网站 →</a>
                )}
                {editedProvider.registerUrl && (
                  <a href={editedProvider.registerUrl} target="_blank" rel="noopener noreferrer"
                    style={{ color: '#4a90d9', textDecoration: 'none', fontSize: '0.75rem' }}>注册账号 →</a>
                )}
                {editedProvider.docsUrl && (
                  <a href={editedProvider.docsUrl} target="_blank" rel="noopener noreferrer"
                    style={{ color: '#4a90d9', textDecoration: 'none', fontSize: '0.75rem' }}>API 文档 →</a>
                )}
              </div>
            </div>
          )}

          {/* Save Result */}
          {saveResult && (
            <div style={{
              marginBottom: '0.5rem', padding: '0.5rem 0.8rem',
              backgroundColor: saveResult === 'success' ? 'rgba(46,125,50,0.15)' : 'rgba(224,96,96,0.15)',
              border: `2px solid ${saveResult === 'success' ? 'rgba(46,125,50,0.5)' : 'rgba(224,96,96,0.5)'}`,
              borderRadius: '8px', fontSize: '0.85rem',
              color: saveResult === 'success' ? '#a5d6a7' : '#e06060',
              fontWeight: 600, display: 'flex', justifyContent: 'space-between', alignItems: 'center',
            }}>
              <span>{saveResult === 'success' ? '✓ 保存成功' : '✗ 保存失败'}</span>
              <button onClick={() => setSaveResult(null)}
                style={{ background: 'none', border: 'none', color: '#8888aa', cursor: 'pointer', fontSize: '0.75rem', padding: '0.1rem' }}>
                ✕
              </button>
            </div>
          )}

          {/* Actions */}
          <div style={{ display: 'flex', gap: '0.5rem', justifyContent: 'flex-end' }}>
            <button className="btn btn-secondary" onClick={onClose}
              style={{ padding: '0.5rem 1rem', fontSize: '0.85rem' }}>
              取消
            </button>
            <button className="btn btn-secondary" onClick={handleCheck}
              disabled={checking || saving}
              style={{ padding: '0.5rem 1rem', fontSize: '0.85rem' }}>
              {checking ? '检测中...' : saving ? '保存中...' : '测试连接'}
            </button>
            <button className="btn btn-primary" onClick={handleSave}
              disabled={saving}
              style={{ padding: '0.5rem 1rem', fontSize: '0.85rem' }}>
              {saving ? '保存中...' : '保存'}
            </button>
          </div>
        </div>

        {/* ===== 右侧：测试结果面板 ===== */}
        {checkResult && (
          <div style={{
            flex: '1 1 0',
            minWidth: 0,
            maxHeight: '88vh',
            overflowY: 'auto',
            backgroundColor: '#12122a',
            borderRadius: '0 12px 12px 0',
          }}>
            <div style={{
              padding: '0.8rem 1rem', borderBottom: '1px solid #2a2a3a',
              display: 'flex', justifyContent: 'space-between', alignItems: 'center',
            }}>
              <span style={{ fontSize: '0.85rem', fontWeight: 600, color: '#9999bb' }}>测试结果</span>
              <button onClick={() => setCheckResult(null)}
                style={{ background: 'none', border: 'none', color: '#666680', cursor: 'pointer', fontSize: '0.9rem', padding: '0.1rem' }}>
                ✕
              </button>
            </div>
            {renderTestPanel()}
          </div>
        )}
      </div>
    </div>
  );
}

/** 尝试格式化 JSON 字符串，失败则原样返回 */
function formatJson(str: string): string {
  try {
    const obj = JSON.parse(str);
    return JSON.stringify(obj, null, 2);
  } catch {
    return str;
  }
}
