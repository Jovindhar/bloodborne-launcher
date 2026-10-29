import { useState } from "react";
import { LoaderCircle, Play, Square } from "lucide-react";
import { useTranslation } from "react-i18next";
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui";
import { OnlinePasswordDialog } from "@/components/sidebar/OnlinePasswordDialog";
import { useConfig, useProfiles, useGameCheck } from "@/hooks";
import { cn } from "@/lib";
import { useGameStore } from "@/stores";

/** Play (or Stop while the game runs); disabled with the reason while the game folder has a
 *  problem. */
export function PlayButton({ className }: { className?: string }) {
  const { t } = useTranslation();
  const { data: config } = useConfig();
  const check = useGameCheck(config?.settings.game_dir);
  const { running, play, stop } = useGameStore();
  const profiles = useProfiles();
  const problem = check.data?.problem;
  const [askPassword, setAskPassword] = useState(false);
  // Seamless Hunt's password, when the player did not save it, is asked for each launch.
  const settings = config?.settings;
  const passwordNeeded = !!settings?.online && settings.online_pool !== "huntersdream" && !config?.online_password_saved;
  const start = () => (passwordNeeded ? setAskPassword(true) : void play());

  if (running) {
    return (
      <button
        type="button"
        onClick={stop}
        className={cn(
          "flex w-full items-center justify-center gap-2 rounded-xl bg-blood-700/20 py-3 font-display text-sm font-semibold tracking-[0.25em] text-blood-400 backdrop-blur-sm transition-colors duration-300 ease-out hover:bg-blood-700/45",
          className,
        )}
      >
        <Square className="size-4 fill-current" /> {t("play.stop").toUpperCase()}
      </button>
    );
  }
  const button = (
    <button
      type="button"
      onClick={start}
      disabled={check.isPending || !!problem || profiles.busy || !profiles.data || profiles.data.setup_required}
      className={cn(
        "flex w-full items-center justify-center gap-2 rounded-xl bg-blood-500/70 py-3 font-display text-sm font-semibold tracking-[0.25em] text-parchment backdrop-blur-sm transition-colors duration-300 ease-out hover:bg-blood-500/90 active:bg-blood-400 disabled:cursor-not-allowed disabled:bg-white/[0.06] disabled:text-muted-foreground",
        className,
      )}
    >
      {check.isPending ? <LoaderCircle className="size-4 animate-spin" /> : <Play className="size-4 fill-current" />} {t("play.play").toUpperCase()}
    </button>
  );
  const prompt = <OnlinePasswordDialog open={askPassword} account={settings?.online_npid ?? ""}
    onPlay={(password) => { setAskPassword(false); void play(password); }} onCancel={() => setAskPassword(false)} />;
  if (!problem) return <>{button}{prompt}</>;
  return (
    <Tooltip>
      <TooltipTrigger render={<span className="block w-full" />}>{button}</TooltipTrigger>
      <TooltipContent>{problem}</TooltipContent>
    </Tooltip>
  );
}
