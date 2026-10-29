import { useTranslation } from "react-i18next";
import type { Character } from "@/lib";
import { Button } from "@/components/ui";
import { ATTRIBUTES, type SaveEdits } from "@/pages/saves/editor/useSaveEdits";
import { Input } from "@/components/ui";


/** The character's Blood Echoes, Insight and attributes, with the level they make. */
export function CharacterSection({ character, edits, disabled }: { character: Character; edits: SaveEdits; disabled: boolean }) {
  const { t } = useTranslation();
  const { fields, setFields, level } = edits;
  if (!character.stats_consistent) return <p className="text-sm text-destructive">{t("saves.inconsistentStats")}</p>;
  const set = (field: keyof typeof fields, value: string) => setFields((all) => ({ ...all, [field]: value }));
  return <div className="grid gap-5 rounded-2xl surface p-4 sm:p-5">
    <div className="grid grid-cols-2 gap-4">
      <label className="grid gap-1 text-sm">{t("saves.echoes")}
        <Input type="number" min="0" max="999999999" step="1" value={fields.echoes} disabled={disabled} onChange={(event) => set("echoes", event.target.value)} />
      </label>
      <label className="grid gap-1 text-sm">{t("saves.insight")}
        <Input type="number" min="0" max="99" step="1" value={fields.insight} disabled={disabled} onChange={(event) => set("insight", event.target.value)} />
      </label>
    </div>
    <div>
      <div className="mb-2 flex items-center justify-between gap-2">
        <p className="text-sm font-medium">{t("saves.attributes")}</p>
        <Button variant="outline" size="sm" disabled={disabled} onClick={() =>
          setFields({ echoes: "999999999", insight: "99", ...Object.fromEntries(ATTRIBUTES.map((field) => [field, "99"])) } as typeof fields)}>
          {t("saves.maxAll")}</Button>
      </div>
      <div className="grid grid-cols-2 gap-4 sm:grid-cols-3">
        {ATTRIBUTES.map((field) => <label key={field} className="grid gap-1 text-sm">{t(`saves.${field}`)}
          <Input type="number" min={field === "vitality" ? "7" : field === "endurance" ? "8" : "1"} max="99" step="1"
            value={fields[field]} disabled={disabled} onChange={(event) => set(field, event.target.value)} />
        </label>)}
      </div>
      <p className="mt-3 text-sm">{t("saves.resultingLevel", { level: Number.isFinite(level) ? level : "—" })}</p>
      <p className="mt-1 text-xs text-muted-foreground">{t("saves.derivedValuesHint")}</p>
    </div>
  </div>;
}
