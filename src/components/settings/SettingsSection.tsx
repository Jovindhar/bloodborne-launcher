import type { ReactNode } from "react";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui";
import { cn } from "@/lib";

type Props = { title?: string; description?: ReactNode; children: ReactNode; className?: string };

/** A card of setting rows. */
export function SettingsSection({ title, description, children, className }: Props) {
  return (
    <Card className={cn("mb-5 gap-0 py-0", className)}>
      {(title || description) && (
        <CardHeader className="px-4 pb-1 pt-4 sm:px-5">
          {title && (
            <CardTitle className="font-display text-xs font-semibold uppercase tracking-[0.2em] text-gold-400">
              {title}
            </CardTitle>
          )}
          {description && <CardDescription className="text-xs leading-relaxed">{description}</CardDescription>}
        </CardHeader>
      )}
      <CardContent className="px-0 pb-1">{children}</CardContent>
    </Card>
  );
}
