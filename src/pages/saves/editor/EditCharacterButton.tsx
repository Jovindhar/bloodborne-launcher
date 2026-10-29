import { useTranslation } from "react-i18next";
import { SquarePen } from "lucide-react";
import type { Character } from "@/lib";
import { Button } from "@/components/ui";
import { useUiStore } from "@/stores";
import { useItemCatalog } from "@/pages/saves/editor/useItemCatalog";

/** Opens a character in the save editor (the Saves tab becomes the editor). Names, icons and numbers
 * come from the player's game files: without them (no game folder, or one the item tables cannot be
 * read from) it stays disabled and says why. */
export function EditCharacterButton({ character, disabled }: { character: Character; disabled: boolean }) {
  const { t } = useTranslation();
  const setEditing = useUiStore((state) => state.setEditing);
  const catalog = useItemCatalog();
  const ready = (catalog.data?.length ?? 0) > 0;
  return <>
    <Button variant="secondary" size="sm" disabled={disabled || !ready} onClick={() => setEditing(`${character.title_id}:${character.slot}`)}>
      <SquarePen aria-hidden="true" /> {t("saves.editCharacter")}
    </Button>
    {!ready && !catalog.isLoading && <p className="basis-full text-xs text-muted-foreground">{t("saves.editorNeedsGame")}</p>}
  </>;
}
