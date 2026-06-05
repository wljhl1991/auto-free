import { useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { useGame } from '../hooks/useGame';
import { useConfig } from '../hooks/useConfig';
import ModalitySelectModal from '../components/Config/ModalitySelectModal';
import type { ModalityAvailability } from '../hooks/useConfig';

interface HistoryItem {
  outline: string;
  gameType: string;
  timestamp: number;
}

const HISTORY_KEY = 'autofree_create_history';
const MAX_HISTORY = 20;

function loadHistory(): HistoryItem[] {
  try {
    const data = localStorage.getItem(HISTORY_KEY);
    return data ? JSON.parse(data) : [];
  } catch { return []; }
}

function saveHistory(items: HistoryItem[]) {
  localStorage.setItem(HISTORY_KEY, JSON.stringify(items.slice(0, MAX_HISTORY)));
}

const GAME_TYPES = [
  { value: '', label: '自动推断' },
  { value: 'visual_novel', label: '视觉小说' },
  { value: 'rpg', label: 'RPG' },
  { value: 'mystery', label: '悬疑解谜' },
  { value: 'horror', label: '恐怖生存' },
  { value: 'simulation', label: '模拟经营' },
];

const EXAMPLE_OUTLINES = [
  '一个失忆的侦探在雨夜醒来，身边只有一张写有地址的纸条，他必须在天亮前找回自己的记忆，否则将永远失去真相。',
  '在一座与世隔绝的小岛上，六位陌生人被困在一栋古堡中。每天晚上都会有一人离奇消失，幸存者必须在彼此间找出凶手。',
  '你是一名星际商人，驾驶着破旧的货船在银河系边缘穿行。一次偶然的货物捡漏让你卷入了两个外星文明的战争之中。',
];

export default function CreateGame() {
  const navigate = useNavigate();
  const { createGame, createGameFromScript, getRandomOutline } = useGame();
  const { checkAvailableModalities } = useConfig();

  const [outline, setOutline] = useState('');
  const [gameType, setGameType] = useState('');
  const [loading, setLoading] = useState(false);
  const [randomLoading, setRandomLoading] = useState(false);
  const [error, setError] = useState('');
  const [showModalityModal, setShowModalityModal] = useState(false);
  const [modalityAvailability, setModalityAvailability] = useState<ModalityAvailability | null>(null);
  const [debugExpanded, setDebugExpanded] = useState(false);
  const [scriptJson, setScriptJson] = useState('');
  const [scriptLoading, setScriptLoading] = useState(false);
  const [scriptError, setScriptError] = useState('');
  const [history, setHistory] = useState<HistoryItem[]>(loadHistory);
  const [showHistory, setShowHistory] = useState(false);

  const handleSubmit = async () => {
    if (!outline.trim()) {
      setError('请输入游戏大纲');
      return;
    }
    setLoading(true);
    setError('');
    try {
      const availability = await checkAvailableModalities();
      const hasAnyMissing = !availability.text || !availability.image || !availability.video || !availability.music || !availability.voice;

      if (hasAnyMissing) {
        setModalityAvailability(availability);
        setShowModalityModal(true);
        setLoading(false);
        return;
      }

      // All modalities available, proceed directly
      await doCreateGame(true);
    } catch (e: any) {
      const msg = typeof e === 'string' ? e : (e?.message || '检测服务失败，请重试');
      setError(msg);
      setLoading(false);
    }
  };

  const doCreateGame = async (useLocalFallback: boolean) => {
    setLoading(true);
    setError('');
    // Save to history before creating
    const newHistory = [{ outline, gameType, timestamp: Date.now() }, ...history.filter(h => h.outline !== outline)];
    setHistory(newHistory);
    saveHistory(newHistory);
    try {
      const gameInfo = await createGame(outline, gameType || undefined, useLocalFallback);
      navigate(`/generate/${gameInfo.id}`);
    } catch (e: any) {
      const msg = typeof e === 'string' ? e : (e?.message || '创建失败，请重试');
      setError(msg);
    } finally {
      setLoading(false);
    }
  };

  const handleModalityConfirm = (useLocalFallback: boolean) => {
    setShowModalityModal(false);
    doCreateGame(useLocalFallback);
  };

  const handleModalityCancel = () => {
    setShowModalityModal(false);
  };

  const handleRandomOutline = async () => {
    setRandomLoading(true);
    setError('');
    try {
      const themes: string[] = [];
      const result = await getRandomOutline(gameType || undefined, themes.length > 0 ? themes : undefined);
      setOutline(result);
    } catch (e: any) {
      // 如果 AI 生成失败，使用本地示例
      const randomExample = EXAMPLE_OUTLINES[Math.floor(Math.random() * EXAMPLE_OUTLINES.length)];
      setOutline(randomExample);
    } finally {
      setRandomLoading(false);
    }
  };

  const handleCreateFromScript = async () => {
    if (!scriptJson.trim()) {
      setScriptError('请粘贴游戏脚本 JSON');
      return;
    }
    // 验证 JSON 格式
    try {
      JSON.parse(scriptJson);
    } catch {
      setScriptError('JSON 格式无效，请检查输入');
      return;
    }
    setScriptLoading(true);
    setScriptError('');
    try {
      const gameInfo = await createGameFromScript(scriptJson);
      navigate(`/generate/${gameInfo.id}`);
    } catch (e: any) {
      const msg = typeof e === 'string' ? e : (e?.message || '从脚本创建失败');
      setScriptError(msg);
    } finally {
      setScriptLoading(false);
    }
  };

  const deleteHistoryItem = (timestamp: number) => {
    const newHistory = history.filter(h => h.timestamp !== timestamp);
    setHistory(newHistory);
    saveHistory(newHistory);
  };

  const gameTypeLabel = (value: string) => {
    const found = GAME_TYPES.find(t => t.value === value);
    return found ? found.label : value || '自动推断';
  };

  return (
    <div className="page create-game">
      <button className="btn btn-back" onClick={() => navigate('/')}>
        ← 返回
      </button>

      <h2 className="page-title">创建新游戏</h2>

      <div className="form-group">
        <label className="form-label">游戏大纲</label>
        <textarea
          className="form-textarea"
          value={outline}
          onChange={(e) => { setOutline(e.target.value); setError(''); }}
          placeholder="输入一句话、几句话、或完整大纲，AI 将为你生成游戏..."
          rows={10}
        />
      </div>

      <div style={{ marginBottom: '1.25rem' }}>
        <button
          type="button"
          onClick={() => setShowHistory(!showHistory)}
          style={{
            background: 'none',
            border: 'none',
            color: '#8888aa',
            fontSize: '0.85rem',
            cursor: 'pointer',
            padding: '0.3rem 0',
            fontFamily: 'inherit',
            display: 'inline-flex',
            alignItems: 'center',
            gap: '0.4rem',
            transition: 'color 0.2s ease',
          }}
          onMouseEnter={e => (e.currentTarget.style.color = '#4a90d9')}
          onMouseLeave={e => (e.currentTarget.style.color = '#8888aa')}
        >
          <span style={{ fontSize: '0.7rem', transition: 'transform 0.2s', transform: showHistory ? 'rotate(90deg)' : 'rotate(0deg)', display: 'inline-block' }}>▶</span>
          输入历史 {history.length > 0 && `(${history.length})`}
        </button>

        {showHistory && (
          <div style={{
            marginTop: '0.5rem',
            maxHeight: '320px',
            overflowY: 'auto',
            backgroundColor: '#12121f',
            border: '1px solid #1e1e30',
            borderRadius: '8px',
          }}>
            {history.length === 0 ? (
              <p style={{ color: '#555570', fontSize: '0.85rem', padding: '1rem', textAlign: 'center', margin: 0 }}>暂无历史记录</p>
            ) : (
              history.map((item) => (
                <div
                  key={item.timestamp}
                  style={{
                    display: 'flex',
                    alignItems: 'flex-start',
                    gap: '0.6rem',
                    padding: '0.7rem 0.85rem',
                    borderBottom: '1px solid #1e1e30',
                    cursor: 'pointer',
                    transition: 'background-color 0.15s ease',
                  }}
                  onMouseEnter={e => (e.currentTarget.style.backgroundColor = '#1a1a2e')}
                  onMouseLeave={e => (e.currentTarget.style.backgroundColor = 'transparent')}
                  onClick={() => { setOutline(item.outline); setGameType(item.gameType); setError(''); }}
                >
                  <div style={{ flex: 1, minWidth: 0 }}>
                    <div style={{
                      fontSize: '0.85rem',
                      color: '#c0c0d0',
                      lineHeight: '1.5',
                      overflow: 'hidden',
                      textOverflow: 'ellipsis',
                      whiteSpace: 'nowrap',
                    }}>
                      {item.outline}
                    </div>
                    <div style={{ display: 'flex', gap: '0.75rem', marginTop: '0.3rem', fontSize: '0.75rem', color: '#666680' }}>
                      <span style={{ color: '#7b68ee', backgroundColor: 'rgba(123, 104, 238, 0.12)', border: '1px solid rgba(123, 104, 238, 0.25)', borderRadius: '8px', padding: '0 6px' }}>
                        {gameTypeLabel(item.gameType)}
                      </span>
                      <span>{new Date(item.timestamp).toLocaleString('zh-CN')}</span>
                    </div>
                  </div>
                  <button
                    type="button"
                    onClick={(e) => { e.stopPropagation(); deleteHistoryItem(item.timestamp); }}
                    style={{
                      background: 'none',
                      border: 'none',
                      color: '#555570',
                      fontSize: '1rem',
                      cursor: 'pointer',
                      padding: '0.15rem 0.3rem',
                      lineHeight: 1,
                      borderRadius: '4px',
                      transition: 'color 0.15s ease, background-color 0.15s ease',
                      flexShrink: 0,
                    }}
                    onMouseEnter={e => { e.currentTarget.style.color = '#f87171'; e.currentTarget.style.backgroundColor = 'rgba(248, 113, 113, 0.1)'; }}
                    onMouseLeave={e => { e.currentTarget.style.color = '#555570'; e.currentTarget.style.backgroundColor = 'transparent'; }}
                    title="删除"
                  >
                    ×
                  </button>
                </div>
              ))
            )}
          </div>
        )}
      </div>

      <div className="form-row">
        <div className="form-group form-group-inline">
          <label className="form-label">游戏类型</label>
          <select
            className="form-select"
            value={gameType}
            onChange={(e) => setGameType(e.target.value)}
          >
            {GAME_TYPES.map((t) => (
              <option key={t.value} value={t.value}>{t.label}</option>
            ))}
          </select>
        </div>

        <button className="btn btn-secondary" onClick={handleRandomOutline} disabled={randomLoading}>
          {randomLoading ? '⏳ 生成中...' : '🎲 随机大纲'}
        </button>
      </div>

      <div className="example-section">
        <p className="form-hint">示例大纲：</p>
        <div className="example-chips">
          {EXAMPLE_OUTLINES.map((ex, i) => (
            <button
              key={i}
              className="chip"
              onClick={() => setOutline(ex)}
              title={ex}
            >
              示例 {i + 1}
            </button>
          ))}
        </div>
      </div>

      {error && <p className="form-error">{error}</p>}

      {loading && (
        <div className="create-loading">
          <div className="create-loading-spinner"></div>
          <div className="create-loading-text">正在生成游戏世界...</div>
          <div className="create-loading-hint">AI 正在解析大纲并构建游戏，通常需要 10-30 秒</div>
        </div>
      )}

      <button
        className="btn btn-primary btn-submit"
        onClick={handleSubmit}
        disabled={loading || !outline.trim()}
      >
        {loading ? '创建中...' : '🚀 开始创建'}
      </button>

      {modalityAvailability && (
        <ModalitySelectModal
          availability={modalityAvailability}
          isOpen={showModalityModal}
          onConfirm={handleModalityConfirm}
          onCancel={handleModalityCancel}
        />
      )}

      <div className="debug-section">
        <button
          className="debug-toggle"
          onClick={() => setDebugExpanded(!debugExpanded)}
        >
          <span className="debug-toggle-icon">{debugExpanded ? '▼' : '▶'}</span>
          调试模式
        </button>

        {debugExpanded && (
          <div className="debug-content">
            <label className="form-label">游戏脚本 JSON</label>
            <textarea
              className="form-textarea debug-textarea"
              value={scriptJson}
              onChange={(e) => { setScriptJson(e.target.value); setScriptError(''); }}
              placeholder='粘贴 GameScript JSON，例如：{"meta":{"title":"...","gameType":"visual_novel",...},"chapters":[...]}'
              rows={12}
            />
            {scriptError && <p className="form-error">{scriptError}</p>}
            <button
              className="btn btn-debug"
              onClick={handleCreateFromScript}
              disabled={scriptLoading || !scriptJson.trim()}
            >
              {scriptLoading ? '创建中...' : '🔧 从脚本创建'}
            </button>
          </div>
        )}
      </div>
    </div>
  );
}
