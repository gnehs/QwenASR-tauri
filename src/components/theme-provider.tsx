import type { ReactNode } from "react";
import { ThemeProvider as NextThemesProvider } from "next-themes";

export const themeStorageKey = "qwenasr.theme";

export function ThemeProvider({ children }: { children: ReactNode }) {
  return (
    <NextThemesProvider
      attribute="class"
      defaultTheme="system"
      disableTransitionOnChange
      enableColorScheme
      enableSystem
      storageKey={themeStorageKey}
    >
      {children}
    </NextThemesProvider>
  );
}
