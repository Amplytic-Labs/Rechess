/**
 * Placeholder game route — F4 replaces this with the real game screen
 * (see src/screens/game/README.md). The room code comes from the URL; the
 * `player_token` NEVER appears in a URL (RULES.md §3) — F3's service keeps
 * it in storage and resumes the session after navigation.
 */
export default async function GamePage({
  params,
}: {
  params: Promise<{ roomCode: string }>;
}) {
  const { roomCode } = await params;

  return (
    <main className="flex flex-1 flex-col items-center justify-center gap-3 px-6">
      <h1 className="text-4xl font-bold text-foreground">Game {roomCode}</h1>
      <p className="text-center text-base text-muted-foreground">
        Team base app. F4 replaces this screen with the game composition.
      </p>
      <p className="text-sm text-muted-foreground">
        Frontend rules: see RULES.md · Your module: src/screens/game/README.md
      </p>
    </main>
  );
}
