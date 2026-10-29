import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import type { RecompProgress } from "@/lib/types";
import { useRecompStore } from "@/stores";

const minutes = (ms: number) => Math.max(1, Math.round(ms / 60000));

/** A running build: its step, a bar (download bytes or files compiled), elapsed time and, while
 * compiling, an estimate from the rate so far. */
export function RecompProgressBar({ progress }: { progress: RecompProgress }) {
  const { t } = useTranslation();
  const { startedAt, compileFrom } = useRecompStore();
  const [now, setNow] = useState(() => Date.now());
  useEffect(() => {
    const timer = setInterval(() => setNow(Date.now()), 1000);
    return () => clearInterval(timer);
  }, []);
  const { step, done = 0, total = 0 } = progress;
  const fraction = total > 0 ? Math.min(1, done / total) : null;
  const detail = step === "download" && total > 0
    ? t("recomp.downloadDetail", { done: Math.round(done / 1e6), total: Math.round(total / 1e6) })
    : step === "compile" && total > 0 ? t("recomp.compileDetail", { done, total }) : null;
  // Remaining: the files left at this rate (resumed builds count from where they started).
  const rate = step === "compile" && compileFrom && done > compileFrom.done ? (now - compileFrom.at) / (done - compileFrom.done) : null;
  const left = rate && total > done ? rate * (total - done) : null;
  return <div className="grid gap-2">
    <div className="flex flex-wrap items-baseline justify-between gap-2 text-sm">
      <span>{t(`recomp.steps.${step === "done" || step === "error" || step === "cancelled" ? "link" : step}`)}</span>
      {detail && <span className="tabular-nums text-muted-foreground">{detail}</span>}
    </div>
    <div className="h-2 overflow-hidden rounded-full bg-night-600" role="progressbar" aria-valuemin={0} aria-valuemax={100}
      aria-valuenow={fraction === null ? undefined : Math.round(fraction * 100)}>
      {fraction === null ? <div className="h-full w-1/3 animate-pulse rounded-full bg-gold-600" />
        : <div className="h-full rounded-full bg-gold-400 transition-[width] duration-500" style={{ width: `${fraction * 100}%` }} />}
    </div>
    <div className="flex flex-wrap justify-between gap-2 text-xs tabular-nums text-muted-foreground">
      {startedAt && <span>{t("recomp.elapsed", { minutes: Math.floor((now - startedAt) / 60000) })}</span>}
      {left !== null && <span>{t("recomp.remaining", { minutes: minutes(left) })}</span>}
    </div>
  </div>;
}
