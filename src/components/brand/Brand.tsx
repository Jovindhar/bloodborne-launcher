import { useTranslation } from "react-i18next";
import { Moon } from "@/components/brand/Moon";

/** The moon, the name and the tagline: stacked in the sidebar, on one line in the narrow header. */
export function Brand({ inline = false }: { inline?: boolean }) {
  const { t } = useTranslation();
  const tagline = <div className="text-[10px] uppercase tracking-[0.25em] text-muted-foreground">{t("app.tagline")}</div>;
  return inline ? (
    <div className="flex items-center gap-2.5">
      <Moon className="size-7" />
      <div className="font-display text-base font-semibold tracking-wider">{t("app.name")}</div>
      {tagline}
    </div>
  ) : (
    <div className="flex items-center gap-3">
      <Moon className="size-9" />
      <div>
        <div className="font-display text-lg font-semibold leading-tight tracking-wider">{t("app.name")}</div>
        {tagline}
      </div>
    </div>
  );
}
