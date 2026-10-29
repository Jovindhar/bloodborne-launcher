import { useTranslation } from "react-i18next";
import type { SaveProfile } from "@/lib";
import { Message } from "@/components/common";

export function TrophyStatus({ profile }: { profile: SaveProfile }) {
  const { t } = useTranslation();
  return <>
    {!profile.trophies_eligible && <Message tone="warning" className="mt-2">{t("trophies.edited")}</Message>}
    {profile.trophy_error && <Message tone="error" className="mt-2">{t("trophies.error")} {profile.trophy_error}</Message>}
  </>;
}
