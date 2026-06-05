import { useState, useEffect } from 'react';
import { useNavigate } from 'react-router-dom';
import { useGame } from '../hooks/useGame';
import type { GameInfo } from '../hooks/useGame';

const GAME_TYPE_LABELS: Record<string, string> = {
  visual_novel: '视觉小说',
  rpg: 'RPG',
  mystery: '悬疑解谜',
  horror: '恐怖生存',
  simulation: '模拟经营',
};

function formatDate(timestamp: number): string {
  return new Date(timestamp).toLocaleDateString('zh-CN', {
    year: 'numeric',
    month: '2-digit',
    day: '2-digit',
  });
}

export default function GameList() {
  const navigate = useNavigate();
  const { listGames } = useGame();

  const [games, setGames] = useState<GameInfo[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState('');

  useEffect(() => {
    listGames()
      .then((data) => {
        setGames(data || []);
      })
      .catch((err) => {
        console.error('Failed to load games:', err);
        setError('加载游戏列表失败');
      })
      .finally(() => {
        setLoading(false);
      });
  }, [listGames]);

  return (
    <div className="page game-list">
      <button className="btn btn-back" onClick={() => navigate('/')}>
        ← 返回
      </button>

      <h2 className="page-title">游戏列表</h2>

      {loading && (
        <div className="empty-state">
          <div className="spinner" />
          <p>加载中...</p>
        </div>
      )}

      {error && <p className="form-error">{error}</p>}

      {!loading && !error && games.length === 0 && (
        <div className="empty-state">
          <p style={{ fontSize: '2.5rem', margin: 0 }}>🎮</p>
          <p style={{ color: '#8888aa', fontSize: '1rem' }}>还没有创建任何游戏</p>
          <button
            className="btn btn-primary"
            onClick={() => navigate('/create')}
          >
            🚀 创建新游戏
          </button>
        </div>
      )}

      {!loading && !error && games.length > 0 && (
        <div className="game-card-list">
          {games.map((game) => (
            <div
              key={game.id}
              className="game-card"
              onClick={() => navigate(`/play/${game.id}`)}
            >
              <div className="game-card-header">
                <h3 className="game-card-title">{game.title}</h3>
                <span className="game-card-type">
                  {GAME_TYPE_LABELS[game.gameType] || game.gameType}
                </span>
              </div>
              <div className="game-card-meta">
                <span>📖 {game.totalChapters} 章</span>
                <span>📅 {formatDate(game.createdAt)}</span>
              </div>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
