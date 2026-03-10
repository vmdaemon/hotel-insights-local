import { useState } from "preact/hooks";
import { authBootstrapCreateAdmin } from "../lib/api";

type Props = {
  onDone: () => void;
};

export function Bootstrap({ onDone }: Props) {
  const [username, setUsername] = useState("");
  const [password, setPassword] = useState("");
  const [confirmPassword, setConfirmPassword] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [submitting, setSubmitting] = useState(false);

  async function submit(e: Event) {
    e.preventDefault();
    setError(null);

    if (password !== confirmPassword) {
      setError("Passwords do not match");
      return;
    }

    setSubmitting(true);
    try {
      await authBootstrapCreateAdmin(username, password);
      onDone();
    } catch (err) {
      setError(String(err));
    } finally {
      setSubmitting(false);
    }
  }

  return (
    <main class="container">
      <h1>Create Admin</h1>
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
        <input
          type="password"
          value={confirmPassword}
          onInput={(e) => setConfirmPassword(e.currentTarget.value)}
          placeholder="Confirm password"
        />
        <button type="submit" disabled={submitting}>
          {submitting ? "Creating..." : "Create"}
        </button>
      </form>
      {error ? <p>{error}</p> : null}
    </main>
  );
}
