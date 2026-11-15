import { Suspense } from "react";
import { notFound } from "next/navigation";
import { viewFromPath } from "@/navigation";
import Workspace from "@/components/workspace";
export default async function SectionPage({
  params,
}: {
  params: Promise<{ section: string }>;
}) {
  const { section } = await params;
  if (!viewFromPath("/" + section)) notFound();
  return (
    <Suspense
      fallback={
        <div className="p-8" role="status">
          Abriendo QORI…
        </div>
      }
    >
      <Workspace />
    </Suspense>
  );
}
