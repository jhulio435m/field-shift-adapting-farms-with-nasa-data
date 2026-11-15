"use client";

import { Dialog as Primitive } from "radix-ui";
import { ReactNode } from "react";
import { Button } from "@/components/ui/button";

export function Dialog({
  open,
  onOpenChange,
  title,
  description,
  children,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  title: string;
  description: string;
  children: ReactNode;
}) {
  return (
    <Primitive.Root open={open} onOpenChange={onOpenChange}>
      <Primitive.Portal>
        <Primitive.Overlay className="fixed inset-0 z-50 bg-black/40" />
        <Primitive.Content className="fixed left-1/2 top-1/2 z-50 max-h-[85svh] w-[calc(100%_-_2rem)] max-w-lg -translate-x-1/2 -translate-y-1/2 overflow-y-auto rounded-lg border bg-background p-6 text-foreground shadow-lg focus:outline-none">
          <Primitive.Title className="pr-12 text-xl font-semibold tracking-tight">
            {title}
          </Primitive.Title>
          <Primitive.Description className="mt-2 text-sm text-muted-foreground">
            {description}
          </Primitive.Description>
          <div className="mt-6">{children}</div>
          <Primitive.Close asChild>
            <Button variant="outline" className="mt-6 w-full">
              Cerrar
            </Button>
          </Primitive.Close>
        </Primitive.Content>
      </Primitive.Portal>
    </Primitive.Root>
  );
}
