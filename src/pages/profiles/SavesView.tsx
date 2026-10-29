import { useTranslation } from "react-i18next";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { RefreshCw } from "lucide-react";
import { Message } from "@/components/common";
import { PageHeader } from "@/components/settings";
import { Button } from "@/components/ui";
import { useSaves } from "@/hooks";
import { api } from "@/lib";
import { useUiStore } from "@/stores";
import { SaveEditor } from "@/pages/saves/editor";
import { CharacterPreview } from "@/pages/saves/CharacterPreview";
import { SaveProfileCard } from "@/pages/saves/SaveProfileCard";

/** A profile's save set, backups and save editor (the Saves tab shows the active one's). */
export function SavesView({ profileId }: { profileId: string }) {
  const { t } = useTranslation();
  const client = useQueryClient();
  const { data, error, refetch, isFetching, change, busy, running } = useSaves(profileId);
  const selected = data?.profiles[0];
  // A character opened in the save editor replaces the page (same query as the character list).
  const editing = useUiStore((state) => state.editing);
  const setEditing = useUiStore((state) => state.setEditing);
  const characters = useQuery({ queryKey: ["characters", profileId, running], queryFn: () => api.listCharacters(profileId), enabled: !running, staleTime: 0 });
  const character = characters.data?.find((entry) => `${entry.title_id}:${entry.slot}` === editing);
  if (editing && character) {
    return <SaveEditor key={character.fingerprint} profileId={profileId} character={character} disabled={running} onClose={() => setEditing(null)} />;
  }
  return (
    <>
      <PageHeader title={t("nav.saves")} subtitle={selected && [selected.name,
        ...(characters.data ? [t("saves.characterCount", { count: characters.data.length })] : []),
        t("saves.backupCount", { count: selected.backups.length })].join(" · ")} actions={
        <Button variant="secondary" size="sm" disabled={busy} onClick={() => {
          void refetch();
          void client.invalidateQueries({ queryKey: ["characters", profileId] });
        }}>
          <RefreshCw className={isFetching ? "animate-spin" : undefined} /> {t("common.refresh")}
        </Button>
      } />
      {running && <Message tone="warning" className="mb-4">{t("saves.stopFirst")}</Message>}
      {error && <Message tone="error" className="mb-4">{String(error)}</Message>}
      {selected && <>
        <CharacterPreview profileId={profileId} running={running} />
        <SaveProfileCard profile={selected} disabled={running || busy || isFetching} onChange={change} />
      </>}
    </>
  );
}
