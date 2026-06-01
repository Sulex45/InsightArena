"use client";

import { CalendarDays, WalletCards } from "lucide-react";

export interface EventParticipantRow {
  address: string;
  joinedAt: string;
  correctPredictions: number;
  totalMatches: number;
}

interface ParticipantListProps {
  participants: EventParticipantRow[];
}

export default function ParticipantList({ participants }: ParticipantListProps) {
  if (participants.length === 0) {
    return (
      <div className="rounded-3xl border border-dashed border-white/15 bg-slate-900/70 p-8 text-center text-slate-400">
        No participants have joined this event yet.
      </div>
    );
  }

  return (
    <div className="overflow-hidden rounded-3xl border border-white/10 bg-slate-900/80 shadow-xl shadow-black/20">
      <div className="grid grid-cols-[1.4fr_1fr_0.8fr] gap-4 border-b border-white/10 px-5 py-4 text-xs font-semibold uppercase tracking-[0.18em] text-slate-500">
        <span>Wallet address</span>
        <span>Join date</span>
        <span className="text-right">Score</span>
      </div>

      <div className="divide-y divide-white/10">
        {participants.map((participant) => (
          <div
            key={participant.address}
            className="grid grid-cols-[1.4fr_1fr_0.8fr] gap-4 px-5 py-4 text-sm text-slate-300"
          >
            <span className="flex min-w-0 items-center gap-2 font-semibold text-white">
              <WalletCards className="h-4 w-4 shrink-0 text-amber-300" />
              <span className="truncate">{participant.address}</span>
            </span>
            <span className="flex items-center gap-2">
              <CalendarDays className="h-4 w-4 text-slate-500" />
              {new Date(participant.joinedAt).toLocaleDateString()}
            </span>
            <span className="text-right font-semibold text-emerald-200">
              {participant.correctPredictions} / {participant.totalMatches}
            </span>
          </div>
        ))}
      </div>
    </div>
  );
}
