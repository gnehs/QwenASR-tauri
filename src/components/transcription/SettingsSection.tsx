import type { ReactNode } from "react";

export function SettingsSection({
  id,
  title,
  description,
  children,
}: {
  id: string;
  title: ReactNode;
  description: ReactNode;
  children: ReactNode;
}) {
  return (
    <section className="flex min-w-0 flex-col gap-4" aria-labelledby={id}>
      <div className="flex min-w-0 flex-col gap-1">
        <h2 id={id} className="font-sans text-lg leading-[1.35] font-semibold">
          {title}
        </h2>
        <p className="text-sm/relaxed text-muted-foreground">{description}</p>
      </div>
      <div className="flex min-w-0 flex-col gap-3">{children}</div>
    </section>
  );
}
