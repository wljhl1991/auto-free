import { useState } from 'react';
import { convertFileSrc } from '../../adapters/tauri';

interface LocalAsset {
  id: string;
  type: string;
  localPath: string;
  source: string;
  cacheKey: string;
  createdAt: number;
}

interface CandidateSelectorProps {
  candidates: LocalAsset[];
  onSelect: (candidate: LocalAsset) => void;
  onRegenerateAll: () => void;
  onClose: () => void;
  isRegenerating: boolean;
}

function CandidateSelector({ candidates, onSelect, onRegenerateAll, onClose, isRegenerating }: CandidateSelectorProps) {
  const [selectedIndex, setSelectedIndex] = useState<number | null>(null);

  const handleSelect = (index: number) => {
    if (isRegenerating) return;
    setSelectedIndex(index);
    onSelect(candidates[index]);
  };

  const resolveAssetUrl = (localPath: string): string => {
    try {
      return convertFileSrc(localPath);
    } catch {
      return localPath;
    }
  };

  return (
    <div
      style={{
        position: 'fixed',
        inset: 0,
        backgroundColor: 'rgba(0, 0, 0, 0.4)',
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
        zIndex: 1100,
        backdropFilter: 'blur(4px)', WebkitBackdropFilter: 'blur(4px)',
      }}
      onClick={(e) => {
        if (e.target === e.currentTarget) onClose();
      }}
    >
      <div style={{
        backgroundColor: 'rgba(255, 255, 255, 0.85)',
        border: '1px solid rgba(99, 102, 241, 0.15)',
        borderRadius: '16px',
        padding: '1.5rem',
        width: '90%',
        maxWidth: '640px',
        backdropFilter: 'blur(20px)', WebkitBackdropFilter: 'blur(20px)',
        boxShadow: '0 8px 32px rgba(99, 102, 241, 0.12), 0 2px 8px rgba(0, 0, 0, 0.06)',
      }}>
        <h3 style={{
          fontSize: '1.1rem',
          fontWeight: 600,
          color: '#1a1a2e',
          marginBottom: '1rem',
        }}>
          选择候选资源
        </h3>

        <div style={{
          display: 'grid',
          gridTemplateColumns: `repeat(${Math.min(candidates.length, 3)}, 1fr)`,
          gap: '0.75rem',
          marginBottom: '1.25rem',
        }}>
          {candidates.map((candidate, index) => (
            <div
              key={candidate.id}
              onClick={() => handleSelect(index)}
              style={{
                position: 'relative',
                borderRadius: '8px',
                overflow: 'hidden',
                border: selectedIndex === index
                  ? '2px solid #6366f1'
                  : '2px solid rgba(99, 102, 241, 0.15)',
                cursor: isRegenerating ? 'not-allowed' : 'pointer',
                transition: 'border-color 0.2s ease',
                aspectRatio: '16/10',
                backgroundColor: 'rgba(255, 255, 255, 0.8)',
              }}
            >
              {candidate.type === 'Video' ? (
                <video
                  src={resolveAssetUrl(candidate.localPath)}
                  style={{
                    width: '100%',
                    height: '100%',
                    objectFit: 'cover',
                  }}
                  muted
                />
              ) : (
                <img
                  src={resolveAssetUrl(candidate.localPath)}
                  alt={`候选 ${index + 1}`}
                  style={{
                    width: '100%',
                    height: '100%',
                    objectFit: 'cover',
                  }}
                />
              )}
              <div style={{
                position: 'absolute',
                bottom: 0,
                left: 0,
                right: 0,
                padding: '0.4rem 0.6rem',
                backgroundColor: 'rgba(255, 255, 255, 0.85)',
                fontSize: '0.8rem',
                color: '#4a4a6a',
              }}>
                候选 {index + 1}
              </div>
              {selectedIndex === index && (
                <div style={{
                  position: 'absolute',
                  top: '0.4rem',
                  right: '0.4rem',
                  width: '24px',
                  height: '24px',
                  borderRadius: '50%',
                  backgroundColor: '#6366f1',
                  display: 'flex',
                  alignItems: 'center',
                  justifyContent: 'center',
                  color: '#fff',
                  fontSize: '0.8rem',
                  fontWeight: 700,
                }}>
                  ✓
                </div>
              )}
            </div>
          ))}
        </div>

        <div style={{
          display: 'flex',
          gap: '0.75rem',
          justifyContent: 'flex-end',
        }}>
          <button
            className="btn btn-secondary"
            onClick={onClose}
            style={{ padding: '0.6rem 1.2rem', fontSize: '0.9rem' }}
          >
            取消
          </button>
          <button
            className="btn btn-secondary"
            onClick={onRegenerateAll}
            disabled={isRegenerating}
            style={{ padding: '0.6rem 1.2rem', fontSize: '0.9rem' }}
          >
            {isRegenerating ? '生成中...' : '重新生成全部'}
          </button>
        </div>
      </div>
    </div>
  );
}

export default CandidateSelector;
