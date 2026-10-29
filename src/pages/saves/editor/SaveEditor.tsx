import { useState } from "react";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { useTranslation } from "react-i18next";
import { api, queryKeys } from "@/lib";
import type { Character } from "@/lib";
import { ArrowLeft } from "lucide-react";
import { PageHeader } from "@/components/settings";
import { Button, Tabs, TabsList, TabsTrigger } from "@/components/ui";
import { CharacterSection } from "@/pages/saves/editor/CharacterSection";
import { GemsSection } from "@/pages/saves/editor/GemsSection";
import { ItemsSection } from "@/pages/saves/editor/ItemsSection";
import { WeaponsSection } from "@/pages/saves/editor/WeaponsSection";
import { useSaveEdits } from "@/pages/saves/editor/useSaveEdits";
import { Message } from "@/components/common";

const SECTIONS = ["character", "item", "weapon", "armor", "gems", "storage"] as const;
type Section = (typeof SECTIONS)[number];
const TITLES = { character: "inventory.characterTab", item: "inventory.item", weapon: "inventory.weapon", armor: "inventory.armor",
  gems: "inventory.gems", storage: "inventory.storage" } as const;

type SaveEditorProps = { profileId: string; character: Character; disabled: boolean; onClose: () => void };

/** The save editor of one character, as a page of the Saves tab: its attributes and everything it
 * carries, saved together (one backup, one confirmation). */
export function SaveEditor({ profileId, character, disabled, onClose }: SaveEditorProps) {
  const { t, i18n } = useTranslation();
  const client = useQueryClient();
  const [section, setSection] = useState<Section>("character");
  const [pending, setPending] = useState(false);
  const [error, setError] = useState<string>();
  const inventory = useQuery({
    queryKey: ["inventory", profileId, character.title_id, character.slot, character.fingerprint, i18n.language],
    queryFn: () => api.characterInventory(profileId, character.title_id, character.slot, i18n.language),
    staleTime: 0,
  });
  const edits = useSaveEdits(character, inventory.data);
  const entries = inventory.data?.entries ?? [];
  const gems = inventory.data?.upgrades ?? [];
  const named = entries.some((entry) => entry.name);
  const locked = disabled || pending;
  const save = async () => {
    setPending(true); setError(undefined);
    try {
      await api.editSave(profileId, {
        character: edits.characterEdit,
        inventory: edits.changes.length && inventory.data ? { title_id: character.title_id, slot: character.slot, fingerprint: inventory.data.fingerprint, changes: edits.changes } : null,
      });
      await Promise.all([
        client.invalidateQueries({ queryKey: ["inventory", profileId] }),
        client.invalidateQueries({ queryKey: ["characters", profileId] }),
        client.invalidateQueries({ queryKey: queryKeys.saves }),
        client.invalidateQueries({ queryKey: queryKeys.profiles }),
      ]);
      onClose();
    } catch (failure) { setError(String(failure)); }
    finally { setPending(false); }
  };
  const body = () => {
    if (section === "character") return <CharacterSection character={character} edits={edits} disabled={locked} />;
    if (inventory.isPending) return <p className="text-sm text-muted-foreground">{t("inventory.reading")}</p>;
    if (inventory.error) return <Message tone="error">{String(inventory.error)}</Message>;
    const notice = !named && <p className="mb-3 text-xs text-muted-foreground">{t("inventory.noNames")}</p>;
    switch (section) {
      case "item": return <>{notice}<ItemsSection entries={entries.filter((e) => e.location === "inventory" && e.kind === "item")} edits={edits} addTo={named ? "inventory" : undefined} disabled={locked} /></>;
      case "weapon": return <>{notice}<WeaponsSection weapons={entries.filter((e) => e.kind === "weapon")} gems={gems} edits={edits} disabled={locked} /></>;
      case "armor": return <>{notice}<ItemsSection entries={entries.filter((e) => e.location === "inventory" && e.kind === "armor")} edits={edits}
        addTo={named ? "inventory" : undefined} categories={["armor"]} addLabel={t("inventory.addArmor")} disabled={locked} /></>;
      case "gems": return <GemsSection gems={gems} weapons={entries.filter((e) => e.kind === "weapon")} edits={edits} disabled={locked} />;
      case "storage": return <>{notice}<ItemsSection entries={entries.filter((e) => e.location === "storage" && e.kind !== "upgrade")} edits={edits} addTo={named ? "storage" : undefined}
        categories={["consumable", "material", "chalice", "weapon", "armor"]} disabled={locked} /></>;
    }
  };
  // A page of the Saves tab: back to the saves, the character, its sections as tabs, the section
  // (its lists scroll on their own, so the page stays put), and the changes with Save under it.
  return <>
    <Button variant="ghost" size="sm" className="mb-4 -ml-2" disabled={pending} onClick={onClose}>
      <ArrowLeft aria-hidden="true" /> {t("nav.saves")}
    </Button>
    <PageHeader title={character.name}
      subtitle={`${t("saves.level", { level: edits.characterEdit ? edits.level : character.level })} · ${t("saves.slot", { slot: character.slot + 1 })}`} />
    <Tabs value={section} onValueChange={(value) => setSection(value as Section)}>
      <TabsList className="mb-5 h-auto max-w-full justify-start gap-1 overflow-x-auto p-1 [scrollbar-width:none]">
        {SECTIONS.map((name) => <TabsTrigger key={name} value={name} className="h-8 shrink-0 gap-1.5 px-3.5">
          {t(TITLES[name])}
          {edits.counts[name] > 0 && <span className="rounded-full bg-primary px-1.5 text-xs text-primary-foreground tabular-nums">{edits.counts[name]}</span>}
        </TabsTrigger>)}
      </TabsList>
    </Tabs>
    <div className="pb-4">{body()}</div>
    <div className="sticky bottom-[calc(5.75rem+env(safe-area-inset-bottom))] z-10 flex flex-wrap items-center justify-end gap-2 rounded-2xl bg-sidebar/95 px-4 py-3 backdrop-blur md:static md:bottom-4 md:surface">
      {error ? <Message tone="error" className="mr-auto min-w-0 basis-full sm:basis-auto">{error}</Message>
        : <span className="mr-auto basis-full text-xs text-muted-foreground sm:basis-auto">
          {t("inventory.changes", { count: edits.total })}{edits.total > 0 && <> · {t("saves.editWarning")}</>}
        </span>}
      <Button variant="secondary" disabled={pending || !edits.total} onClick={edits.reset}>{t("inventory.discard")}</Button>
      <Button disabled={locked || !edits.valid || !edits.total} onClick={save}>{t("saves.saveEditedCharacter")}</Button>
    </div>
  </>;
}
