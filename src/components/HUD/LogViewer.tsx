import { useState, useEffect, useRef, useCallback } from 'react';
import { useConfig } from '@/hooks/useConfig';

interface LogViewerProps {
  isOpen: boolean;
  onClose: () => void;
}

export default function LogViewer({ isOpen, onClose }: LogViewerProps) {
  const [logs, setLogs] = useState('');
  const [loading, setLoading] = useState(false);
  const logEndRef = useRef<HTMLDivElement>(null);
  const config = useConfig();

  const fetchLogs = useCallback(async () => {
    setLoading(true);
    try {
      const content = await config.readLogs(200);
      setLogs(content);
    } catch (err) {
      setLogs(`读取日志失败: ${typeof err === 'string' ? err : (err as any)?.message || '未知错误'}`);
    } finally {
      setLoading(false);
    }
  }, [config]);

  useEffect(() => {
    if (isOpen) {
      fetchLogs();
    }
  }, [isOpen, fetchLogs]);

  useEffect(() => {
    logEndRef.current?.scrollIntoView({ behavior: 'smooth' });
  }, [logs]);

  if (!isOpen) return null;

  return (
    <div style={{
      position: 'fixed', inset: 0,
      backgroundColor: 'rgba(0,0,0,0.4)',
      display: 'flex', alignItems: 'center', justifyContent: 'center',
      zIndex: 1000,
      backdropFilter: 'blur(4px)', WebkitBackdropFilter: 'blur(4px)',
    }} onClick={e => { if (e.target === e.currentTarget) onClose(); }}>
      <div style={{
        backgroundColor: 'rgba(255, 255, 255, 0.85)', border: '1px solid rgba(99, 102, 241, 0.15)',
        borderRadius: '16px', padding: '1.5rem',
        width: '90%', maxWidth: '800px', maxHeight: '80vh',
        display: 'flex', flexDirection: 'column',
        backdropFilter: 'blur(20px)', WebkitBackdropFilter: 'blur(20px)',
        boxShadow: '0 8px 32px rgba(99, 102, 241, 0.12), 0 2px 8px rgba(0, 0, 0, 0.06)',
      }}>
        {/* Header */}
        <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '1rem' }}>
          <h3 style={{ fontSize: '1.1rem', fontWeight: 600, color: '#1a1a2e' }}>
            日志查看器
          </h3>
          <div style={{ display: 'flex', gap: '0.5rem' }}>
            <button
              className="btn btn-secondary"
              onClick={fetchLogs}
              disabled={loading}
              style={{ padding: '0.4rem 0.8rem', fontSize: '0.8rem' }}
            >
              {loading ? '加载中...' : '刷新'}
            </button>
            <button
              className="btn btn-secondary"
              onClick={onClose}
              style={{ padding: '0.4rem 0.8rem', fontSize: '0.8rem' }}
            >
              关闭
            </button>
          </div>
        </div>

        {/* Log Content */}
        <div style={{
          flex: 1, overflow: 'auto',
          backgroundColor: 'rgba(255, 255, 255, 0.5)', borderRadius: '10px',
          padding: '1rem', minHeight: '300px', maxHeight: '60vh',
        }}>
          <pre style={{
            margin: 0, fontSize: '0.8rem', fontFamily: 'monospace',
            color: '#4a4a6a', whiteSpace: 'pre-wrap', wordBreak: 'break-all',
            lineHeight: 1.5,
          }}>
            {logs}
          </pre>
          <div ref={logEndRef} />
        </div>
      </div>
    </div>
  );
}
