import type { ReactNode } from "react";
import { useTranslation } from "react-i18next";
import { RotateCcw } from "lucide-react";
import { FolderActions, IconAction } from "@/components/common";
import { SettingRow } from "@/components/settings/SettingRow";

type Props = {
  title: ReactNode;
  description?: ReactNode;
  /** The folder in use (the default when none was chosen). */
  path: string;
  /** Whether a folder was chosen (else the default is used). */
  custom?: boolean;
  /** Without it the folder cannot be changed (shown and revealed only). */
  onChoose?: (path: string) => void;
  onReset?: () => void;
};

/** A folder: its path, a chooser, showing it in the file manager, and back to the default. */
export function FolderRow({ title, description, path, custom, onChoose, onReset }: Props) {
  const { t } = useTranslation();
  return (
    <SettingRow
      title={title}
      description={
        <>
          {description && <span className="block">{description}</span>}
          <span className="mt-1 block max-w-md truncate font-mono text-[11px] text-muted-foreground/80" title={path}>
            {path || "—"}
          </span>
        </>
      }
    >
      <FolderActions path={path} title={String(title)} onChoose={onChoose} />
      {onReset && custom && <IconAction label={t("common.useDefault")} onClick={onReset}><RotateCcw /></IconAction>}
    </SettingRow>
  );
}
