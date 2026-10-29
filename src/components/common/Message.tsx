import type { ReactNode } from "react";
import { cn } from "@/lib";

/** A line of feedback in a page or dialog: an error (announced) or a warning to act on. */
export function Message({ tone, className, children }: { tone: "error" | "warning"; className?: string; children: ReactNode }) {
  return (
    <p role={tone === "error" ? "alert" : undefined}
      className={cn("break-words text-sm", tone === "error" ? "text-destructive" : "text-blood-400", className)}>
      {children}
    </p>
  );
}
