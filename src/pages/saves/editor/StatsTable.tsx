import { useTranslation } from "react-i18next";
import type { InventoryStats } from "@/lib";
import { STAT_GROUPS, groupTitle, statLabel } from "@/pages/saves/editor/statLabels";

type Column = { label: string; stats: InventoryStats; chosen?: boolean };
type Props = {
  /** One column per set of numbers: a single one for an item, or several to compare. */
  columns: Column[];
  /** Compare: only the stats that differ between the columns, the best highlighted. */
  differences?: boolean;
};

/** The game's numbers by group (attack, scaling, requirements, defense), in one or more columns. */
export function StatsTable({ columns, differences = false }: Props) {
  const { t } = useTranslation();
  const groups = STAT_GROUPS.map((group) => {
    const keys = [...new Set(columns.flatMap((column) => Object.keys(column.stats[group] ?? {})))];
    const rows = keys.map((key) => ({ key, values: columns.map((column) => column.stats[group]?.[key] ?? 0) }))
      .filter(({ values }) => values.some((value) => value > 0))
      .filter(({ values }) => !differences || values.some((value) => value !== values[0]));
    return { group, rows };
  }).filter(({ rows }) => rows.length);
  if (!groups.length) return null;
  const headed = columns.some((column) => column.label);
  return <table className="w-full text-xs tabular-nums">
    {headed && <thead><tr>
      <th />
      {columns.map((column, at) => <th key={at} className={`pb-1 pl-3 text-right font-medium ${column.chosen ? "text-foreground" : "text-muted-foreground"}`}>{column.label}</th>)}
    </tr></thead>}
    {groups.map(({ group, rows }) => <tbody key={group}>
      <tr><th colSpan={columns.length + 1} className="pt-2 pb-0.5 text-left font-medium">{groupTitle(t, group)}</th></tr>
      {rows.map(({ key, values }) => {
        // Lower requirements are better; higher everything else.
        const best = group === "requirements" ? Math.min(...values) : Math.max(...values);
        const spread = values.some((value) => value !== values[0]);
        return <tr key={key}>
          <td className="text-muted-foreground">{statLabel(t, group, key)}</td>
          {values.map((value, at) => <td key={at} className={`pl-3 text-right ${spread && value === best ? "text-emerald-500" : ""} ${columns[at].chosen ? "font-medium" : ""}`}>{value}</td>)}
        </tr>;
      })}
    </tbody>)}
  </table>;
}
