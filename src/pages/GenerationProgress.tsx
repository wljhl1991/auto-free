import { useParams, useNavigate } from 'react-router-dom';

export default function GenerationProgress() {
  const { gameId } = useParams<{ gameId: string }>();
  const navigate = useNavigate();

  return (
    <div className="page generation-progress">
      <h2 className="page-title">生成进度</h2>
      <p className="game-id">Game ID: {gameId}</p>
      <p className="placeholder-text">（生成进度界面将在后续节点完善）</p>
      <button className="btn btn-primary" onClick={() => navigate(`/play/${gameId}`)}>
        🎮 开始游玩
      </button>
    </div>
  );
}
