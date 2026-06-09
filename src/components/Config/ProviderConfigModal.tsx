import { useState, useEffect, useCallback, useRef } from 'react';
import type { AIProviderConfig, ProviderStatus, AIModality } from '@/types';
import { invoke } from '@/adapters/tauri';
import { openUrl } from '@tauri-apps/plugin-opener';

interface ProviderConfigModalProps {
  provider: AIProviderConfig | null;
  isOpen: boolean;
  onClose: () => void;
  onSave: (provider: AIProviderConfig) => Promise<void>;
  onCheck: (providerId: string, testPrompt?: string, modelId?: string, providerOverride?: any) => Promise<any>;
  isNew?: boolean;
}

const MODALITY_OPTIONS: { value: AIModality; label: string }[] = [
  { value: 'text', label: '文本' },
  { value: 'image', label: '图片' },
  { value: 'video', label: '视频' },
  { value: 'music', label: '音乐' },
  { value: 'voice', label: '语音' },
];

interface MediaItemData {
  id: string;
  mediaType?: string;
  title?: string;
  mediaUrl?: string;
  imageUrl?: string;
  audioUrl?: string;
  localPath?: string;
  dataUrl?: string;
  status?: string;
  tags?: string;
  progress?: number;
  error?: string;
}

interface CheckResultData {
  status: string;
  message?: string;
  latency?: number;
  responsePreview?: string;
  testPrompt?: string;
  mediaUrl?: string;  // 本地文件路径（需要转换为 data URL）
  mediaDataUrl?: string;  // base64 data URL（前端直接使用）
  mediaType?: string;
  mediaError?: string;  // 媒体文件读取错误
  // 轮询状态（妙音 AI 等异步生成）
  pollingTaskId?: string;
  pollingStatus?: string;
  pollingElapsedSecs?: number;
  // 多媒体结果（多首音乐 + 封面）
  mediaItems?: MediaItemData[];
  // 请求详情
  requestEndpoint?: string;
  requestModel?: string;
  requestHeaders?: string;
  requestBody?: string;
  responseStatus?: number;
}

function generateId(): string {
  return `custom_${Date.now()}_${Math.random().toString(36).substring(2, 8)}`;
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
  const [selectedModelId, setSelectedModelId] = useState('');
  const [testPrompt, setTestPrompt] = useState('hi');
  const [tipOpen, setTipOpen] = useState<Record<string, boolean>>({});

  // 新建流程相关状态
  const [builtinTemplates, setBuiltinTemplates] = useState<AIProviderConfig[]>([]);
  const [selectedModality, setSelectedModality] = useState<AIModality | null>(null);
  const [selectedBuiltinId, setSelectedBuiltinId] = useState<string>('');

  // 弹窗打开时重置状态（使用 ref 防止 provider 引用变化导致重复重置）
  const openIdRef = useRef<string>('');
  useEffect(() => {
    if (isOpen && provider) {
      // 仅在弹窗新打开或切换 provider 时重置
      const openId = `${isOpen}-${provider.id}`;
      if (openIdRef.current !== openId) {
        openIdRef.current = openId;
        setEditedProvider({ ...provider, models: provider.models.map(m => ({ ...m })) });
        const defaultModel = provider.models.find(m => m.isDefault) || provider.models[0];
        setSelectedModelId(defaultModel?.id || '');
        setCheckResult(null);
        setSaveResult(null);
        setShowApiKey(false);
        // 新建流程重置
        setSelectedModality(null);
        setSelectedBuiltinId('');
      } else {
        // provider 引用变了但 id 没变（如测试后状态更新），只同步状态字段，不覆盖用户编辑的内容
        setEditedProvider(prev => prev ? {
          ...prev,
          status: provider.status,
          lastChecked: provider.lastChecked,
          errorMessage: provider.errorMessage,
        } : { ...provider, models: provider.models.map(m => ({ ...m })) });
      }
    }
  }, [isOpen, provider]);

  // 加载内置模板（仅新建时）
  useEffect(() => {
    if (isOpen && isNew) {
      invoke<AIProviderConfig[]>('get_builtin_provider_templates')
        .then(templates => {
          setBuiltinTemplates(templates || []);
        })
        .catch(err => {
          console.warn('加载内置模板失败:', err);
          setBuiltinTemplates([]);
        });
    }
  }, [isOpen, isNew]);

  // 关闭时清空状态
  const handleClose = useCallback(() => {
    setCheckResult(null);
    setSaveResult(null);
    openIdRef.current = '';
    onClose();
  }, [onClose]);

  if (!isOpen || !editedProvider) return null;

  const defaultModel = editedProvider.models.find(m => m.isDefault) || editedProvider.models[0];
  const selectedModel = editedProvider.models.find(m => m.id === selectedModelId);
  const currentEndpoint = selectedModel?.endpoint || defaultModel?.endpoint || '';

  // 新建流程：根据选择的模态过滤内置服务商
  const filteredBuiltinProviders = builtinTemplates.filter(p =>
    selectedModality ? p.modality.includes(selectedModality) : false
  );

  // --- Handlers ---

  const handleNameChange = (value: string) => {
    setEditedProvider(prev => prev ? { ...prev, name: value } : prev);
  };

  const handleDescriptionChange = (value: string) => {
    setEditedProvider(prev => prev ? { ...prev, description: value } : prev);
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
      if (selectedModelId) {
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

  // 新建流程：选择模态
  const handleNewModalitySelect = (mod: AIModality) => {
    setSelectedModality(mod);
    setSelectedBuiltinId('');
    // 重置为空服务商
    setEditedProvider(prev => prev ? {
      ...prev,
      modality: [mod],
      models: [],
    } : prev);
    setSelectedModelId('');
  };

  // 新建流程：选择内置服务商，回填信息
  const handleBuiltinProviderSelect = (builtinId: string) => {
    setSelectedBuiltinId(builtinId);
    const template = builtinTemplates.find(t => t.id === builtinId);
    if (template && selectedModality) {
      // 只保留当前模态的模型
      const filteredModels = template.models
        .filter(m => m.modality === selectedModality)
        .map(m => ({ ...m }));
      // 确保至少有一个默认模型
      if (filteredModels.length > 0 && filteredModels.every(m => !m.isDefault)) {
        filteredModels[0].isDefault = true;
      }
      // 回填模板信息，但生成新的唯一 ID，modality 只保留选中的
      const newProvider: AIProviderConfig = {
        ...template,
        id: generateId(),
        providerType: template.id, // 保存原始内置服务商 ID，用于后端路由
        modality: [selectedModality],
        models: filteredModels.length > 0 ? filteredModels : template.models.map(m => ({ ...m })),
        status: 'unconfigured',
        lastChecked: undefined,
        errorMessage: undefined,
        authConfig: {
          ...template.authConfig,
          apiKey: template.authConfig.apiKey
            ? { ...template.authConfig.apiKey, value: '' }
            : undefined,
          extraParams: template.authConfig.extraParams
            ? Object.fromEntries(
                Object.entries(template.authConfig.extraParams).map(([k, v]) => [k, { ...v, value: '' }])
              )
            : undefined,
        },
      };
      setEditedProvider(newProvider);
      const defaultM = newProvider.models.find(m => m.isDefault) || newProvider.models[0];
      setSelectedModelId(defaultM?.id || '');
    }
  };

  const handleAdvancedParamChange = (key: string, value: string) => {
    setEditedProvider(prev => {
      if (!prev) return prev;
      return {
        ...prev,
        models: prev.models.map(m => {
          if (m.id !== selectedModelId) return m;
          const currentParams = m.advancedParams || {};
          if (value.trim() === '') {
            // 清空则删除该参数
            const { [key]: _, ...rest } = currentParams;
            return { ...m, advancedParams: Object.keys(rest).length > 0 ? rest : undefined };
          }
          const numVal = Number(value);
          if (isNaN(numVal)) return m;
          return { ...m, advancedParams: { ...currentParams, [key]: numVal } };
        }),
      };
    });
  };

  // 语音参数处理（字符串值）
  const handleVoiceParamChange = (key: string, value: string) => {
    setEditedProvider(prev => {
      if (!prev) return prev;
      return {
        ...prev,
        models: prev.models.map(m => {
          if (m.id !== selectedModelId) return m;
          const currentParams = m.advancedParams || {};
          if (value === '' || value === '__default__') {
            const { [key]: _, ...rest } = currentParams;
            return { ...m, advancedParams: Object.keys(rest).length > 0 ? rest : undefined };
          }
          return { ...m, advancedParams: { ...currentParams, [key]: value } };
        }),
      };
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
      // 新建时确保 ID 唯一
      const providerToSave = { ...editedProvider };
      if (isNew && (!providerToSave.id || providerToSave.id.startsWith('custom_') === false)) {
        // 如果 id 仍与内置模板相同，重新生成
        const builtinIds = builtinTemplates.map(t => t.id);
        if (builtinIds.includes(providerToSave.id)) {
          providerToSave.id = generateId();
        }
      }
      await onSave(providerToSave);
      setSaveResult('success');
    } catch (err) {
      setSaveResult('error');
    } finally {
      setSaving(false);
    }
  };

  const handleCheck = async () => {
    setChecking(true);
    setCheckResult(null);
    try {
      // 确定要测试的模型 ID
      let modelIdToTest: string | undefined = selectedModelId || undefined;

      const result = await onCheck(editedProvider.id, testPrompt || undefined, modelIdToTest, editedProvider);

      // 如果有媒体文件，转换为 base64 data URL（兜底：如果后端已返回 dataUrl 则直接使用）
      let mediaDataUrl: string | undefined;
      let mediaError: string | undefined;
      if (result?.mediaUrl && !result?.mediaItems) {
        try {
          mediaDataUrl = await invoke<string>('read_file_as_data_url', { filePath: result.mediaUrl });
          console.log('[ProviderConfigModal] 媒体文件加载成功, 大小:', mediaDataUrl.length, '字符');
        } catch (e: any) {
          console.warn('[ProviderConfigModal] 读取媒体文件失败:', e, '路径:', result.mediaUrl);
          mediaError = typeof e === 'string' ? e : (e?.message || '读取失败');
        }
      }

      // 如果后端返回了 mediaItems，则直接使用（已包含 dataUrl 的音频可直接播放）
      const mediaItems: MediaItemData[] | undefined = result?.mediaItems;

      setCheckResult({
        status: result?.status || 'ok',
        message: result?.errorMessage,
        latency: result?.latency,
        responsePreview: result?.responsePreview,
        testPrompt: result?.testPrompt,
        mediaUrl: result?.mediaUrl,
        mediaDataUrl,
        mediaType: result?.mediaType,
        mediaError,
        pollingTaskId: result?.pollingTaskId,
        pollingStatus: result?.pollingStatus,
        pollingElapsedSecs: result?.pollingElapsedSecs,
        mediaItems,
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
      connected: '#38a169',
      configured: '#d69e2e',
      unconfigured: '#a0aec0',
      auth_failed: '#e53e3e',
      quota_exceeded: '#d69e2e',
      network_error: '#e53e3e',
      error: '#e53e3e',
    };
    return map[status] || '#a0aec0';
  };

  // --- Styles (明亮主题) ---
  const inputStyle: React.CSSProperties = {
    width: '100%', padding: '0.6rem 0.8rem', fontSize: '0.9rem',
    fontFamily: 'monospace', backgroundColor: '#ffffff', color: '#2d3748',
    border: '1px solid #d4cdc2', borderRadius: '10px', outline: 'none',
    boxSizing: 'border-box',
    transition: 'border-color 0.2s',
  };

  const labelStyle: React.CSSProperties = {
    display: 'block', fontSize: '0.85rem', color: '#4a5568', marginBottom: '0.4rem', fontWeight: 500,
  };

  const sectionHeaderStyle: React.CSSProperties = {
    fontSize: '0.8rem', fontWeight: 600, color: '#718096', textTransform: 'uppercase',
    letterSpacing: '0.05em', marginBottom: '0.75rem', paddingBottom: '0.4rem',
    borderBottom: '1px solid #e8e2d8',
  };

  const readOnlyStyle: React.CSSProperties = {
    ...inputStyle, color: '#a0aec0', backgroundColor: '#f5f0e8', cursor: 'default',
  };

  // --- 右侧测试结果面板 ---
  const renderTestPanel = () => {
    if (checking) {
      return (
        <div style={{
          display: 'flex', flexDirection: 'column', alignItems: 'center', justifyContent: 'center',
          height: '100%', color: '#718096', fontSize: '0.9rem', textAlign: 'center', padding: '2rem',
        }}>
          <div style={{
            width: '48px', height: '48px', border: '4px solid #e8e2d8',
            borderTopColor: '#e07a2f', borderRadius: '50%',
            animation: 'spin 1s linear infinite', marginBottom: '1.2rem',
          }} />
          <div style={{ fontWeight: 500, fontSize: '1rem', color: '#4a5568' }}>正在测试连接...</div>
          <div style={{ fontSize: '0.8rem', marginTop: '0.5rem', color: '#a0aec0' }}>
            {editedProvider?.modality?.includes('music')
              ? 'AI 正在生成音乐，可能需要 1-3 分钟，请耐心等待'
              : '正在发送请求并等待响应'}
          </div>
          <div style={{
            fontSize: '0.75rem', marginTop: '1rem', padding: '0.4rem 0.8rem',
            backgroundColor: '#f5f0e8', borderRadius: '6px', color: '#718096',
            fontFamily: 'monospace', maxWidth: '80%', wordBreak: 'break-all',
          }}>
            服务商: {editedProvider?.name || editedProvider?.id || ''}
          </div>
        </div>
      );
    }

    if (!checkResult) {
      return (
        <div style={{
          display: 'flex', flexDirection: 'column', alignItems: 'center', justifyContent: 'center',
          height: '100%', color: '#a0aec0', fontSize: '0.9rem', textAlign: 'center', padding: '2rem',
        }}>
          <div style={{
            width: '56px', height: '56px', borderRadius: '14px',
            backgroundColor: '#f5f0e8', display: 'flex', alignItems: 'center', justifyContent: 'center',
            marginBottom: '1rem', fontSize: '1.5rem',
          }}>&#9881;</div>
          <div style={{ color: '#718096', fontWeight: 500 }}>接口调用信息</div>
          <div style={{ fontSize: '0.8rem', marginTop: '0.4rem', lineHeight: 1.5 }}>
            点击左侧"测试连接"按钮后<br/>请求和响应详情将在此处显示
          </div>
        </div>
      );
    }

    const isSuccess = checkResult.status === 'ok';
    const hasMultipleMedia = checkResult.mediaItems && checkResult.mediaItems.length > 0;

    return (
      <div style={{ padding: '1rem', overflowY: 'auto', height: '100%' }}>
        {/* 状态横幅 */}
        <div style={{
          padding: '0.8rem 1rem', borderRadius: '8px', marginBottom: '1rem',
          backgroundColor: isSuccess ? 'rgba(56,161,105,0.08)' : 'rgba(229,62,62,0.08)',
          border: `2px solid ${isSuccess ? 'rgba(56,161,105,0.3)' : 'rgba(229,62,62,0.3)'}`,
        }}>
          <div style={{ fontWeight: 600, fontSize: '1rem', color: isSuccess ? '#38a169' : '#e53e3e' }}>
            {isSuccess ? '✓ 连接成功' : '✗ 连接失败'}
          </div>
          {checkResult.latency != null && (
            <div style={{ color: '#718096', fontSize: '0.85rem', marginTop: '0.2rem' }}>
              延迟: {checkResult.latency}ms
              {checkResult.latency < 500 ? ' (很快)' : checkResult.latency < 60000 ? ' (正常)' : ' (较慢)'}
            </div>
          )}
          {!isSuccess && checkResult.message && (
            <div style={{ color: '#e53e3e', fontSize: '0.85rem', marginTop: '0.25rem', wordBreak: 'break-word' }}>
              {checkResult.message}
            </div>
          )}
          {/* 轮询任务信息 */}
          {checkResult.pollingTaskId && (
            <div style={{
              marginTop: '0.5rem', padding: '0.4rem 0.6rem',
              backgroundColor: 'rgba(224,122,47,0.06)', borderRadius: '6px',
              fontSize: '0.78rem', color: '#718096', fontFamily: 'monospace',
            }}>
              <div>任务ID: {checkResult.pollingTaskId}</div>
              {checkResult.pollingElapsedSecs != null && (
                <div>生成用时: {checkResult.pollingElapsedSecs}s</div>
              )}
              {hasMultipleMedia && (
                <div>生成结果: {checkResult.mediaItems!.length} 项</div>
              )}
            </div>
          )}
        </div>

        {/* 请求详情 */}
        <div style={{ marginBottom: '0.75rem' }}>
          <div style={sectionHeaderStyle}>请求详情</div>

          {checkResult.requestEndpoint && (
            <div style={{ marginBottom: '0.5rem' }}>
              <div style={{ fontSize: '0.75rem', color: '#718096', marginBottom: '0.15rem' }}>接口地址</div>
              <div style={{
                padding: '0.4rem 0.6rem', backgroundColor: '#f5f0e8', borderRadius: '6px',
                fontSize: '0.8rem', color: '#4a5568', fontFamily: 'monospace',
                wordBreak: 'break-all', border: '1px solid #e8e2d8',
              }}>
                {checkResult.requestEndpoint}
              </div>
            </div>
          )}

          {checkResult.requestModel && (
            <div style={{ marginBottom: '0.5rem' }}>
              <div style={{ fontSize: '0.75rem', color: '#718096', marginBottom: '0.15rem' }}>模型</div>
              <div style={{
                padding: '0.4rem 0.6rem', backgroundColor: '#f5f0e8', borderRadius: '6px',
                fontSize: '0.8rem', color: '#4a5568', fontFamily: 'monospace',
                border: '1px solid #e8e2d8',
              }}>
                {checkResult.requestModel}
              </div>
            </div>
          )}

          {checkResult.testPrompt && (
            <div style={{ marginBottom: '0.5rem' }}>
              <div style={{ fontSize: '0.75rem', color: '#718096', marginBottom: '0.15rem' }}>提示词</div>
              <div style={{
                padding: '0.4rem 0.6rem', backgroundColor: '#f5f0e8', borderRadius: '6px',
                fontSize: '0.8rem', color: '#4a5568', fontFamily: 'monospace',
                border: '1px solid #e8e2d8', whiteSpace: 'pre-wrap', wordBreak: 'break-word',
              }}>
                {checkResult.testPrompt}
              </div>
            </div>
          )}

          {checkResult.requestHeaders && (
            <div style={{ marginBottom: '0.5rem' }}>
              <div style={{ fontSize: '0.75rem', color: '#718096', marginBottom: '0.15rem' }}>请求头</div>
              <pre style={{
                padding: '0.5rem 0.6rem', backgroundColor: '#f5f0e8', borderRadius: '6px',
                fontSize: '0.75rem', color: '#718096', fontFamily: 'monospace',
                border: '1px solid #e8e2d8', margin: 0, whiteSpace: 'pre-wrap', wordBreak: 'break-all',
                maxHeight: '120px', overflowY: 'auto',
              }}>
                {formatJson(checkResult.requestHeaders)}
              </pre>
            </div>
          )}

          {checkResult.requestBody && (
            <div style={{ marginBottom: '0.5rem' }}>
              <div style={{ fontSize: '0.75rem', color: '#718096', marginBottom: '0.15rem' }}>请求体</div>
              <pre style={{
                padding: '0.5rem 0.6rem', backgroundColor: '#f5f0e8', borderRadius: '6px',
                fontSize: '0.75rem', color: '#718096', fontFamily: 'monospace',
                border: '1px solid #e8e2d8', margin: 0, whiteSpace: 'pre-wrap', wordBreak: 'break-all',
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
              <div style={{ fontSize: '0.75rem', color: '#718096', marginBottom: '0.15rem' }}>状态码</div>
              <span style={{
                display: 'inline-block', padding: '0.2rem 0.6rem', borderRadius: '6px',
                fontSize: '0.85rem', fontWeight: 600, fontFamily: 'monospace',
                backgroundColor: checkResult.responseStatus >= 200 && checkResult.responseStatus < 300
                  ? 'rgba(56,161,105,0.08)' : 'rgba(229,62,62,0.08)',
                color: checkResult.responseStatus >= 200 && checkResult.responseStatus < 300
                  ? '#38a169' : '#e53e3e',
              }}>
                {checkResult.responseStatus}
              </span>
            </div>
          )}

          {/* 文本响应 */}
          {checkResult.responsePreview && (
            <div style={{ marginBottom: '0.5rem' }}>
              <div style={{ fontSize: '0.75rem', color: '#718096', marginBottom: '0.15rem' }}>AI 响应</div>
              <div style={{
                padding: '0.6rem', backgroundColor: '#f5f0e8', borderRadius: '8px',
                fontSize: '0.85rem', color: '#2d3748', lineHeight: 1.6,
                border: '1px solid #e8e2d8',
                maxHeight: '200px', overflowY: 'auto',
                whiteSpace: 'pre-wrap', wordBreak: 'break-word',
              }}>
                {checkResult.responsePreview}
              </div>
            </div>
          )}

          {/* 多媒体卡片（妙音 AI 多首音乐 + 封面） */}
          {hasMultipleMedia && (
            <div style={{ marginBottom: '1rem' }}>
              <div style={{ fontSize: '0.75rem', color: '#718096', marginBottom: '0.5rem', fontWeight: 600 }}>
                🎵 生成结果 ({checkResult.mediaItems!.length})
              </div>
              <div style={{
                display: 'grid', gridTemplateColumns: 'repeat(auto-fill, minmax(180px, 1fr))',
                gap: '0.75rem',
              }}>
                {checkResult.mediaItems!.map((item, idx) => {
                  const statusLabel = item.status || '';
                  const isComplete = statusLabel === 'complete';
                  const isFailed = statusLabel === 'failed';
                  const cardBorderColor = isComplete ? 'rgba(56,161,105,0.3)'
                    : isFailed ? 'rgba(229,62,62,0.3)'
                    : '#e8e2d8';
                  const audioSrc = item.dataUrl || item.audioUrl || item.mediaUrl;
                  return (
                    <div key={item.id || idx} style={{
                      border: `1px solid ${cardBorderColor}`,
                      borderRadius: '10px',
                      overflow: 'hidden',
                      backgroundColor: '#fff',
                      display: 'flex', flexDirection: 'column',
                    }}>
                      {/* 封面图 */}
                      <div style={{
                        aspectRatio: '1/1', backgroundColor: '#f5f0e8',
                        display: 'flex', alignItems: 'center', justifyContent: 'center',
                        overflow: 'hidden',
                      }}>
                        {item.imageUrl ? (
                          <img
                            src={item.imageUrl}
                            alt={item.title || `Track ${idx + 1}`}
                            style={{ width: '100%', height: '100%', objectFit: 'cover' }}
                          />
                        ) : (
                          <span style={{ fontSize: '2rem', opacity: 0.4 }}>🎵</span>
                        )}
                      </div>
                      {/* 信息区 */}
                      <div style={{ padding: '0.6rem' }}>
                        <div style={{
                          fontSize: '0.85rem', fontWeight: 600, color: '#2d3748',
                          marginBottom: '0.25rem',
                          overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap',
                        }}>
                          {item.title || `Track ${idx + 1}`}
                        </div>
                        {item.tags && (
                          <div style={{
                            fontSize: '0.7rem', color: '#718096', marginBottom: '0.4rem',
                            overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap',
                          }}>
                            {item.tags}
                          </div>
                        )}
                        {/* 状态标签 */}
                        {statusLabel && (
                          <div style={{
                            display: 'inline-block', padding: '0.15rem 0.4rem',
                            fontSize: '0.7rem', borderRadius: '4px',
                            backgroundColor: isComplete ? 'rgba(56,161,105,0.1)'
                              : isFailed ? 'rgba(229,62,62,0.1)'
                              : 'rgba(224,122,47,0.1)',
                            color: isComplete ? '#38a169'
                              : isFailed ? '#e53e3e'
                              : '#e07a2f',
                            marginBottom: '0.4rem',
                          }}>
                            {statusLabel}
                          </div>
                        )}
                        {/* 音频播放器 */}
                        {audioSrc && (
                          <audio
                            controls
                            src={audioSrc}
                            style={{ width: '100%', height: '32px', marginTop: '0.3rem' }}
                            preload="metadata"
                          >
                            <p>浏览器不支持音频</p>
                          </audio>
                        )}
                        {/* 错误信息 */}
                        {item.error && (
                          <div style={{
                            marginTop: '0.3rem', fontSize: '0.7rem', color: '#e53e3e',
                            wordBreak: 'break-word',
                          }}>
                            {item.error}
                          </div>
                        )}
                        {/* 无音频提示 */}
                        {!audioSrc && !item.error && (
                          <div style={{
                            marginTop: '0.3rem', fontSize: '0.7rem', color: '#a0aec0',
                          }}>
                            暂无音频
                          </div>
                        )}
                      </div>
                    </div>
                  );
                })}
              </div>
            </div>
          )}

          {/* 图片（单张图片 fallback，多媒体时不显示） */}
          {!hasMultipleMedia && checkResult.mediaDataUrl && checkResult.mediaType === 'image' && (
            <div style={{ marginBottom: '0.5rem' }}>
              <div style={{ fontSize: '0.75rem', color: '#718096', marginBottom: '0.25rem' }}>测试图片</div>
              <img
                src={checkResult.mediaDataUrl}
                alt="Test result"
                style={{ maxWidth: '100%', maxHeight: '250px', borderRadius: '10px', border: '1px solid #e8e2d8' }}
              />
            </div>
          )}

          {/* 音频（单个音频 fallback，多媒体时不显示） */}
          {!hasMultipleMedia && checkResult.mediaType === 'audio' && (
            <div style={{ marginBottom: '0.5rem' }}>
              <div style={{ fontSize: '0.75rem', color: '#718096', marginBottom: '0.25rem' }}>测试音频</div>
              {checkResult.mediaDataUrl ? (
                <audio
                  key={checkResult.mediaDataUrl.substring(0, 50)}
                  controls
                  src={checkResult.mediaDataUrl}
                  style={{ width: '100%' }}
                  onError={(e) => {
                    const audioEl = e.currentTarget;
                    console.warn('[ProviderConfigModal] 音频播放失败, error:', audioEl.error?.code, audioEl.error?.message);
                  }}
                >
                  <p>您的浏览器不支持音频播放</p>
                </audio>
              ) : checkResult.mediaError ? (
                <div style={{
                  padding: '0.5rem 0.6rem', backgroundColor: 'rgba(229,62,62,0.06)',
                  borderRadius: '6px', fontSize: '0.8rem', color: '#e53e3e',
                  border: '1px solid rgba(229,62,62,0.2)',
                }}>
                  音频加载失败: {checkResult.mediaError}
                  {checkResult.mediaUrl && (
                    <div style={{ fontSize: '0.7rem', color: '#718096', marginTop: '0.3rem' }}>
                      文件路径: {checkResult.mediaUrl}
                    </div>
                  )}
                </div>
              ) : (
                <div style={{
                  padding: '0.5rem', color: '#a0aec0', fontSize: '0.8rem',
                }}>
                  音频数据不可用
                </div>
              )}
            </div>
          )}

          {/* 视频（单个视频 fallback） */}
          {!hasMultipleMedia && checkResult.mediaDataUrl && checkResult.mediaType === 'video' && (
            <div style={{ marginBottom: '0.5rem' }}>
              <div style={{ fontSize: '0.75rem', color: '#718096', marginBottom: '0.25rem' }}>测试视频</div>
              <video controls src={checkResult.mediaDataUrl} style={{ maxWidth: '100%', maxHeight: '250px', borderRadius: '8px' }} />
            </div>
          )}
        </div>
      </div>
    );
  };

  // --- 渲染新建流程的模态选择和服务商选择 ---
  const renderNewProviderFlow = () => {
    // 如果已选择内置服务商，显示完整表单
    if (selectedBuiltinId && editedProvider.models.length > 0) {
      return null; // 返回 null 表示显示正常表单
    }

    return (
      <>
        {/* 选择模态 */}
        <div style={{ marginBottom: '1.25rem' }}>
          <div style={sectionHeaderStyle}>选择模态</div>
          <div style={{ display: 'flex', flexWrap: 'wrap', gap: '0.5rem' }}>
            {MODALITY_OPTIONS.map(opt => (
              <button
                key={opt.value}
                onClick={() => handleNewModalitySelect(opt.value)}
                style={{
                  padding: '0.6rem 1.2rem',
                  fontSize: '0.9rem',
                  borderRadius: '10px',
                  cursor: 'pointer',
                  border: selectedModality === opt.value
                    ? '2px solid #e07a2f'
                    : '1px solid #d4cdc2',
                  backgroundColor: selectedModality === opt.value
                    ? 'rgba(224,122,47,0.08)'
                    : '#ffffff',
                  color: selectedModality === opt.value
                    ? '#e07a2f'
                    : '#4a5568',
                  fontWeight: selectedModality === opt.value ? 600 : 400,
                  transition: 'all 0.2s',
                }}
              >
                {opt.label}
              </button>
            ))}
          </div>
        </div>

        {/* 选择服务商 */}
        {selectedModality && (
          <div style={{ marginBottom: '1.25rem' }}>
            <div style={sectionHeaderStyle}>选择服务商</div>
            {filteredBuiltinProviders.length === 0 ? (
              <div style={{
                padding: '1rem', textAlign: 'center',
                color: '#a0aec0', fontSize: '0.85rem',
                backgroundColor: '#f5f0e8', borderRadius: '10px',
              }}>
                该模态暂无内置服务商
              </div>
            ) : (
              <div style={{ display: 'flex', flexDirection: 'column', gap: '0.5rem' }}>
                {filteredBuiltinProviders.map(p => (
                  <div
                    key={p.id}
                    onClick={() => handleBuiltinProviderSelect(p.id)}
                    style={{
                      padding: '0.8rem 1rem',
                      borderRadius: '10px',
                      cursor: 'pointer',
                      border: selectedBuiltinId === p.id
                        ? '2px solid #e07a2f'
                        : '1px solid #d4cdc2',
                      backgroundColor: selectedBuiltinId === p.id
                        ? 'rgba(224,122,47,0.06)'
                        : '#ffffff',
                      transition: 'all 0.2s',
                    }}
                    onMouseEnter={e => {
                      if (selectedBuiltinId !== p.id) e.currentTarget.style.backgroundColor = '#f5f0e8';
                    }}
                    onMouseLeave={e => {
                      if (selectedBuiltinId !== p.id) e.currentTarget.style.backgroundColor = '#ffffff';
                    }}
                  >
                    <div style={{ fontWeight: 600, fontSize: '0.95rem', color: '#2d3748', marginBottom: '0.2rem' }}>
                      {p.name}
                    </div>
                    <div style={{ fontSize: '0.8rem', color: '#718096', lineHeight: 1.4 }}>
                      {p.description}
                    </div>
                    <div style={{ fontSize: '0.75rem', color: '#a0aec0', marginTop: '0.3rem' }}>
                      模型数量: {p.models.filter(m => m.modality === selectedModality).length}
                    </div>
                  </div>
                ))}
              </div>
            )}
          </div>
        )}
      </>
    );
  };

  // 判断是否显示新建流程（未选择内置服务商时）
  const showNewProviderFlow = isNew && !selectedBuiltinId;

  return (
    <div style={{
      position: 'fixed', inset: 0,
      backgroundColor: 'rgba(45, 55, 72, 0.3)',
      display: 'flex', alignItems: 'center', justifyContent: 'center',
      zIndex: 1000,
      backdropFilter: 'blur(4px)', WebkitBackdropFilter: 'blur(4px)',
    }} onClick={e => { if (e.target === e.currentTarget) handleClose(); }}>
      <div style={{
        backgroundColor: '#ffffff',
        border: '1px solid #e8e2d8',
        borderRadius: '16px',
        width: '95%', maxWidth: '1100px',
        maxHeight: '88vh',
        display: 'flex',
        transition: 'max-width 0.3s ease',
        boxShadow: '0 8px 32px rgba(45, 55, 72, 0.15), 0 2px 8px rgba(45, 55, 72, 0.08)',
      }} onClick={e => e.stopPropagation()}>
        {/* ===== 左侧：配置表单 ===== */}
        <div style={{
          flex: '1 1 0',
          minWidth: 0,
          padding: '1.5rem',
          overflowY: 'auto',
          borderRight: '1px solid #e8e2d8',
        }}>
          {/* Header */}
          <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'flex-start', marginBottom: '1.25rem' }}>
            <div>
              <h3 style={{ fontSize: '1.1rem', fontWeight: 600, color: '#2d3748', marginBottom: '0.2rem' }}>
                {isNew ? (selectedBuiltinId ? '配置服务商' : '添加服务商') : editedProvider.name}
              </h3>
              {!isNew && (
                <span style={{
                  display: 'inline-flex', alignItems: 'center', gap: '6px',
                  fontSize: '0.75rem', color: '#718096',
                }}>
                  <span style={{
                    width: '7px', height: '7px', borderRadius: '50%',
                    backgroundColor: getStatusColor(editedProvider.status),
                  }} />
                  {getStatusLabel(editedProvider.status)}
                </span>
              )}
              {isNew && selectedBuiltinId && (
                <button
                  className="btn btn-secondary"
                  onClick={() => {
                    setSelectedBuiltinId('');
                    setEditedProvider(prev => prev ? {
                      ...prev,
                      modality: selectedModality ? [selectedModality] : prev.modality,
                      models: [],
                    } : prev);
                    setSelectedModelId('');
                    setCheckResult(null);
                  }}
                  style={{ padding: '0.3rem 0.8rem', fontSize: '0.8rem' }}
                >
                  ← 重新选择
                </button>
              )}
            </div>
            <button
              onClick={handleClose}
              style={{
                background: 'none', border: 'none', color: '#a0aec0', fontSize: '1.1rem',
                cursor: 'pointer', padding: '0.2rem',
              }}
            >
              ✕
            </button>
          </div>

          {/* ===== 新建流程：模态选择 + 服务商选择 ===== */}
          {showNewProviderFlow && renderNewProviderFlow()}

          {/* ===== 已选择服务商或编辑模式：显示完整表单 ===== */}
          {!showNewProviderFlow && (
            <>
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

                <div style={{ marginBottom: '0.75rem' }}>
                  <label style={labelStyle}>模型能力</label>
                  <div style={{ display: 'flex', flexWrap: 'wrap', gap: '0.6rem' }}>
                    {MODALITY_OPTIONS.map(opt => (
                      <span key={opt.value} style={{
                        display: 'inline-flex', alignItems: 'center', gap: '0.3rem',
                        fontSize: '0.8rem', color: '#4a5568',
                        padding: '0.35rem 0.7rem',
                        backgroundColor: editedProvider.modality.includes(opt.value) ? 'rgba(224,122,47,0.08)' : '#f5f0e8',
                        borderRadius: '6px',
                        border: editedProvider.modality.includes(opt.value) ? '1px solid rgba(224,122,47,0.3)' : '1px solid #e8e2d8',
                      }}>
                        {editedProvider.modality.includes(opt.value) && (
                          <span style={{ color: '#e07a2f', fontWeight: 600 }}>✓</span>
                        )}
                        {opt.label}
                      </span>
                    ))}
                  </div>
                  {isNew && (
                    <span style={{ fontSize: '0.7rem', color: '#a0aec0', marginTop: '0.3rem', display: 'block' }}>
                      选择服务商后可更改
                    </span>
                  )}
                </div>
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
                  <span style={{ fontSize: '0.7rem', color: '#a0aec0', marginTop: '0.2rem', display: 'block' }}>
                    {currentEndpoint.includes('/chat/completions') || currentEndpoint.includes('/completions')
                      ? '当前为完整 URL，调用时将直接使用此地址'
                      : '当前为基础 URL，调用时将自动追加 /chat/completions'}
                  </span>
                </div>

                {/* 模型ID - 统一逻辑：输入框 + 内置模型列表 */}
                <div style={{ marginBottom: '0.75rem' }}>
                  <label style={labelStyle}>模型 ID</label>
                  <input
                    type="text"
                    value={selectedModelId || ''}
                    onChange={e => {
                      const val = e.target.value;
                      setSelectedModelId(val);
                      // 同步更新 provider 的默认模型 ID
                      setEditedProvider(prev => {
                        if (!prev) return prev;
                        const existing = prev.models.find(m => m.id === val);
                        if (existing) {
                          return { ...prev, models: prev.models.map(m => ({ ...m, isDefault: m.id === val })) };
                        }
                        // 自定义模型：更新第一个模型的 id 或新增
                        if (prev.models.length > 0) {
                          return {
                            ...prev,
                            models: prev.models.map((m, i) => i === 0 ? { ...m, id: val, name: val, isDefault: true } : m),
                          };
                        }
                        return {
                          ...prev,
                          models: [{
                            id: val, name: val, modality: prev.modality[0] || 'text',
                            isDefault: true, endpoint: currentEndpoint, quality: 'standard' as const,
                          }],
                        };
                      });
                    }}
                    placeholder="输入模型 ID"
                    style={inputStyle}
                  />
                  {/* 内置模型列表（点击填入输入框） */}
                  {(() => {
                    const builtinModels = isNew
                      ? (builtinTemplates.find(t => t.id === selectedBuiltinId)?.models || []).filter(m => m.modality === editedProvider.modality[0])
                      : (provider?.models || []);
                    if (builtinModels.length === 0) return null;
                    return (
                      <div style={{ marginTop: '0.4rem' }}>
                        <div style={{ fontSize: '0.75rem', color: '#a0aec0', marginBottom: '0.2rem' }}>
                          内置模型（点击选择）：
                        </div>
                        {builtinModels.map(model => (
                          <div key={model.id} style={{
                            display: 'flex', alignItems: 'center', gap: '0.3rem',
                            padding: '0.4rem 0.6rem', marginBottom: '0.2rem',
                            backgroundColor: model.id === selectedModelId ? 'rgba(224,122,47,0.08)' : '#f5f0e8',
                            borderRadius: '8px', fontSize: '0.85rem',
                            border: model.id === selectedModelId ? '1px solid rgba(224,122,47,0.2)' : '1px solid #e8e2d8',
                            cursor: 'pointer',
                          }}
                            onClick={() => {
                              setSelectedModelId(model.id);
                              setEditedProvider(prev => {
                                if (!prev) return prev;
                                // 如果模型已在列表中，标记为默认
                                if (prev.models.some(m => m.id === model.id)) {
                                  return { ...prev, models: prev.models.map(m => ({ ...m, isDefault: m.id === model.id })) };
                                }
                                // 否则替换第一个模型
                                return {
                                  ...prev,
                                  models: prev.models.map((m, i) => i === 0 ? { ...model, isDefault: true } : m),
                                };
                              });
                            }}
                          >
                            <span style={{ flex: 1, fontFamily: 'monospace', color: '#2d3748' }}>
                              {model.name !== model.id ? `${model.name} (${model.id})` : model.id}
                            </span>
                            {model.freeQuota && <span style={{ color: model.freeQuota === '付费' ? '#e53e3e' : '#38a169', fontSize: '0.7rem' }}>{model.freeQuota}</span>}
                          </div>
                        ))}
                      </div>
                    );
                  })()}
                  {/* 模型说明 */}
                  {selectedModelId && (() => {
                    const sm = editedProvider.models.find(m => m.id === selectedModelId);
                    if (!sm) return null;
                    return (
                      <div style={{
                        marginTop: '0.4rem', padding: '0.5rem 0.6rem',
                        backgroundColor: '#f5f0e8', borderRadius: '8px',
                        fontSize: '0.75rem', color: '#718096', border: '1px solid #e8e2d8',
                      }}>
                        {sm.name !== sm.id && <div>名称: {sm.name}</div>}
                        <div>模态: {MODALITY_OPTIONS.find(o => o.value === sm.modality)?.label || sm.modality}</div>
                        {sm.quality && <div>质量: {sm.quality === 'fast' ? '快速' : sm.quality === 'standard' ? '标准' : '高质量'}</div>}
                        {sm.freeQuota && <div style={{ color: sm.freeQuota === '付费' ? '#e53e3e' : '#38a169' }}>{sm.freeQuota === '付费' ? '付费' : '免费额度'}: {sm.freeQuota}</div>}
                        {sm.maxTokens && <div>最大 Token: {sm.maxTokens}</div>}
                      </div>
                    );
                  })()}
                </div>
              </div>

              {/* ===== Section: 高级参数（图片模型） ===== */}
              {selectedModel?.modality === 'image' && (() => {
                const advParams = selectedModel?.advancedParams || {};
                const paramDefs = [
                  { key: 'guidance_scale', label: '提示词相关性', fieldKey: 'guidance_scale', placeholder: '默认 7.5', defaultVal: 7.5, min: 0, max: 20, step: 0.1,
                    tip: '指生成的图片参考提示词的程度大小，相关性越高，参考提示词越严格。范围：0~20.0' },
                  { key: 'num_inference_steps', label: '采样步数', fieldKey: 'num_inference_steps', placeholder: '默认 20', defaultVal: 20, min: 0, max: 100, step: 1,
                    tip: '采样迭代步数是决定图像细节和质量的关键，步数越多，图像越精细。但过多的步数会导致资源消耗增加和生成速度变慢。范围：0~100' },
                  { key: 'seed', label: '随机种子', fieldKey: 'seed', placeholder: '留空则不传', min: 0, max: 9999999999, step: 1,
                    tip: '可随机生成一个随机数作为图像生成的起点。范围：0~9999999999', hasRandomBtn: true },
                ];
                return (
                  <div style={{ marginBottom: '1.25rem' }}>
                    <div style={sectionHeaderStyle}>高级参数</div>
                    <div style={{
                      padding: '0.5rem 0.7rem', marginBottom: '0.6rem',
                      backgroundColor: 'rgba(224,122,47,0.06)', border: '1px solid rgba(224,122,47,0.2)',
                      borderRadius: '8px', fontSize: '0.75rem', color: '#718096', lineHeight: 1.5,
                    }}>
                      不填则使用默认值或接口不传。清空输入框可删除已配置的参数。
                    </div>
                    {paramDefs.map(def => {
                      const currentVal = advParams[def.key];
                      return (
                        <div key={def.key} style={{ marginBottom: '0.6rem' }}>
                          <div style={{ display: 'flex', alignItems: 'center', gap: '0.3rem', marginBottom: '0.4rem' }}>
                            <label style={{ ...labelStyle, marginBottom: 0 }}>
                              {def.label}
                            </label>
                            <span style={{ fontSize: '0.75rem', color: '#a0aec0', fontWeight: 400 }}>
                              ({def.fieldKey})
                            </span>
                            {def.defaultVal != null && (
                              <span style={{ fontSize: '0.75rem', color: '#a0aec0', fontWeight: 400 }}>
                                默认 {def.defaultVal}
                              </span>
                            )}
                            <span
                              onClick={() => setTipOpen(prev => ({ ...prev, [def.key]: !prev[def.key] }))}
                              style={{
                                display: 'inline-flex', alignItems: 'center', justifyContent: 'center',
                                width: '16px', height: '16px', borderRadius: '50%',
                                backgroundColor: tipOpen[def.key] ? 'rgba(224,122,47,0.15)' : 'rgba(113,128,150,0.1)',
                                color: tipOpen[def.key] ? '#e07a2f' : '#a0aec0',
                                fontSize: '0.65rem', cursor: 'pointer', userSelect: 'none',
                                transition: 'all 0.2s',
                              }}
                              title="查看说明"
                            >
                              ?
                            </span>
                          </div>
                          {tipOpen[def.key] && (
                            <div style={{
                              padding: '0.4rem 0.6rem', marginBottom: '0.4rem',
                              backgroundColor: 'rgba(224,122,47,0.06)', border: '1px solid rgba(224,122,47,0.15)',
                              borderRadius: '6px', fontSize: '0.72rem', color: '#718096', lineHeight: 1.5,
                            }}>
                              {def.tip}
                            </div>
                          )}
                          <div style={{ display: 'flex', gap: '0.4rem', alignItems: 'center' }}>
                            <input
                              type="number" step={def.step}
                              min={def.min} max={def.max}
                              value={currentVal != null ? String(currentVal) : ''}
                              onChange={e => {
                                let val = e.target.value;
                                // 限制范围
                                if (val !== '' && def.max != null) {
                                  const num = Number(val);
                                  if (!isNaN(num) && num > def.max) val = String(def.max);
                                }
                                if (val !== '' && def.min != null) {
                                  const num = Number(val);
                                  if (!isNaN(num) && num < def.min) val = String(def.min);
                                }
                                handleAdvancedParamChange(def.key, val);
                              }}
                              placeholder={def.placeholder}
                              style={{ ...inputStyle, flex: 1 }}
                            />
                            {def.hasRandomBtn && (
                              <button
                                onClick={() => handleAdvancedParamChange(def.key, String(Math.floor(Math.random() * 9999999999)))}
                                style={{
                                  padding: '0.5rem 0.7rem', fontSize: '0.75rem', whiteSpace: 'nowrap',
                                  backgroundColor: 'rgba(224,122,47,0.08)', border: '1px solid rgba(224,122,47,0.2)',
                                  borderRadius: '10px', color: '#e07a2f', cursor: 'pointer',
                                }}
                                title="生成随机种子"
                              >
                                🎲
                              </button>
                            )}
                          </div>
                        </div>
                      );
                    })}
                  </div>
                );
              })()}

              {/* ===== Section: 语音参数（Edge TTS） ===== */}
              {selectedModel?.modality === 'voice' && (() => {
                const advParams = selectedModel?.advancedParams || {};
                const rateOptions = [
                  { value: '__default__', label: '默认 (+0%)' },
                  { value: '-50%', label: '-50% (很慢)' },
                  { value: '-20%', label: '-20% (较慢)' },
                  { value: '-10%', label: '-10% (稍慢)' },
                  { value: '+0%', label: '+0% (正常)' },
                  { value: '+10%', label: '+10% (稍快)' },
                  { value: '+20%', label: '+20% (较快)' },
                  { value: '+50%', label: '+50% (很快)' },
                  { value: '+100%', label: '+100% (极快)' },
                ];
                const pitchOptions = [
                  { value: '__default__', label: '默认 (+0Hz)' },
                  { value: '-20Hz', label: '-20Hz (很低)' },
                  { value: '-10Hz', label: '-10Hz (较低)' },
                  { value: '-5Hz', label: '-5Hz (稍低)' },
                  { value: '+0Hz', label: '+0Hz (正常)' },
                  { value: '+5Hz', label: '+5Hz (稍高)' },
                  { value: '+10Hz', label: '+10Hz (较高)' },
                  { value: '+20Hz', label: '+20Hz (很高)' },
                ];
                const volumeOptions = [
                  { value: '__default__', label: '默认 (medium)' },
                  { value: 'silent', label: 'silent (静音)' },
                  { value: 'x-soft', label: 'x-soft (极轻)' },
                  { value: 'soft', label: 'soft (轻声)' },
                  { value: 'medium', label: 'medium (中等)' },
                  { value: 'loud', label: 'loud (响亮)' },
                  { value: 'x-loud', label: 'x-loud (极响)' },
                ];
                const formatOptions = [
                  { value: '__default__', label: '默认 (24khz 96kbps)' },
                  { value: 'audio-24khz-48kbitrate-mono-mp3', label: '24khz 48kbps (文件小)' },
                  { value: 'audio-24khz-96kbitrate-mono-mp3', label: '24khz 96kbps (推荐)' },
                ];
                const voiceOptions = [
                  { value: '__default__', label: '默认 (Yunjian 男声)' },
                  { value: 'zh-CN-XiaoxiaoNeural', label: 'Xiaoxiao (女声，温柔)' },
                  { value: 'zh-CN-XiaoyiNeural', label: 'Xiaoyi (女声，活泼)' },
                  { value: 'zh-CN-XiaochenNeural', label: 'Xiaochen (女声，自然)' },
                  { value: 'zh-CN-XiaohanNeural', label: 'Xiaohan (女声)' },
                  { value: 'zh-CN-XiaomengNeural', label: 'Xiaomeng (女声)' },
                  { value: 'zh-CN-XiaomoNeural', label: 'Xiaomo (女声)' },
                  { value: 'zh-CN-XiaoqiuNeural', label: 'Xiaoqiu (女声)' },
                  { value: 'zh-CN-XiaoruiNeural', label: 'Xiaorui (女声)' },
                  { value: 'zh-CN-XiaoshuangNeural', label: 'Xiaoshuang (女声，甜美)' },
                  { value: 'zh-CN-XiaoxuanNeural', label: 'Xiaoxuan (女声)' },
                  { value: 'zh-CN-XiaoyanNeural', label: 'Xiaoyan (女声)' },
                  { value: 'zh-CN-XiaozhenNeural', label: 'Xiaozhen (女声)' },
                  { value: 'zh-CN-YunjianNeural', label: 'Yunjian (男声，沉稳)' },
                  { value: 'zh-CN-YunxiNeural', label: 'Yunxi (男声，年轻)' },
                  { value: 'zh-CN-YunxiaNeural', label: 'Yunxia (男声，少年)' },
                  { value: 'zh-CN-YunyangNeural', label: 'Yunyang (男声，新闻播音)' },
                  { value: 'zh-CN-YunfengNeural', label: 'Yunfeng (男声)' },
                  { value: 'zh-CN-YunhaoNeural', label: 'Yunhao (男声)' },
                  { value: 'zh-CN-YunjieNeural', label: 'Yunjie (男声)' },
                  { value: 'zh-CN-YunyeNeural', label: 'Yunye (男声)' },
                  { value: 'zh-CN-YunzeNeural', label: 'Yunze (男声)' },
                  { value: 'zh-CN-liaoning-XiaobeiNeural', label: 'Xiaobei (女声，辽宁方言)' },
                  { value: 'zh-CN-shaanxi-XiaoniNeural', label: 'Xiaoni (女声，陕西方言)' },
                  { value: 'zh-TW-HsiaoChenNeural', label: 'HsiaoChen (台湾女声)' },
                  { value: 'zh-TW-HsiaoYuNeural', label: 'HsiaoYu (台湾女声)' },
                  { value: 'zh-TW-YunJheNeural', label: 'YunJhe (台湾男声)' },
                  { value: 'zh-HK-HiuGaaiNeural', label: 'HiuGaai (粤语女声)' },
                  { value: 'zh-HK-HiuMaanNeural', label: 'HiuMaan (粤语女声)' },
                  { value: 'zh-HK-WanLungNeural', label: 'WanLung (粤语男声)' },
                ];
                const voiceParamDefs = [
                  { key: 'voice', label: '音色', tip: '选择不同的语音角色。默认 Yunjian（男声沉稳）。测试连接时使用选中的音色。', options: voiceOptions },
                  { key: 'rate', label: '语速', tip: '控制语音的播放速度。负值减慢，正值加快。范围：-50% ~ +100%', options: rateOptions },
                  { key: 'pitch', label: '音调', tip: '控制语音的音调高低。负值降低，正值升高。范围：-50Hz ~ +50Hz', options: pitchOptions },
                  { key: 'volume', label: '音量', tip: '控制语音的音量大小。支持预设值 (silent/x-soft/soft/medium/loud/x-loud) 或分贝值 (如 +10dB)', options: volumeOptions },
                  { key: 'output_format', label: '输出格式', tip: '音频编码格式和质量。仅支持 24khz 48kbps/96kbps。', options: formatOptions },
                ];
                return (
                  <div style={{ marginBottom: '1.25rem' }}>
                    <div style={sectionHeaderStyle}>语音参数</div>
                    <div style={{
                      padding: '0.5rem 0.7rem', marginBottom: '0.6rem',
                      backgroundColor: 'rgba(224,122,47,0.06)', border: '1px solid rgba(224,122,47,0.2)',
                      borderRadius: '8px', fontSize: '0.75rem', color: '#718096', lineHeight: 1.5,
                    }}>
                      不选则使用默认值。选择「默认」可清除已配置的参数。
                    </div>
                    {voiceParamDefs.map(def => {
                      const currentVal = advParams[def.key];
                      return (
                        <div key={def.key} style={{ marginBottom: '0.6rem' }}>
                          <div style={{ display: 'flex', alignItems: 'center', gap: '0.3rem', marginBottom: '0.4rem' }}>
                            <label style={{ ...labelStyle, marginBottom: 0 }}>{def.label}</label>
                            <span
                              onClick={() => setTipOpen(prev => ({ ...prev, [def.key]: !prev[def.key] }))}
                              style={{
                                display: 'inline-flex', alignItems: 'center', justifyContent: 'center',
                                width: '16px', height: '16px', borderRadius: '50%',
                                backgroundColor: tipOpen[def.key] ? 'rgba(224,122,47,0.15)' : 'rgba(113,128,150,0.1)',
                                color: tipOpen[def.key] ? '#e07a2f' : '#a0aec0',
                                fontSize: '0.65rem', cursor: 'pointer', userSelect: 'none',
                                transition: 'all 0.2s',
                              }}
                              title="查看说明"
                            >
                              ?
                            </span>
                          </div>
                          {tipOpen[def.key] && (
                            <div style={{
                              padding: '0.4rem 0.6rem', marginBottom: '0.4rem',
                              backgroundColor: 'rgba(224,122,47,0.06)', border: '1px solid rgba(224,122,47,0.15)',
                              borderRadius: '6px', fontSize: '0.72rem', color: '#718096', lineHeight: 1.5,
                            }}>
                              {def.tip}
                            </div>
                          )}
                          <select
                            value={currentVal != null ? String(currentVal) : '__default__'}
                            onChange={e => handleVoiceParamChange(def.key, e.target.value)}
                            style={{ ...inputStyle, width: '100%', fontFamily: 'monospace' }}
                          >
                            {def.options.map(opt => (
                              <option key={opt.value} value={opt.value}>{opt.label}</option>
                            ))}
                          </select>
                        </div>
                      );
                    })}
                  </div>
                );
              })()}

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
                      <span onClick={() => openUrl(editedProvider.authConfig.apiKey!.helpUrl!)}
                        style={{ display: 'inline-block', marginTop: '0.3rem', fontSize: '0.75rem', color: '#e07a2f', textDecoration: 'none', cursor: 'pointer' }}>
                        获取 API Key →
                      </span>
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
                  <span style={{ fontSize: '0.7rem', color: '#a0aec0', marginTop: '0.2rem', display: 'block' }}>
                    连接测试时发送给 AI 的提示词，仅对文本类服务商有效
                  </span>
                </div>
              </div>

              {/* Free Quota / 付费标识 */}
              {defaultModel?.freeQuota && (
                <div style={{
                  marginBottom: '1rem', padding: '0.6rem',
                  backgroundColor: defaultModel.freeQuota === '付费' ? 'rgba(229,62,62,0.06)' : 'rgba(56,161,105,0.06)',
                  border: `1px solid ${defaultModel.freeQuota === '付费' ? 'rgba(229,62,62,0.2)' : 'rgba(56,161,105,0.2)'}`,
                  borderRadius: '10px', fontSize: '0.8rem',
                  color: defaultModel.freeQuota === '付费' ? '#e53e3e' : '#38a169',
                }}>
                  {defaultModel.freeQuota === '付费' ? '付费模型' : '免费额度'}：{defaultModel.freeQuota}
                </div>
              )}

              {/* Registration Guide */}
              {!isNew && (editedProvider.officialUrl || editedProvider.registerUrl || editedProvider.docsUrl) && (
                <div style={{
                  marginBottom: '1rem', padding: '0.6rem',
                  backgroundColor: '#f5f0e8', borderRadius: '10px', fontSize: '0.8rem',
                }}>
                  <div style={{ color: '#718096', marginBottom: '0.3rem' }}>注册指引</div>
                  <div style={{ display: 'flex', flexDirection: 'column', gap: '0.2rem' }}>
                    {editedProvider.officialUrl && (
                      <span onClick={() => openUrl(editedProvider.officialUrl)}
                        style={{ color: '#e07a2f', textDecoration: 'none', fontSize: '0.75rem', cursor: 'pointer' }}>官方网站 →</span>
                    )}
                    {editedProvider.registerUrl && (
                      <span onClick={() => openUrl(editedProvider.registerUrl)}
                        style={{ color: '#e07a2f', textDecoration: 'none', fontSize: '0.75rem', cursor: 'pointer' }}>注册账号 →</span>
                    )}
                    {editedProvider.docsUrl && (
                      <span onClick={() => openUrl(editedProvider.docsUrl)}
                        style={{ color: '#e07a2f', textDecoration: 'none', fontSize: '0.75rem', cursor: 'pointer' }}>API 文档 →</span>
                    )}
                  </div>
                </div>
              )}

              {/* Registration Guide for new providers with builtin template */}
              {isNew && selectedBuiltinId && (editedProvider.officialUrl || editedProvider.registerUrl || editedProvider.docsUrl) && (
                <div style={{
                  marginBottom: '1rem', padding: '0.6rem',
                  backgroundColor: '#f5f0e8', borderRadius: '10px', fontSize: '0.8rem',
                }}>
                  <div style={{ color: '#718096', marginBottom: '0.3rem' }}>注册指引</div>
                  <div style={{ display: 'flex', flexDirection: 'column', gap: '0.2rem' }}>
                    {editedProvider.officialUrl && (
                      <span onClick={() => openUrl(editedProvider.officialUrl)}
                        style={{ color: '#e07a2f', textDecoration: 'none', fontSize: '0.75rem', cursor: 'pointer' }}>官方网站 →</span>
                    )}
                    {editedProvider.registerUrl && (
                      <span onClick={() => openUrl(editedProvider.registerUrl)}
                        style={{ color: '#e07a2f', textDecoration: 'none', fontSize: '0.75rem', cursor: 'pointer' }}>注册账号 →</span>
                    )}
                    {editedProvider.docsUrl && (
                      <span onClick={() => openUrl(editedProvider.docsUrl)}
                        style={{ color: '#e07a2f', textDecoration: 'none', fontSize: '0.75rem', cursor: 'pointer' }}>API 文档 →</span>
                    )}
                  </div>
                </div>
              )}

              {/* Save Result */}
              {saveResult && (
                <div style={{
                  marginBottom: '0.5rem', padding: '0.5rem 0.8rem',
                  backgroundColor: saveResult === 'success' ? 'rgba(56,161,105,0.08)' : 'rgba(229,62,62,0.08)',
                  border: `2px solid ${saveResult === 'success' ? 'rgba(56,161,105,0.3)' : 'rgba(229,62,62,0.3)'}`,
                  borderRadius: '10px', fontSize: '0.85rem',
                  color: saveResult === 'success' ? '#38a169' : '#e53e3e',
                  fontWeight: 600, display: 'flex', justifyContent: 'space-between', alignItems: 'center',
                }}>
                  <span>{saveResult === 'success' ? '✓ 保存成功' : '✗ 保存失败'}</span>
                  <button onClick={() => setSaveResult(null)}
                    style={{ background: 'none', border: 'none', color: '#a0aec0', cursor: 'pointer', fontSize: '0.75rem', padding: '0.1rem' }}>
                    ✕
                  </button>
                </div>
              )}

              {/* Actions */}
              <div style={{ display: 'flex', gap: '0.5rem', justifyContent: 'flex-end' }}>
                {isNew && selectedBuiltinId && (
                  <button className="btn btn-secondary" onClick={() => {
                    setSelectedBuiltinId('');
                    setSelectedModality(null);
                    setEditedProvider(prev => prev ? {
                      ...prev,
                      modality: ['text'],
                      models: [],
                    } : prev);
                    setSelectedModelId('');
                  }}
                    style={{ padding: '0.5rem 1rem', fontSize: '0.85rem', marginRight: 'auto' }}>
                    ← 重新选择
                  </button>
                )}
                <button className="btn btn-secondary" onClick={handleClose}
                  style={{ padding: '0.5rem 1rem', fontSize: '0.85rem' }}>
                  取消
                </button>
                <button className="btn btn-secondary" onClick={handleCheck}
                  disabled={checking || saving}
                  style={{ padding: '0.5rem 1rem', fontSize: '0.85rem' }}>
                  {checking ? '检测中...' : '测试连接'}
                </button>
                <button className="btn btn-primary" onClick={handleSave}
                  disabled={saving}
                  style={{ padding: '0.5rem 1rem', fontSize: '0.85rem' }}>
                  {saving ? '保存中...' : '保存'}
                </button>
              </div>
            </>
          )}
        </div>

        {/* ===== 右侧：测试结果面板（始终显示） ===== */}
        <div style={{
          flex: '0 0 420px', maxWidth: '420px',
          maxHeight: '88vh',
          overflowY: 'auto',
          backgroundColor: '#faf8f5',
          borderRadius: '0 16px 16px 0',
        }}>
          <div style={{
            padding: '0.8rem 1rem', borderBottom: '1px solid #e8e2d8',
            display: 'flex', justifyContent: 'space-between', alignItems: 'center',
          }}>
            <span style={{ fontSize: '0.85rem', fontWeight: 600, color: '#4a5568' }}>接口调用信息</span>
            {checkResult && (
              <button onClick={() => setCheckResult(null)}
                style={{ background: 'none', border: 'none', color: '#a0aec0', cursor: 'pointer', fontSize: '0.9rem', padding: '0.1rem' }}>
                ✕
              </button>
            )}
          </div>
          {renderTestPanel()}
        </div>
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
