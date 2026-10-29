import { useTranslation } from "react-i18next";
import { Button } from "@/components/ui";
import { Message } from "@/components/common";
import { PageHeader, SelectRow, SettingsSection } from "@/components/settings";
import { useRecompBuild, useRecompCancel, useRecompDelete, useRecompStatus, useSetting, useSettingsData } from "@/hooks";
import { useRecompStore } from "@/stores";
import { RecompProgressBar } from "@/pages/recomp/RecompProgressBar";

const megabytes = (bytes: number) => `${Math.round(bytes / 1e6)} MB`;

/** The recompiled library: what it is, the choice to use it, and building it (recomp.rs). */
export function RecompPage() {
  const { t, i18n } = useTranslation();
  const { settings } = useSettingsData();
  const set = useSetting();
  const { data } = useRecompStatus();
  const progress = useRecompStore((s) => s.progress);
  const build = useRecompBuild();
  const cancel = useRecompCancel();
  const remove = useRecompDelete();
  const status = data?.status;
  const building = status?.state === "building" || build.isPending;
  const library = status?.library;
  const ready = status?.state === "ready";
  const noGame = !settings.game_dir;
  return <>
    <PageHeader title={t("recomp.title")} subtitle={t("recomp.subtitle")} />
    <SettingsSection title={t("recomp.whatTitle")}>
      <div className="grid gap-2 px-4 pb-4 pt-1 text-sm text-muted-foreground sm:px-5">
        <p>{t("recomp.what1")}</p>
        <p>{t("recomp.what2")}</p>
        <p>{t("recomp.what3")}</p>
      </div>
    </SettingsSection>
    <SettingsSection title={t("recomp.modeTitle")}>
      <SelectRow title={t("recomp.mode")} description={ready ? undefined : t("recomp.modeNeedsLibrary")}
        value={settings.cpu === "recompiled" ? "recompiled" : "translated"} disabled={!ready && settings.cpu !== "recompiled"}
        options={[{ value: "translated", label: "recomp.translated", hint: "recomp.translatedHint" },
                  { value: "recompiled", label: "recomp.recompiled", hint: "recomp.recompiledHint" }]}
        onChange={(value) => set("cpu", value)} />
    </SettingsSection>
    <SettingsSection title={t("recomp.libraryTitle")} description={t("recomp.buildHint")}>
      <div className="grid gap-4 px-4 pb-4 pt-1 sm:px-5">
        {status && !status.available ? <Message tone="warning">{t("recomp.unavailable")}</Message>
          : noGame && !library ? <Message tone="warning">{t("recomp.needsGame")}</Message>
          : building ? (progress ? <RecompProgressBar progress={progress} /> : <p className="text-sm text-muted-foreground">{t("recomp.steps.check")}</p>)
          : <p className="text-sm">
            {status?.state === "stale" ? t(status.reason === "game" ? "recomp.staleGame" : "recomp.staleRecompiler")
              : library ? t("recomp.ready", { date: new Date(library.built).toLocaleDateString(i18n.language), functions: library.functions.toLocaleString(i18n.language), size: megabytes(library.size) })
              : t("recomp.none")}
          </p>}
        {progress?.step === "cancelled" && !building && <p className="text-xs text-muted-foreground">{t("recomp.cancelled")}</p>}
        {progress?.step === "error" && !building && progress.message && <Message tone="error">{progress.message}</Message>}
        <div className="flex flex-wrap items-center gap-2">
          {building
            ? <Button size="sm" variant="secondary" disabled={cancel.isPending} onClick={() => cancel.mutate()}>{t("recomp.cancel")}</Button>
            : <Button size="sm" disabled={!status?.available || noGame} onClick={() => build.mutate()}>
              {library ? t("recomp.rebuild") : t("recomp.build")}</Button>}
          {!building && (data?.disk_used ?? 0) > 0 && <Button size="sm" variant="ghost" disabled={remove.isPending}
            onClick={() => { if (settings.cpu === "recompiled") set("cpu", "translated"); remove.mutate(); }}>{t("recomp.delete")}</Button>}
          {(data?.disk_used ?? 0) > 0 && <span className="ml-auto text-xs text-muted-foreground">{t("recomp.disk", { size: megabytes(data!.disk_used) })}</span>}
        </div>
      </div>
    </SettingsSection>
  </>;
}
