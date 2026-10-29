import background from "@/assets/page-background.webp";

/** The whole window (Home's own picture fades into it): a night street (an AI-generated image,
 * docs/CREDITS.md), darkened behind the whole window (menu and page), still while the page scrolls. */
export function PageBackground() {
  return (
    <div className="pointer-events-none absolute inset-0 -z-10 overflow-hidden" aria-hidden="true">
      <img src={background} alt="" draggable={false} className="size-full object-cover" />
      <div className="absolute inset-0 bg-background/60" />
    </div>
  );
}
