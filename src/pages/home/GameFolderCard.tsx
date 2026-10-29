import { useEffect, useMemo } from "react";
import { useQuery } from "@tanstack/react-query";
import { useTranslation } from "react-i18next";
import { CircleAlert, CircleCheck, LoaderCircle } from "lucide-react";
import { Moon } from "@/components/brand";
import { FolderActions } from "@/components/common";
import { Badge } from "@/components/ui";
import { useConfig, useGameCheck, useSetting } from "@/hooks";
import { api } from "@/lib";

/** The game: its own tile (from the dump), edition and version, The Old Hunters, whether it can
 * start, and its folder. */
export function GameFolderCard() {
  const { t } = useTranslation();
  const { data: config } = useConfig();
  const setSetting = useSetting();
  const dir = config?.settings.game_dir ?? "";
  const fixed = config?.platform === "ios";
  const check = useGameCheck(config?.settings.game_dir);
  const game = dir ? check.data?.game : undefined;
  const tile = useGameTile(dir);
  const status = check.isPending ? (
    <span className="flex items-center gap-2 text-muted-foreground"><LoaderCircle className="size-4 animate-spin" /> {t("home.checking")}</span>
  ) : check.data?.problem ? (
    <span className="flex items-start gap-2 text-blood-400"><CircleAlert className="mt-0.5 size-4 shrink-0" /> {dir ? check.data.problem : t("home.noFolder")}</span>
  ) : (
    <span className="flex items-center gap-2 text-emerald-400"><CircleCheck className="size-4" /> {t("home.ready")}</span>
  );
  return (
    <section className="mb-8">
      <div className="flex flex-col gap-4 rounded-2xl surface p-4 sm:flex-row sm:items-center sm:p-5">
        <div className="flex min-w-0 flex-1 items-center gap-4">
          <div className="flex size-20 shrink-0 items-center justify-center overflow-hidden rounded-xl surface-inset sm:size-24">
            {tile ? <img src={tile} alt="" className="size-full object-cover" draggable={false} /> : <Moon className="size-14" />}
          </div>
          <div className="min-w-0 space-y-1">
            <p className="truncate text-sm font-medium">
              {game ? `${game.title || "Bloodborne"} · ${t(`home.region.${game.region}`)}` : "Bloodborne"}
            </p>
            {game && <p className="flex flex-wrap items-center gap-x-3 gap-y-1 text-xs text-muted-foreground">
              {t("home.edition", { id: game.title_id, version: game.version.replace(/^0/, "") })}
              <span className="flex items-center gap-1.5">{t("home.oldHunters")}
                {game.old_hunters ? <Badge className="bg-emerald-500/15 text-emerald-400">{t("home.dlcIncluded")}</Badge>
                  : <Badge variant="outline" title={t("home.dlcMissingHint")}>{t("home.dlcMissing")}</Badge>}
              </span>
            </p>}
            {!game && <p className="text-xs text-muted-foreground">{fixed ? t("home.copyGame") : t("home.gameFolderHint")}</p>}
            <div className="pt-1 text-sm">{status}</div>
          </div>
        </div>
        <div className="flex shrink-0 items-center gap-2 sm:self-center">
          {/* iOS: the app's own folder, filled with the Files app; it cannot be chosen or shown. */}
          {!fixed && <FolderActions path={dir} title={t("home.folder")} onChoose={(path) => setSetting("game_dir", path)} />}
        </div>
      </div>
    </section>
  );
}

/** The game's tile from the dump as an object URL, or nothing (no folder, or no tile in it). */
function useGameTile(dir: string) {
  const { data } = useQuery({ queryKey: ["game-tile", dir], queryFn: () => api.gameIcon(dir), enabled: dir !== "", staleTime: Infinity, retry: false });
  const url = useMemo(() => data?.byteLength ? URL.createObjectURL(new Blob([data], { type: "image/png" })) : undefined, [data]);
  useEffect(() => () => { if (url) URL.revokeObjectURL(url); }, [url]);
  return dir ? url : undefined;
}
