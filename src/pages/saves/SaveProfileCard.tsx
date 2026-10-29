import { useTranslation } from "react-i18next";
import { Archive, FileText, FolderOpen } from "lucide-react";
import { Badge, Button } from "@/components/ui";
import { SettingsSection } from "@/components/settings";
import { RestoreBackupDialog } from "@/pages/saves/RestoreBackupDialog";
import { api } from "@/lib";
import type { SaveChange, SaveProfile } from "@/lib";

export function SaveProfileCard({ profile, disabled, onChange }: {
  profile: SaveProfile; disabled: boolean; onChange: (change: SaveChange) => Promise<void>;
}) {
  const { t, i18n } = useTranslation();
  const date = (seconds: number) => new Date(seconds * 1000).toLocaleString(i18n.language);
  const size = (bytes: number) => t("saves.size", { size: (bytes / 1_000_000).toLocaleString(i18n.language, { maximumFractionDigits: 1 }) });
  return <>
    <SettingsSection>
      <div className="flex flex-wrap items-center gap-4 px-5 py-5">
        <div className="flex size-10 shrink-0 items-center justify-center rounded-lg bg-secondary text-parchment/70">
          <Archive className="size-5" aria-hidden="true" />
        </div>
        <div className="min-w-0 flex-1">
          <h2 className="font-medium">{t("saves.current")}</h2>
          <p className="mt-1 text-xs text-muted-foreground">
            {profile.bytes ? size(profile.bytes) : t("saves.empty")}
            {profile.modified !== null && <> · {date(profile.modified)}</>}
          </p>
        </div>
        <Button size="sm" variant="secondary" disabled={disabled || !profile.bytes}
          onClick={() => onChange({ action: "backup", id: profile.id }).catch(() => {})}>{t("saves.backup")}</Button>
        <Button variant="ghost" size="icon-sm" aria-label={t("common.showFolder")} title={t("common.showFolder")}
          onClick={() => api.openPath(profile.path)}><FolderOpen aria-hidden="true" /></Button>
      </div>
    </SettingsSection>
    <SettingsSection title={t("saves.backups")} description={t("saves.backupHint")}>
      {profile.backups.length ? profile.backups.map((backup) => (
        <div key={backup.id} className="flex flex-wrap items-center gap-4 px-5 py-3">
          <FileText className="size-5 shrink-0 text-muted-foreground" aria-hidden="true" />
          <div className="min-w-0 flex-1">
            <p className="text-sm">{date(backup.created)}</p>
            <p className="mt-0.5 text-xs text-muted-foreground">{size(backup.bytes)}</p>
          </div>
          <Badge variant={backup.edited ? "destructive" : "secondary"}>{t(backup.edited ? "saves.backupEdited" : "saves.backupOriginal")}</Badge>
          <RestoreBackupDialog label={date(backup.created)} disabled={disabled}
            onRestore={() => onChange({ action: "restore", id: profile.id, backup: backup.id })} />
        </div>
      )) : <p className="px-5 py-4 text-sm text-muted-foreground">{t("saves.noBackups")}</p>}
    </SettingsSection>
  </>;
}
