import { create } from "zustand";
import type { RecompProgress } from "@/lib/types";

type RecompState = {
  /** The last progress of the build running now (null: none since the launcher started). */
  progress: RecompProgress | null;
  /** When the running build started (for its elapsed time and estimate). */
  startedAt: number | null;
  /** When compiling started, and how many files were done then (a build resumed after a cancel). */
  compileFrom: { at: number; done: number } | null;
  update: (progress: RecompProgress) => void;
  start: () => void;
};

/** The recompiled library's build, kept here so its progress survives leaving the page. */
export const useRecompStore = create<RecompState>((set, get) => ({
  progress: null,
  startedAt: null,
  compileFrom: null,
  start: () => set({ progress: { step: "check" }, startedAt: Date.now(), compileFrom: null }),
  update: (progress) => {
    const state = get();
    const compileFrom = progress.step === "compile" && !state.compileFrom ? { at: Date.now(), done: progress.done ?? 0 } : state.compileFrom;
    set({ progress, compileFrom, startedAt: state.startedAt ?? Date.now() });
  },
}));
