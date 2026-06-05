import { useState, useEffect } from 'react';
import { useNavigate } from 'react-router-dom';
import { useGeneration } from '../hooks/useGeneration';

export default function MainMenu() {
  const navigate = useNavigate();
  const generation = useGeneration();
  const [activeGenerations, setActiveGenerations] = useState<string[]>([]);

  useEffect(() => {
    generation.getActiveGenerations()
      .then((ids) => setActiveGenerations(ids))
      .catch(() => {});
  }, []);

  return (
    <div className="page main-menu">
      <div className="menu-center">
        <h1 className="app-title">AutoFree</h1>
        <p className="app-subtitle">AI 生成式游戏引擎</p>

        {activeGenerations.length > 0 && (
          <div className="active-generation-indicator" onClick={() => navigate(`/generate/${activeGenerations[0]}`)}>
            <span className="hint-icon">⟳</span>
            {activeGenerations.length} 个游戏正在生成中...
          </div>
        )}

        <div className="menu-buttons">
          <button className="btn btn-primary" onClick={() => navigate('/create')}>
            🎮 新游戏
          </button>
          <button className="btn btn-secondary" onClick={() => navigate('/games')}>
            📂 继续游戏
          </button>
          <button className="btn btn-secondary" onClick={() => navigate('/games')}>
            📋 游戏列表
          </button>
          <button className="btn btn-secondary" onClick={() => navigate('/settings')}>
            ⚙️ 设置
          </button>
        </div>
      </div>

      <footer className="version-footer">v0.1.0</footer>
    </div>
  );
}
