import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui";
import { cn } from "@/lib";

type Props<T extends string | number> = {
  value: T;
  options: { value: T; label: string }[];
  onChange: (value: T) => void;
  label: string;
  disabled?: boolean;
  className?: string;
};

/** A dropdown of choices in the launcher's style (shadcn's Select on Base UI), for lists outside a
 * setting row: the trophy sort, a gem slot, a gem's effect. */
export function Choice<T extends string | number>({ value, options, onChange, label, disabled, className }: Props<T>) {
  return (
    <Select value={value} disabled={disabled} items={options} onValueChange={(next) => next !== null && onChange(next as T)}>
      <SelectTrigger aria-label={label} className={cn("min-w-0", className)}>
        <SelectValue />
      </SelectTrigger>
      <SelectContent className="w-auto min-w-(--anchor-width)">
        {options.map((option) => <SelectItem key={String(option.value)} value={option.value}>{option.label}</SelectItem>)}
      </SelectContent>
    </Select>
  );
}
