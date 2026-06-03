import { useParams } from 'react-router-dom';

export default function GamePlay() {
  const { gameId } = useParams<{ gameId: string }>();

  return (
    <div className="page game-play">
      <h2 className="page-title">游戏主界面</h2>
      <p className="game-id">Game ID: {gameId}</p>
      <p className="placeholder-text">（游戏主界面将在后续节点完善）</p>
    </div>
  );
}
