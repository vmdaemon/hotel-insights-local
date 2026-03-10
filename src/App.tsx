import { useEffect, useState } from "preact/hooks";
import "./App.css";

import { authStatus, dbInit } from "./lib/api";
import { Bootstrap } from "./pages/Bootstrap";
import { Home } from "./pages/Home";
import { Login } from "./pages/Login";

function App() {
  const [loading, setLoading] = useState(true);
  const [dbPath, setDbPath] = useState<string | null>(null);
  const [hasAdmin, setHasAdmin] = useState(false);
  const [loggedInUser, setLoggedInUser] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  async function refreshAuth() {
    const status = await authStatus();
    setHasAdmin(status.has_admin);
    setLoggedInUser(status.logged_in_user);
  }

  useEffect(() => {
    let cancelled = false;
    (async () => {
      try {
        const { db_path } = await dbInit();
        if (!cancelled) setDbPath(db_path);
        if (!cancelled) await refreshAuth();
      } catch (e) {
        if (!cancelled) setError(String(e));
      } finally {
        if (!cancelled) setLoading(false);
      }
    })();
    return () => {
      cancelled = true;
    };
  }, []);

  if (loading) {
    return (
      <main class="container">
        <h1>Hotel Insights</h1>
        <p>Loading...</p>
      </main>
    );
  }

  if (error) {
    return (
      <main class="container">
        <h1>Hotel Insights</h1>
        <p>{error}</p>
      </main>
    );
  }

  if (!hasAdmin) {
    return <Bootstrap onDone={refreshAuth} />;
  }

  if (!loggedInUser) {
    return <Login onDone={refreshAuth} />;
  }

  return (
    <Home username={loggedInUser} dbPath={dbPath} onLogout={refreshAuth} />
  );
}

export default App;
