import type { AIProviderConfig } from '@/types';
import ProviderCard from './ProviderCard';

interface ModalitySectionProps {
  modality: string;
  title: string;
  providers: AIProviderConfig[];
  onConfigure: (id: string) => void;
  onCheck: (id: string, testPrompt?: string, modelId?: string) => void;
  onCopy: (id: string) => void;
  onDelete: (id: string) => void;
  onReset: (id: string) => void;
}

const modalityIcons: Record<string, string> = {
  text: '📝',
  image: '🖼️',
  video: '🎬',
  music: '🎵',
  voice: '🎙️',
};

export default function ModalitySection({
  modality,
  title,
  providers,
  onConfigure,
  onCheck,
  onCopy,
  onDelete,
  onReset,
}: ModalitySectionProps) {
  if (providers.length === 0) return null;

  return (
    <div style={{ marginBottom: '2rem' }}>
      <h3 style={{
        fontSize: '1.1rem',
        fontWeight: 600,
        color: '#1a1a2e',
        marginBottom: '1rem',
        display: 'flex',
        alignItems: 'center',
        gap: '0.5rem',
      }}>
        <span>{modalityIcons[modality] || '🔧'}</span>
        {title}
        <span style={{
          fontSize: '0.75rem',
          fontWeight: 400,
          color: '#9ca3af',
          marginLeft: '0.25rem',
        }}>
          ({providers.length})
        </span>
      </h3>
      <div style={{
        display: 'grid',
        gridTemplateColumns: 'repeat(auto-fill, minmax(340px, 1fr))',
        gap: '1rem',
      }}>
        {providers.map((provider) => (
          <ProviderCard
            key={provider.id}
            provider={provider}
            onConfigure={() => onConfigure(provider.id)}
            onCheck={() => onCheck(provider.id)}
            onCopy={() => onCopy(provider.id)}
            onDelete={() => onDelete(provider.id)}
            onReset={() => onReset(provider.id)}
          />
        ))}
      </div>
    </div>
  );
}
