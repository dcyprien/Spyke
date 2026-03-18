"use client";
import { useState } from "react";
import { useRouter } from "next/navigation";
import { useAuth } from "../app/context";
import { useLang } from "../app/langContext";

type Props = {
  switchToRegister: () => void;
};

export default function LoginForm({ switchToRegister }: Props) {
  const [username, setUsername] = useState("");
  const [password, setPassword] = useState("");
  const [error, setError] = useState<string>("");
  const [isLoading, setIsLoading] = useState(false);
  const { t } = useLang();
  const { refreshUserData, connectWs } = useAuth();

 const handleSubmit = (e: React.FormEvent) => {
  e.preventDefault();
  setError("");
  setIsLoading(true);

  if (!username.trim()) {
    setError(t.login_username_required || "Veuillez entrer votre identifiant");
    setIsLoading(false);
    return;
  }

  if (!password.trim()) {
    setError(t.login_password_required || "Veuillez entrer votre mot de passe");
    setIsLoading(false);
    return;
  }

  const apiUrl = process.env.NEXT_PUBLIC_API_URL || "http://localhost:3000";

  fetch(`${apiUrl}/auth/login`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ username, password }),
  })
    .then(async (res) => {
      if (!res.ok) {
        const text = await res.text();

        // Parse error messages
        let errorMsg = "";
        try {
          const json = JSON.parse(text);
          errorMsg = json.error || text;
        } catch {
          errorMsg = text;
        }

        // Handle specific errors
        if (res.status === 401 || errorMsg.toLowerCase().includes("invalid") || errorMsg.toLowerCase().includes("unauthorized")) {
          throw new Error(t.login_invalid_credentials || "Identifiant ou mot de passe incorrect");
        } else if (res.status === 404 || errorMsg.toLowerCase().includes("not found")) {
          throw new Error(t.login_user_not_found || "Cet identifiant n'existe pas");
        } else if (res.status === 429) {
          throw new Error(t.login_too_many_attempts || "Trop de tentatives. Réessayez plus tard.");
        } else if (res.status >= 500) {
          throw new Error(t.login_server_error || "Erreur serveur. Veuillez réessayer.");
        } else {
          throw new Error(errorMsg || "Erreur de connexion");
        }
      }
      return res.json();
    })
    .then(async (data) => {
      // expected: { access_token, refresh_token, id, username }

      if (!data.access_token) {
        throw new Error(t.login_no_token || "Erreur: Pas de token reçu");
      }

      localStorage.setItem("access_token", data.access_token);

      if (data.refresh_token) {
        localStorage.setItem("refresh_token", data.refresh_token);
      }

      if (data.username) {
        localStorage.setItem("username", data.username);
      }

      await refreshUserData();
      connectWs(); // <-- Reconnecte le WebSocket pour lui passer le nouveau token

      router.push("/main");
    })
    .catch((err) => {
      setError(err.message || "Une erreur est survenue");
    })
    .finally(() => {
      setIsLoading(false);
    });
};

  const router = useRouter();

  return (
    <div className="max-w-md mx-auto p-6 bg-white dark:bg-[#1E1E2E] rounded-lg shadow-lg">
      <h1 className="text-2xl font-bold text-center text-heading dark:text-white mb-6">
        {t.login_title}
      </h1>

      {/* Error message display */}
      {error && (
        <div className="mb-4 p-3 bg-red-500/20 border border-red-500/50 rounded-lg text-red-400 text-sm">
          <p className="font-semibold flex items-center gap-2">
            <span>⚠️</span>
            {error}
          </p>
        </div>
      )}

      <form onSubmit={handleSubmit} className="space-y-4">
        {/* Username */}
        <div>
          <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
            {t.login_username}
          </label>
          <input
            type="text"
            value={username}
            onChange={(e) => setUsername(e.target.value)}
            disabled={isLoading}
            required
            className="w-full px-4 py-2 rounded-lg bg-white dark:bg-[#2A2A3D] text-black dark:text-white border border-gray-300 dark:border-gray-600 focus:outline-none focus:ring-2 focus:ring-blue-500 disabled:opacity-50 disabled:cursor-not-allowed"
          />
        </div>

        {/* Password */}
        <div>
          <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
            {t.login_password}
          </label>
          <input
            type="password"
            value={password}
            onChange={(e) => setPassword(e.target.value)}
            disabled={isLoading}
            required
            className="w-full px-4 py-2 rounded-lg bg-white dark:bg-[#2A2A3D] text-black dark:text-white border border-gray-300 dark:border-gray-600 focus:outline-none focus:ring-2 focus:ring-blue-500 disabled:opacity-50 disabled:cursor-not-allowed"
          />
        </div>

        {/* Bouton */}
        <button
          type="submit"
          disabled={isLoading}
          className="w-full py-2 rounded-lg bg-blue-600 hover:bg-blue-700 text-white font-semibold transition disabled:bg-gray-500 disabled:cursor-not-allowed flex items-center justify-center gap-2"
        >
          {isLoading && <span className="animate-spin">⏳</span>}
          {isLoading ? "Connexion..." : t.login_submit}
        </button>
      </form>

      {/* Footer */}
      <p className="text-center text-sm text-gray-500 dark:text-gray-400 mt-4">
        {t.login_no_account}{" "}
        <button
          onClick={switchToRegister}
          disabled={isLoading}
          className="text-blue-600 dark:text-cyan-400 underline disabled:opacity-50"
        >
          {t.login_register_link}
        </button>
      </p>
    </div>
  );
}
