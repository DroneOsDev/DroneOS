import { FC, useState, useEffect } from 'react';
import { useWallet } from '@solana/wallet-adapter-react';
import { Connection } from '@solana/web3.js';
import { DroneOS } from '@droneos/sdk';
import { Activity, Package, DollarSign, Users } from 'lucide-react';

interface DashboardStats {
  totalRobots: number;
  activeTasks: number;
  totalEarned: number;
  reputation: number;
}

export const Dashboard: FC = () => {
  const { publicKey, connected } = useWallet();
  const [stats, setStats] = useState<DashboardStats>({
    totalRobots: 0,
    activeTasks: 0,
    totalEarned: 0,
    reputation: 0
  });
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    if (connected && publicKey) {
      loadDashboardData();
    }
  }, [connected, publicKey]);

  const loadDashboardData = async () => {
    try {
      const connection = new Connection('https://api.devnet.solana.com');
      const droneos = new DroneOS(connection);
      
      // Load operator data
      // TODO: Implement actual data fetching
      setStats({
        totalRobots: 3,
        activeTasks: 7,
        totalEarned: 1250.50,
        reputation: 98.5
      });
    } catch (error) {
      console.error('Failed to load dashboard:', error);
    } finally {
      setLoading(false);
    }
  };

  if (!connected) {
    return (
      <div className="min-h-screen bg-black flex items-center justify-center">
        <div className="text-center">
          <h2 className="text-3xl font-bold text-cyan-400 mb-4">Connect Wallet</h2>
          <p className="text-gray-400">Please connect your wallet to access the dashboard</p>
        </div>
      </div>
    );
  }

  return (
    <div className="min-h-screen bg-black text-cyan-50 p-8">
      <div className="max-w-7xl mx-auto">
        {/* Header */}
        <div className="mb-8">
          <h1 className="text-4xl font-bold text-cyan-400 mb-2">Operator Dashboard</h1>
          <p className="text-gray-400">{publicKey?.toBase58().slice(0, 8)}...{publicKey?.toBase58().slice(-8)}</p>
        </div>

        {/* Stats Grid */}
        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-6 mb-8">
          {[
            { icon: Activity, label: 'Active Robots', value: stats.totalRobots, color: 'cyan' },
            { icon: Package, label: 'Active Tasks', value: stats.activeTasks, color: 'blue' },
            { icon: DollarSign, label: 'Total Earned', value: `${stats.totalEarned} DRONEOS`, color: 'green' },
            { icon: Users, label: 'Reputation', value: `${stats.reputation}%`, color: 'purple' }
          ].map((stat, i) => (
            <div
              key={i}
              className="bg-gradient-to-br from-cyan-950/30 to-black border border-cyan-900/50 rounded-xl p-6 hover:border-cyan-500/50 transition"
            >
              <div className="flex items-start justify-between mb-4">
                <stat.icon className={`w-8 h-8 text-${stat.color}-400`} />
                <span className="text-sm text-gray-500">Live</span>
              </div>
              <div className="text-3xl font-bold text-cyan-300 mb-1">{stat.value}</div>
              <div className="text-sm text-gray-400">{stat.label}</div>
            </div>
          ))}
        </div>

        {/* Quick Actions */}
        <div className="grid md:grid-cols-3 gap-6 mb-8">
          <button className="p-6 bg-cyan-500 hover:bg-cyan-400 text-black font-bold rounded-xl transition flex items-center justify-center gap-2">
            <Activity className="w-5 h-5" />
            Register Robot
          </button>
          <button className="p-6 bg-blue-500 hover:bg-blue-400 text-white font-bold rounded-xl transition flex items-center justify-center gap-2">
            <Package className="w-5 h-5" />
            Browse Tasks
          </button>
          <button className="p-6 border-2 border-cyan-500 text-cyan-400 hover:bg-cyan-500/10 font-bold rounded-xl transition flex items-center justify-center gap-2">
            <DollarSign className="w-5 h-5" />
            Claim Rewards
          </button>
        </div>

        {/* Recent Activity */}
        <div className="bg-gradient-to-br from-cyan-950/20 to-black border border-cyan-900/50 rounded-xl p-6">
          <h2 className="text-2xl font-bold text-cyan-400 mb-4">Recent Activity</h2>
          <div className="space-y-3">
            {[
              { type: 'Task Completed', robot: 'Drone-A1', reward: '45 DRONEOS', time: '2 min ago' },
              { type: 'Bid Accepted', robot: 'Drone-B2', task: 'Delivery #1234', time: '15 min ago' },
              { type: 'Payment Received', robot: 'Drone-A1', amount: '23.5 DRONEOS', time: '1 hour ago' }
            ].map((activity, i) => (
              <div key={i} className="flex items-center justify-between p-4 bg-black/50 rounded-lg hover:bg-cyan-950/20 transition">
                <div>
                  <div className="font-medium text-cyan-300">{activity.type}</div>
                  <div className="text-sm text-gray-400">{activity.robot} • {activity.time}</div>
                </div>
                <div className="text-green-400 font-bold">
                  {activity.reward || activity.amount || '✓'}
                </div>
              </div>
            ))}
          </div>
        </div>
      </div>
    </div>
  );
};
