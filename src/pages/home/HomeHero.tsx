import { useTranslation } from "react-i18next";
import { PlayButton } from "@/components/sidebar";
import { SummaryGrid } from "@/pages/home/SummaryGrid";
import hero from "@/assets/home-hero.webp";

/** The top of Home: a night city (an AI-generated image, docs/CREDITS.md) as the page's background,
 * edge to edge behind the content and fading into the page, with the title and Play over it. */
export function HomeHero() {
  const { t } = useTranslation();
  return (
    <>
      {/* Absolute to the scrolling main (AppWindow), behind the content (main isolates it). */}
      {/* Its lower part fades out (a mask) into the window's own background. */}
      <div className="pointer-events-none absolute inset-x-0 top-0 -z-10 h-[30rem] overflow-hidden [mask-image:linear-gradient(to_bottom,#000_55%,transparent)] md:h-[36rem]" aria-hidden="true">
        <img src={hero} alt="" draggable={false} className="size-full object-cover object-[72%_center]" />
        {/* The text side, and narrow screens where the text covers the city: shade it. */}
        <div className="absolute inset-0 bg-gradient-to-r from-background/90 via-background/55 to-background/0 md:from-background/85 md:via-background/30" />
      </div>
      <div className="flex min-h-64 flex-col justify-center pb-10 pt-6 md:min-h-80 md:pt-12">
        <p className="max-w-lg flex items-center gap-3 font-display text-[11px] font-semibold uppercase tracking-[0.3em] text-gold-400">
          <span className="h-px w-6 bg-gold-400/70" aria-hidden="true" />{t("home.eyebrow")}
        </p>
        <h1 className="max-w-lg mt-3 font-display text-3xl font-semibold tracking-wide text-gold-300 drop-shadow-lg sm:text-5xl">{t("home.title")}</h1>
        <p className="max-w-lg mt-3 text-sm text-parchment/80 drop-shadow">{t("home.subtitle")}</p>
        <PlayButton className="mt-8 hidden max-w-56 py-3.5 text-base md:flex" />
        <div className="mt-6 md:mt-4"><SummaryGrid /></div>
      </div>
    </>
  );
}
