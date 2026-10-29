import { useQuery } from "@tanstack/react-query";
import { useTranslation } from "react-i18next";
import { api } from "@/lib";
import { Message } from "@/components/common";
import { CharacterCard } from "@/pages/saves/CharacterCard";

/** The save set's characters, each as a card. */
export function CharacterPreview({ profileId, running }: { profileId: string; running: boolean }) {
  const { t } = useTranslation();
  const { data, error, isPending } = useQuery({
    queryKey: ["characters", profileId, running],
    queryFn: () => api.listCharacters(profileId),
    enabled: !running,
    staleTime: 0,
  });
  const note = (text: string) => <p className="rounded-2xl surface px-5 py-4 text-sm text-muted-foreground">{text}</p>;
  return <div className="mb-5 space-y-4">
    {running ? note(t("saves.stopFirst"))
      : error ? <Message tone="error" className="rounded-2xl surface px-5 py-4">{String(error)}</Message>
      : isPending ? note(t("saves.readingCharacters"))
      : !data?.length ? note(t("saves.noCharacters"))
      : data.map((character) => <CharacterCard key={`${character.title_id}:${character.slot}`} character={character} disabled={running} />)}
  </div>;
}
