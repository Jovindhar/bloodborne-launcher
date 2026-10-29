import { useTranslation } from "react-i18next";
import { RefreshCw } from "lucide-react";
import { Button, Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui";
import { SettingRow } from "@/components/settings";
import { useDisplays, useSetting, useSettingsData } from "@/hooks";

/** Which monitor the game window and fullscreen open on (BB_DISPLAY): the main display, a
 *  connected one by name, or the saved one while it is not connected. */
export function DisplayPicker() {
  const { t } = useTranslation();
  const { settings } = useSettingsData();
  const setSetting = useSetting();
  const displays = useDisplays();
  const connected = displays.data ?? [];
  const chosen = settings.display ?? "";
  const items = [
    { value: "", label: t("graphics.monitorMain") },
    ...connected.map((d) => ({ value: d.name, label: `${d.name} · ${d.size}` })),
  ];
  if (chosen && !connected.some((d) => d.name === chosen)) {
    items.push({ value: chosen, label: t("controls.notConnected", { name: chosen }) });
  }
  return (
    <SettingRow title={t("graphics.monitor")} description={t("graphics.monitorHint")}>
      <Select value={chosen} items={items} onValueChange={(v) => v !== null && setSetting("display", v)}>
        <SelectTrigger className="min-w-0 flex-1 bg-secondary sm:min-w-64 sm:flex-none">
          <SelectValue />
        </SelectTrigger>
        <SelectContent>
          {items.map((item) => (
            <SelectItem key={item.value || "main"} value={item.value}>
              {item.label}
            </SelectItem>
          ))}
        </SelectContent>
      </Select>
      <Button variant="ghost" size="icon-sm" onClick={() => displays.refetch()} title={t("common.refresh")}>
        <RefreshCw className={displays.isFetching ? "animate-spin" : undefined} />
      </Button>
    </SettingRow>
  );
}
