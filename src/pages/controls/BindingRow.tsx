import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { Crosshair, LoaderCircle, Mouse, RotateCcw } from "lucide-react";
import { toast } from "sonner";
import { IconAction } from "@/components/common";
import { Badge, Button } from "@/components/ui";
import { useIni, useSettingsData } from "@/hooks";
import { api } from "@/lib";
import { padLabel } from "@/lib/data";
import type { Control } from "@/lib/data";

type Props = { control: Control; kind: "pad" | "key" };

/** The runtime's names (src/runtime_pad.c mouse_keys) of the DOM's mouse buttons: 3 and 4 are
 *  back and forward, SDL's X1 and X2. */
const MOUSE_BUTTONS = ["Mouse Left", "Mouse Middle", "Mouse Right", "Mouse X1", "Mouse X2"];

/** A held modifier makes a chord ("Left Shift+Mouse Left"), as the runtime reads it. */
function withModifier(event: MouseEvent | WheelEvent, name: string): string {
  const modifier = event.shiftKey ? "Left Shift" : event.ctrlKey ? "Left Ctrl" : event.altKey ? "Left Alt" : "";
  return modifier ? `${modifier}+${name}` : name;
}

/** One input's binding: the current key or button, assign by pressing it, back to the default. */
export function BindingRow({ control, kind }: Props) {
  const { t } = useTranslation();
  const { ini } = useSettingsData();
  const setIni = useIni();
  const [capturing, setCapturing] = useState(false);
  const [mouse, setMouse] = useState(false);
  const iniKey = `${kind}.${control.id}`;
  const fallback = kind === "pad" ? control.pad : control.key;
  const custom = iniKey in ini;
  const binding = custom ? ini[iniKey] : (fallback ?? "");
  const shown = binding ? (kind === "pad" ? padLabel(binding, t("controls.or")) : binding) : t("common.notAssigned");

  const assign = async () => {
    setCapturing(true);
    try {
      const pressed = await api.readInput(kind);
      if (pressed) setIni(iniKey, pressed);
    } catch (error) {
      toast.error(t("common.saveFailed"), { description: String(error) });
    } finally {
      setCapturing(false);
    }
  };

  // Mouse assignment: the next click or wheel turn anywhere in the launcher is the binding.
  useEffect(() => {
    if (!mouse) return;
    const done = (name: string | null) => {
      setMouse(false);
      if (name) setIni(iniKey, name);
    };
    const onDown = (event: MouseEvent) => {
      event.preventDefault();
      event.stopPropagation();
      const name = MOUSE_BUTTONS[event.button];
      if (name) done(withModifier(event, name));
    };
    const onWheel = (event: WheelEvent) => {
      if (event.deltaY === 0) return;
      event.preventDefault();
      done(withModifier(event, event.deltaY < 0 ? "Wheel Up" : "Wheel Down"));
    };
    const onKey = (event: KeyboardEvent) => {
      if (event.key === "Escape") done(null);
    };
    const block = (event: Event) => event.preventDefault();
    // Listen from the next frame: the click that started this must not be taken.
    const start = window.setTimeout(() => {
      window.addEventListener("mousedown", onDown, true);
      window.addEventListener("wheel", onWheel, { capture: true, passive: false });
      window.addEventListener("contextmenu", block, true);
    }, 0);
    window.addEventListener("keydown", onKey, true);
    return () => {
      window.clearTimeout(start);
      window.removeEventListener("mousedown", onDown, true);
      window.removeEventListener("wheel", onWheel, true);
      window.removeEventListener("contextmenu", block, true);
      window.removeEventListener("keydown", onKey, true);
    };
  }, [mouse, iniKey, setIni]);

  if (kind === "pad" && control.pad === null) {
    return (
      <div className="flex items-center justify-between px-5 py-2.5 text-sm opacity-50">
        <span>{t(`controls.inputs.${control.id}` as "controls.inputs.cross")}</span>
        <span className="text-xs text-muted-foreground">{t("controls.keyboardOnly")}</span>
      </div>
    );
  }
  return (
    <div className="flex flex-wrap items-center justify-between gap-x-4 gap-y-2 px-4 py-2.5 sm:px-5">
      <div className="flex items-center gap-3 text-sm">
        {control.glyph && (
          <span className="flex size-7 items-center justify-center rounded-full bg-secondary text-sm text-blood-400">{control.glyph}</span>
        )}
        <span>{t(`controls.inputs.${control.id}` as "controls.inputs.cross")}</span>
      </div>
      <div className="flex flex-wrap items-center gap-2">
        {capturing || mouse ? (
          <span className="flex items-center gap-2 text-xs text-blood-400">
            <LoaderCircle className="size-3.5 animate-spin" /> {t(mouse ? "controls.pressMouse" : "controls.press")}
          </span>
        ) : (
          <Badge variant={custom ? "default" : "secondary"} className="font-mono text-[11px]">
            {shown}
          </Badge>
        )}
        <Button variant="secondary" size="sm" onClick={assign} disabled={capturing || mouse}>
          <Crosshair /> {t("controls.assign")}
        </Button>
        {kind === "key" && (
          <Button variant="secondary" size="sm" onClick={() => setMouse(true)} disabled={capturing || mouse}>
            <Mouse /> {t("controls.mouse")}
          </Button>
        )}
        <IconAction label={t("common.reset")} onClick={() => setIni(iniKey, null)} disabled={!custom}><RotateCcw /></IconAction>
      </div>
    </div>
  );
}
