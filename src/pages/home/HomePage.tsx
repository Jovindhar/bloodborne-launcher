import { useTranslation } from "react-i18next";
import { Info } from "lucide-react";
import { Alert, AlertDescription } from "@/components/ui";
import { GameFolderCard } from "@/pages/home/GameFolderCard";
import { HomeHero } from "@/pages/home/HomeHero";
import { useGameStore } from "@/stores";

export function HomePage() {
  const { t } = useTranslation();
  const lastExit = useGameStore((s) => s.lastExit);
  return (
    <>
      <HomeHero />
      {lastExit !== undefined && (
        <Alert className="mb-5" variant={lastExit === 0 || lastExit === null ? "default" : "destructive"}>
          <Info />
          <AlertDescription>
            {lastExit === 0 || lastExit === null ? t("home.lastRunOk") : t("home.lastRunError", { code: lastExit })}
          </AlertDescription>
        </Alert>
      )}
      <GameFolderCard />
    </>
  );
}
