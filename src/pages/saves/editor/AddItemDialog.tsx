import { useState } from "react";
import { useTranslation } from "react-i18next";
import type { CatalogItem, InventoryLocation } from "@/lib";
import { Button, Dialog, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle, DialogTrigger } from "@/components/ui";
import { AddItemPanel, type CatalogCategory } from "@/pages/saves/editor/AddItemPanel";

type Props = {
  /** The button's text and the dialog's title ("Add item", "Add weapon"...). */
  label: string;
  held: Record<InventoryLocation, Set<number>>;
  location: InventoryLocation;
  categories: CatalogCategory[];
  disabled: boolean;
  onAdd: (item: CatalogItem, amount: number, shape?: number) => void;
};

/** An Add button that opens what can be added in a dialog. It stays open to add more; what was
 * added waits in its list, saved with the rest of the edit. */
export function AddItemDialog({ label, held, location, categories, disabled, onAdd }: Props) {
  const { t } = useTranslation();
  const [open, setOpen] = useState(false);
  const [added, setAdded] = useState(0);
  return <Dialog open={open} onOpenChange={(next) => { setOpen(next); if (next) setAdded(0); }}>
    <DialogTrigger render={<Button size="sm" variant="secondary" disabled={disabled} />}>{label}</DialogTrigger>
    <DialogContent className="sm:max-w-2xl">
      <DialogHeader>
        <DialogTitle>{label}</DialogTitle>
        <DialogDescription>{t("inventory.addHint")}</DialogDescription>
      </DialogHeader>
      <AddItemPanel held={held} location={location} categories={categories} disabled={disabled} label={label}
        onAdd={(item, amount, shape) => { onAdd(item, amount, shape); setAdded((count) => count + 1); }} />
      <DialogFooter className="items-center">
        {added > 0 && <p className="mr-auto text-xs text-muted-foreground">{t("inventory.addedCount", { count: added })}</p>}
        <Button size="sm" onClick={() => setOpen(false)}>{t("inventory.done")}</Button>
      </DialogFooter>
    </DialogContent>
  </Dialog>;
}
