"use client";
import { useState } from "react";
import { useRouter } from "next/navigation";
import { useAuth } from "../app/context";
import { useLang } from "../app/langContext";
import { TranslationKey } from "../lib/i18n";

type Props = {
  switchToRegister: () => void;
};

// Helper function to get the right translation key for an error
function getErrorKey(status: number, errorMsg: string): TranslationKey | null {
  const msg = errorMsg.toLowerCase();

  // Vérifier les patterns d'erreur courants
  if (msg.includes("invalid") || msg.includes("unauthorized") || msg.includes("incorrect")) {
    return "login_invalid_credentials";
  }
  if (msg.includes("not found") || msg.includes("does not exist") || msg.includes("n'existe pas")) {
    return "login_user_not_found";
  }
  if (msg.includes("already") || msg.includes("existe")) {
    return "login_user_not_found";
  }
  
  // Vérifier les codes HTTP
  if (status === 401 || status === 403) {
    return "login_invalid_credentials";
  }
  if (status === 404) {
    return "login_user_not_found";
  }
  if (status === 429) {
    return "login_too_many_attempts";
  }
  if (status >= 500) {
    return "login_server_error";
  }

  // Fallback pour les autres cas
  return "login_generic_error";
}

export default function LoginForm({ switchToRegister }: Props) {
  const [username, setUsername] = useState("");
  const [password, setPassword] = useState("");
  const [errorKey, setErrorKey] = useState<TranslationKey | null>(null);
  const [errorMessage, setErrorMessage] = useState<string>("");
  const [isLoading, setIsLoading] = useState(false);
  const { t } = useLang();
  const { refreshUserData, connectWs } = useAuth();

 const handleSubmit = (e: React.FormEvent) => {
  e.preventDefault();
  setErrorKey(null);
  setErrorMessage("");
  setIsLoading(true);

  if (!username.trim()) {
    setErrorKey("login_username_required");
    setIsLoading(false);
    return;
  }

  if (!password.trim()) {
    setErrorKey("login_password_required");
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

        const errorKey = getErrorKey(res.status, errorMsg);
        throw new Error(JSON.stringify({ key: errorKey, fallback: errorMsg }));
      }
      return res.json();
    })
    .then(async (data) => {
      // expected: { access_token, refresh_token, id, username }

      if (!data.access_token) {
        setErrorKey("login_no_token");
        setIsLoading(false);
        return;
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
      try {
        const errorData = JSON.parse(err.message);
        // errorData.key contient toujours une clé traduite (getErrorKey ne retourne jamais null)
        setErrorKey(errorData.key);
        setErrorMessage("");
      } catch {
        // Fallback si parsing JSON échoue
        setErrorKey("login_generic_error");
        setErrorMessage("");
      }
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
      {(errorKey || errorMessage) && (
        <div className="mb-4 p-3 bg-red-500/20 border border-red-500/50 rounded-lg text-red-400 text-sm">
          <p className="font-semibold flex items-center gap-2">
            <span>⚠️</span>
            {errorKey ? t[errorKey] : errorMessage}
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
          {isLoading ? t.login_connecting : t.login_submit}
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
