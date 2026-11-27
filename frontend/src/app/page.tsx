/**
 * Placeholder home route — proves the base app + theming + routing run.
 * F1 replaces this with the real lobby home screen
 * (see src/screens/lobby/README.md).
 */
export default function HomePage() {
  return (
    <main className="flex flex-1 flex-col items-center justify-center gap-3 px-6">
      <h1 className="text-4xl font-bold text-foreground">Rechess</h1>
      <p className="text-center text-base text-muted-foreground">
        Team base app. F1 replaces this screen with the lobby.
      </p>
      <p className="text-sm text-muted-foreground">
        Frontend rules: see RULES.md · Your module: src/screens/lobby/README.md
      </p>
    </main>
  );
}
