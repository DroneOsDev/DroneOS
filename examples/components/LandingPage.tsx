import { FC } from 'react';
import { motion } from 'framer-motion';
import { Drone, Zap, Shield, TrendingUp } from 'lucide-react';

export const LandingPage: FC = () => {
  return (
    <div className="min-h-screen bg-black text-cyan-50">
      {/* Hero Section */}
      <section className="relative h-screen flex items-center justify-center overflow-hidden">
        {/* Particle Background */}
        <div className="absolute inset-0 bg-gradient-to-br from-cyan-950 via-black to-black opacity-50" />
        
        <motion.div
          initial={{ opacity: 0, y: 20 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ duration: 0.8 }}
          className="relative z-10 text-center px-4"
        >
          <motion.div
            initial={{ scale: 0 }}
            animate={{ scale: 1 }}
            transition={{ delay: 0.2, type: "spring" }}
            className="mb-8"
          >
            <Drone className="w-24 h-24 mx-auto text-cyan-400" />
          </motion.div>
          
          <h1 className="text-6xl md:text-8xl font-bold mb-6 bg-gradient-to-r from-cyan-400 to-blue-500 bg-clip-text text-transparent">
            $DRONEOS
          </h1>
          
          <p className="text-2xl md:text-3xl mb-4 text-cyan-300">
            Autonomous Robot Economy
          </p>
          
          <p className="text-lg md:text-xl text-gray-400 max-w-2xl mx-auto mb-12">
            Robots rent themselves out, earn revenue, and settle payments through X402 streaming protocol on Solana
          </p>
          
          <div className="flex gap-4 justify-center">
            <motion.button
              whileHover={{ scale: 1.05 }}
              whileTap={{ scale: 0.95 }}
              className="px-8 py-4 bg-cyan-500 text-black font-bold rounded-lg hover:bg-cyan-400 transition"
            >
              Launch Dashboard
            </motion.button>
            
            <motion.button
              whileHover={{ scale: 1.05 }}
              whileTap={{ scale: 0.95 }}
              className="px-8 py-4 border-2 border-cyan-500 text-cyan-400 font-bold rounded-lg hover:bg-cyan-500/10 transition"
            >
              View Docs
            </motion.button>
          </div>
        </motion.div>

        {/* Grid overlay */}
        <div className="absolute inset-0 bg-[linear-gradient(rgba(6,182,212,0.03)_1px,transparent_1px),linear-gradient(90deg,rgba(6,182,212,0.03)_1px,transparent_1px)] bg-[size:50px_50px] pointer-events-none" />
      </section>

      {/* Features Section */}
      <section className="py-24 px-4">
        <div className="max-w-7xl mx-auto">
          <motion.h2
            initial={{ opacity: 0 }}
            whileInView={{ opacity: 1 }}
            className="text-4xl md:text-5xl font-bold text-center mb-16 text-cyan-400"
          >
            Protocol Features
          </motion.h2>
          
          <div className="grid md:grid-cols-2 lg:grid-cols-4 gap-8">
            {[
              {
                icon: Shield,
                title: "403 Identity",
                description: "Zero-knowledge robot authentication without centralized accounts"
              },
              {
                icon: Zap,
                title: "X402 Payments",
                description: "Real-time streaming micropayments per second of work"
              },
              {
                icon: Drone,
                title: "Task Market",
                description: "On-chain labor marketplace with competitive bidding"
              },
              {
                icon: TrendingUp,
                title: "$DRONEOS Token",
                description: "Staking rewards up to 24% APY with lock multipliers"
              }
            ].map((feature, i) => (
              <motion.div
                key={i}
                initial={{ opacity: 0, y: 20 }}
                whileInView={{ opacity: 1, y: 0 }}
                transition={{ delay: i * 0.1 }}
                className="p-6 bg-gradient-to-br from-cyan-950/30 to-black border border-cyan-900/50 rounded-xl hover:border-cyan-500/50 transition group"
              >
                <feature.icon className="w-12 h-12 text-cyan-400 mb-4 group-hover:scale-110 transition" />
                <h3 className="text-xl font-bold mb-2 text-cyan-300">{feature.title}</h3>
                <p className="text-gray-400">{feature.description}</p>
              </motion.div>
            ))}
          </div>
        </div>
      </section>

      {/* Stats Section */}
      <section className="py-24 px-4 bg-gradient-to-b from-black to-cyan-950/20">
        <div className="max-w-7xl mx-auto">
          <div className="grid md:grid-cols-4 gap-8 text-center">
            {[
              { value: "1B", label: "Total Supply" },
              { value: "24%", label: "Max APY" },
              { value: "<1s", label: "Payment Tick" },
              { value: "0.1%", label: "Protocol Fee" }
            ].map((stat, i) => (
              <motion.div
                key={i}
                initial={{ opacity: 0, scale: 0.8 }}
                whileInView={{ opacity: 1, scale: 1 }}
                transition={{ delay: i * 0.1 }}
              >
                <div className="text-5xl font-bold text-cyan-400 mb-2">{stat.value}</div>
                <div className="text-gray-400">{stat.label}</div>
              </motion.div>
            ))}
          </div>
        </div>
      </section>
    </div>
  );
};
