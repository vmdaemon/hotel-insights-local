import { useState } from "preact/hooks";
import { authLogin } from "../lib/api";

type Props = {
  onDone: () => void;
};

export function Login({ onDone }: Props) {
  const [username, setUsername] = useState("");
  const [password, setPassword] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [submitting, setSubmitting] = useState(false);

  async function submit(e: Event) {
    e.preventDefault();
    setError(null);
    setSubmitting(true);
    try {
      await authLogin(username, password);
      onDone();
    } catch (err) {
      setError(String(err));
    } finally {
      setSubmitting(false);
    }
  }

  return (
    <main class="container">
      <h1>Login</h1>
      <form class="row" onSubmit={submit}>
        <input
          value={username}
          onInput={(e) => setUsername(e.currentTarget.value)}
          placeholder="User ID"
          autoCapitalize="none"
          autoCorrect="off"
        />
        <input
          type="password"
          value={password}
          onInput={(e) => setPassword(e.currentTarget.value)}
          placeholder="Password"
        />
        <button type="submit" disabled={submitting}>
          {submitting ? "Signing in..." : "Sign in"}
        </button>
      </form>
      {error ? <p>{error}</p> : null}
    </main>
  );
}
