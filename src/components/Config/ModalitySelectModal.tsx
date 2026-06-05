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
        backgroundColor: 'rgba(0,0,0,0.7)',
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
        zIndex: 1000,
      }}
      onClick={(e) => { if (e.target === e.currentTarget) onCancel(); }}
    >
      <div
        style={{
          backgroundColor: '#16162a',
          border: '1px solid #2a2a3a',
          borderRadius: '12px',
          padding: '2rem',
          width: '90%',
          maxWidth: '480px',
        }}
        onClick={(e) => e.stopPropagation()}
      >
        {/* Header */}
        <h3 style={{ fontSize: '1.2rem', fontWeight: 600, color: '#e0e0f0', marginBottom: '0.5rem' }}>
          服务可用性检测
        </h3>
        <p style={{ fontSize: '0.85rem', color: '#8888aa', marginBottom: '1.25rem' }}>
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
                  backgroundColor: available ? 'rgba(46,125,50,0.08)' : 'rgba(224,96,96,0.08)',
                  border: `1px solid ${available ? 'rgba(46,125,50,0.2)' : 'rgba(224,96,96,0.2)'}`,
                  borderRadius: '8px',
                }}
              >
                <span style={{ display: 'flex', alignItems: 'center', gap: '0.5rem', fontSize: '0.9rem', color: '#e0e0f0' }}>
                  <span>{item.icon}</span>
                  {item.label}
                </span>
                <span style={{
                  fontSize: '0.85rem',
                  fontWeight: 500,
                  color: available ? '#4caf50' : '#e06060',
                }}>
                  {available ? '✓ 已配置' : '✗ 未配置'}
                </span>
              </div>
            );
          })}
        </div>

        {/* Fallback Options */}
        <div style={{ marginBottom: '1.5rem' }}>
          <p style={{ fontSize: '0.85rem', color: '#9999bb', marginBottom: '0.75rem' }}>
            缺失服务处理方式：
          </p>
          <div style={{ display: 'flex', flexDirection: 'column', gap: '0.5rem' }}>
            <label
              style={{
                display: 'flex',
                alignItems: 'flex-start',
                gap: '0.6rem',
                padding: '0.75rem',
                backgroundColor: useLocalFallback ? 'rgba(74,144,217,0.1)' : '#0a0a1a',
                border: `1px solid ${useLocalFallback ? 'rgba(74,144,217,0.3)' : '#2a2a3a'}`,
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
                <div style={{ fontSize: '0.9rem', color: '#e0e0f0', fontWeight: 500 }}>
                  使用本地资源替代缺失服务
                </div>
                <div style={{ fontSize: '0.8rem', color: '#8888aa', marginTop: '0.2rem' }}>
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
                backgroundColor: !useLocalFallback ? 'rgba(74,144,217,0.1)' : '#0a0a1a',
                border: `1px solid ${!useLocalFallback ? 'rgba(74,144,217,0.3)' : '#2a2a3a'}`,
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
                <div style={{ fontSize: '0.9rem', color: '#e0e0f0', fontWeight: 500 }}>
                  仅使用已配置的服务
                </div>
                <div style={{ fontSize: '0.8rem', color: '#8888aa', marginTop: '0.2rem' }}>
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
