import { useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { useGame } from '../hooks/useGame';

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
  const { createGame, getRandomOutline } = useGame();

  const [outline, setOutline] = useState('');
  const [gameType, setGameType] = useState('');
  const [loading, setLoading] = useState(false);
  const [randomLoading, setRandomLoading] = useState(false);
  const [error, setError] = useState('');

  const handleSubmit = async () => {
    if (!outline.trim()) {
      setError('请输入游戏大纲');
      return;
    }
    setLoading(true);
    setError('');
    try {
      const gameInfo = await createGame(outline, gameType || undefined);
      navigate(`/generate/${gameInfo.id}`);
    } catch (e: any) {
      const msg = typeof e === 'string' ? e : (e?.message || '创建失败，请重试');
      setError(msg);
    } finally {
      setLoading(false);
    }
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
    </div>
  );
}
