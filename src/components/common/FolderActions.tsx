import { useTranslation } from "react-i18next";
import { open } from "@tauri-apps/plugin-dialog";
import { FolderOpen, FolderSearch } from "lucide-react";
import { Button } from "@/components/ui";
import { IconAction } from "@/components/common/IconAction";
import { useConfig } from "@/hooks";
import { api } from "@/lib";

type Props = {
  /** The folder in use. */
  path: string;
  /** The chooser's title. */
  title: string;
  /** Without it the folder cannot be changed (shown only). */
  onChoose?: (path: string) => void;
};

/** A folder's actions: choose another, and show it in the file manager (not on iOS, which has none). */
export function FolderActions({ path, title, onChoose }: Props) {
  const { t } = useTranslation();
  const { data: config } = useConfig();
  const choose = async () => {
    const picked = await open({ directory: true, defaultPath: path || undefined, title });
    if (typeof picked === "string") onChoose?.(picked);
  };
  return (
    <>
      {onChoose && <Button variant="secondary" size="sm" onClick={choose}><FolderSearch /> {t("common.choose")}</Button>}
      {config?.platform !== "ios" && <IconAction label={t("common.showFolder")} onClick={() => api.openPath(path)} disabled={!path}><FolderOpen /></IconAction>}
    </>
  );
}
