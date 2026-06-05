import { useState } from 'react';
import type { ModalityAvailability } from '../../hooks/useConfig';

interface ModalitySelectModalProps {
  availability: ModalityAvailability;
  isOpen: boolean;
  onConfirm: (useLocalFallback: boolean) => void;
  onCancel: () => void;
}

const MODALITY_ITEMS: { key: keyof ModalityAvailability; label: string; icon: string }[] = [
  { key: 'text', label: '文本生成', icon: '📝' },
  { key: 'image', label: '图片生成', icon: '🖼️' },
  { key: 'video', label: '视频生成', icon: '🎬' },
  { key: 'music', label: '音乐生成', icon: '🎵' },
  { key: 'voice', label: '语音生成', icon: '🎙️' },
];

export default function ModalitySelectModal({
  availability,
  isOpen,
  onConfirm,
  onCancel,
}: ModalitySelectModalProps) {
  const [useLocalFallback, setUseLocalFallback] = useState(true);

  if (!isOpen) return null;

  const hasAnyMissing = MODALITY_ITEMS.some(item => !availability[item.key]);

  if (!hasAnyMissing) return null;

  return (
    <div
      style={{
        position: 'fixed',
        inset: 0,
        backgroundColor: 'rgba(0,0,0,0.4)',
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
        zIndex: 1000,
        backdropFilter: 'blur(4px)', WebkitBackdropFilter: 'blur(4px)',
      }}
      onClick={(e) => { if (e.target === e.currentTarget) onCancel(); }}
    >
      <div
        style={{
          backgroundColor: 'rgba(26, 35, 50, 0.95)',
          border: '1px solid #2a3a4e',
          borderRadius: '16px',
          padding: '2rem',
          width: '90%',
          maxWidth: '480px',
          backdropFilter: 'blur(20px)', WebkitBackdropFilter: 'blur(20px)',
          boxShadow: '0 8px 32px rgba(0, 0, 0, 0.4), 0 2px 8px rgba(0, 0, 0, 0.2)',
        }}
        onClick={(e) => e.stopPropagation()}
      >
        {/* Header */}
        <h3 style={{ fontSize: '1.2rem', fontWeight: 600, color: '#e8eaed', marginBottom: '0.5rem' }}>
          服务可用性检测
        </h3>
        <p style={{ fontSize: '0.85rem', color: '#7a8594', marginBottom: '1.25rem' }}>
          以下模态服务尚未配置或未连接，可能影响游戏生成效果
        </p>

        {/* Modality Status List */}
        <div style={{
          display: 'flex',
          flexDirection: 'column',
          gap: '0.6rem',
          marginBottom: '1.5rem',
        }}>
          {MODALITY_ITEMS.map(item => {
            const available = availability[item.key];
            return (
              <div
                key={item.key}
                style={{
                  display: 'flex',
                  alignItems: 'center',
                  justifyContent: 'space-between',
                  padding: '0.6rem 0.8rem',
                  backgroundColor: available ? 'rgba(34,197,94,0.08)' : 'rgba(239,68,68,0.08)',
                  border: `1px solid ${available ? 'rgba(34,197,94,0.2)' : 'rgba(239,68,68,0.2)'}`,
                  borderRadius: '8px',
                }}
              >
                <span style={{ display: 'flex', alignItems: 'center', gap: '0.5rem', fontSize: '0.9rem', color: '#e8eaed' }}>
                  <span>{item.icon}</span>
                  {item.label}
                </span>
                <span style={{
                  fontSize: '0.85rem',
                  fontWeight: 500,
                  color: available ? '#16a34a' : '#ef4444',
                }}>
                  {available ? '✓ 已配置' : '✗ 未配置'}
                </span>
              </div>
            );
          })}
        </div>

        {/* Fallback Options */}
        <div style={{ marginBottom: '1.5rem' }}>
          <p style={{ fontSize: '0.85rem', color: '#b0b8c4', marginBottom: '0.75rem' }}>
            缺失服务处理方式：
          </p>
          <div style={{ display: 'flex', flexDirection: 'column', gap: '0.5rem' }}>
            <label
              style={{
                display: 'flex',
                alignItems: 'flex-start',
                gap: '0.6rem',
                padding: '0.75rem',
                backgroundColor: useLocalFallback ? 'rgba(201,102,241,0.1)' : 'rgba(26, 35, 50, 0.6)',
                border: `1px solid ${useLocalFallback ? 'rgba(201,102,241,0.3)' : '#2a3a4e'}`,
                borderRadius: '8px',
                cursor: 'pointer',
                transition: 'all 0.15s',
              }}
              onClick={() => setUseLocalFallback(true)}
            >
              <input
                type="radio"
                name="fallback-option"
                checked={useLocalFallback}
                onChange={() => setUseLocalFallback(true)}
                style={{ marginTop: '2px' }}
              />
              <div>
                <div style={{ fontSize: '0.9rem', color: '#e8eaed', fontWeight: 500 }}>
                  使用本地资源替代缺失服务
                </div>
                <div style={{ fontSize: '0.8rem', color: '#7a8594', marginTop: '0.2rem' }}>
                  缺失的模态将使用内置默认资源，游戏可正常体验
                </div>
              </div>
            </label>
            <label
              style={{
                display: 'flex',
                alignItems: 'flex-start',
                gap: '0.6rem',
                padding: '0.75rem',
                backgroundColor: !useLocalFallback ? 'rgba(201,102,241,0.1)' : 'rgba(26, 35, 50, 0.6)',
                border: `1px solid ${!useLocalFallback ? 'rgba(201,102,241,0.3)' : '#2a3a4e'}`,
                borderRadius: '8px',
                cursor: 'pointer',
                transition: 'all 0.15s',
              }}
              onClick={() => setUseLocalFallback(false)}
            >
              <input
                type="radio"
                name="fallback-option"
                checked={!useLocalFallback}
                onChange={() => setUseLocalFallback(false)}
                style={{ marginTop: '2px' }}
              />
              <div>
                <div style={{ fontSize: '0.9rem', color: '#e8eaed', fontWeight: 500 }}>
                  仅使用已配置的服务
                </div>
                <div style={{ fontSize: '0.8rem', color: '#7a8594', marginTop: '0.2rem' }}>
                  缺失的模态将被跳过，相关内容不会生成
                </div>
              </div>
            </label>
          </div>
        </div>

        {/* Actions */}
        <div style={{ display: 'flex', gap: '0.75rem', justifyContent: 'flex-end' }}>
          <button
            className="btn btn-secondary"
            onClick={onCancel}
            style={{ padding: '0.6rem 1.2rem', fontSize: '0.9rem' }}
          >
            取消
          </button>
          <button
            className="btn btn-primary"
            onClick={() => onConfirm(useLocalFallback)}
            style={{ padding: '0.6rem 1.2rem', fontSize: '0.9rem' }}
          >
            确认创建
          </button>
        </div>
      </div>
    </div>
  );
}
