import { authLogout } from "../lib/api";
import { ImportBookingsCsv } from "../components/ImportBookingsCsv";
import { OverviewDashboard } from "../components/OverviewDashboard";

type Props = {
  username: string;
  dbPath: string | null;
  onLogout: () => void;
};

export function Home({ username, dbPath, onLogout }: Props) {
  async function logout() {
    await authLogout();
    onLogout();
  }

  return (
    <main class="container">
      <h1>Hotel Insights</h1>
      <p>Signed in as: {username}</p>
      {dbPath ? <p>DB: {dbPath}</p> : null}

      <ImportBookingsCsv />

      <OverviewDashboard />

      <div class="row">
        <button type="button" onClick={logout}>
          Logout
        </button>
      </div>
    </main>
  );
}
