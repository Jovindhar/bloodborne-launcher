import { useTranslation } from "react-i18next";
import { toast } from "sonner";
import { Button, Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui";
import { TitleBar } from "@/components/window/TitleBar";
import { useConfig, useProfiles } from "@/hooks";
import { api } from "@/lib";
import { Message } from "@/components/common";

/** Keep recovery available even when configuration or database initialization fails. */
export function ProfileRecovery({ error }: { error: unknown }) {
  const { t } = useTranslation();
  const config = useConfig();
  const profiles = useProfiles();
  const retrying = config.isFetching || profiles.isFetching;

  return (
    <div className="flex h-full flex-col">
      <TitleBar />
      <section role="dialog" aria-modal="true" aria-labelledby="profile-recovery-title"
        aria-describedby="profile-recovery-description"
        className="flex flex-1 items-center justify-center px-8 pb-9">
        <Card className="w-full max-w-md">
          <CardHeader>
            <CardTitle id="profile-recovery-title" className="font-display text-2xl">
              {t("profiles.storageTitle")}
            </CardTitle>
            <CardDescription id="profile-recovery-description">{t("profiles.storageHelp")}</CardDescription>
          </CardHeader>
          <CardContent>
            <Message tone="error" className="mb-4">{String(error)}</Message>
            <div className="flex gap-2">
              <Button disabled={retrying} onClick={() => Promise.all([config.refetch(), profiles.refetch()])}>
                {t("profiles.retry")}
              </Button>
              <Button variant="outline" onClick={() => api.openLauncherData().catch((failure: unknown) => toast.error(String(failure)))}>
                {t("common.showFolder")}
              </Button>
            </div>
          </CardContent>
        </Card>
      </section>
    </div>
  );
}
