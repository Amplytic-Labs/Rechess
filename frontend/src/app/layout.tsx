import type { Metadata } from "next";
import type { PropsWithChildren } from "react";

import "../../global.css";

export const metadata: Metadata = {
  title: "Rechess",
  description: "Online chess, one room at a time.",
};

/**
 * Root layout — proves the base app + theming run. F1/F4 add the real
 * screens as `src/app/**` route files rendering components from
 * `src/screens/**` (route files stay thin — see RULES.md §3). Theming needs
 * no provider: tokens live in `global.css` and follow the system theme.
 */
export default function RootLayout({ children }: PropsWithChildren) {
  return (
    <html lang="en" className="h-full antialiased">
      <body className="flex min-h-full flex-col bg-background text-foreground">
        {children}
      </body>
    </html>
  );
}
