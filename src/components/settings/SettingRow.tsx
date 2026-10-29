import type { ReactNode } from "react";
import { cn } from "@/lib";

type Props = { title: ReactNode; description?: ReactNode; children?: ReactNode; disabled?: boolean };

/** A setting: title and explanation on the left, its control on the right (below them on narrow
 *  screens). */
export function SettingRow({ title, description, children, disabled }: Props) {
  return (
    <div className={cn("flex flex-col items-stretch gap-3 px-4 py-3.5 sm:flex-row sm:items-center sm:justify-between sm:gap-6 sm:px-5",
      disabled && "pointer-events-none opacity-40")}>
      <div className="min-w-0">
        <div className="text-sm font-medium">{title}</div>
        {description && <div className="mt-0.5 text-xs leading-relaxed text-muted-foreground">{description}</div>}
      </div>
      <div className="flex min-w-0 shrink-0 flex-wrap items-center gap-2 [&>*]:max-w-full">{children}</div>
    </div>
  );
}
