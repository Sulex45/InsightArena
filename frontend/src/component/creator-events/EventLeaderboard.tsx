"use client";

import { Medal, Trophy } from "lucide-react";

export interface EventLeaderboardRow {
  rank: number;
  address: string;
  score: string;
  completionTime: string;
}

interface EventLeaderboardProps {
  winners: EventLeaderboardRow[];
}

export default function EventLeaderboard({ winners }: EventLeaderboardProps) {
  if (winners.length === 0) {
    return (
      <div className="rounded-3xl border border-dashed border-white/15 bg-slate-900/70 p-8 text-center text-slate-400">
        <Trophy className="mx-auto mb-3 h-8 w-8 text-slate-500" />
        Winners will appear here after all matches resolve and perfect predictions are verified.
      </div>
    );
  }

  return (
    <div className="overflow-hidden rounded-3xl border border-white/10 bg-slate-900/80 shadow-xl shadow-black/20">
      <div className="grid grid-cols-[0.5fr_1.5fr_0.8fr_1fr] gap-4 border-b border-white/10 px-5 py-4 text-xs font-semibold uppercase tracking-[0.18em] text-slate-500">
        <span>Rank</span>
        <span>User address</span>
        <span>Score</span>
        <span className="text-right">Completion time</span>
      </div>

      <div className="divide-y divide-white/10">
        {winners.map((winner) => (
          <div
            key={`${winner.rank}-${winner.address}`}
            className="grid grid-cols-[0.5fr_1.5fr_0.8fr_1fr] gap-4 px-5 py-4 text-sm text-slate-300"
          >
            <span className="flex items-center gap-2 font-semibold text-amber-200">
              <Medal className="h-4 w-4" />
              #{winner.rank}
            </span>
            <span className="truncate font-semibold text-white">{winner.address}</span>
            <span className="text-emerald-200">{winner.score}</span>
            <span className="text-right">{winner.completionTime}</span>
          </div>
        ))}
      </div>
    </div>
  );
}
