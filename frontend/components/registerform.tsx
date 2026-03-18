"use client";
import { useState } from "react";
import { useRouter } from "next/navigation";
import { useLang } from "../app/langContext";
import { buildApiUrl } from "../lib/api";

type Props = {
  switchToLogin: () => void;
};

export default function RegisterForm({ switchToLogin }: Props) {
  const [firstName, setFirstName] = useState("");
  const [lastName, setLastName] = useState("");
  const [username, setUsername] = useState("");
  const [email, setEmail] = useState("");
  const [password, setPassword] = useState("");
  const [confirmPassword, setConfirmPassword] = useState("");
  const [error, setError] = useState<string>("");
  const [isLoading, setIsLoading] = useState(false);
  const { t } = useLang();

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    setError("");
    setIsLoading(true);

    // Validate username
    if (!username.trim()) {
      setError(t.register_username_required || "Veuillez entrer un identifiant");
      setIsLoading(false);
      return;
    }

    if (username.length < 3) {
      setError(t.register_username_short || "L'identifiant doit contenir au moins 3 caractères");
      setIsLoading(false);
      return;
    }

    if (!password.trim()) {
      setError(t.register_password_required || "Veuillez entrer un mot de passe");
      setIsLoading(false);
      return;
    }

    // Validate password length
    if (password.length < 8) {
      setError(t.register_pw_short || "Le mot de passe doit contenir au moins 8 caractères");
      setIsLoading(false);
      return;
    }

    // Validate password match
    if (password !== confirmPassword) {
      setError(t.register_pw_mismatch || "Les mots de passe ne correspondent pas");
      setIsLoading(false);
      return;
    }

    // backend SignupRequest expects { username, password }
    fetch(buildApiUrl("/auth/signup"), {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ username, password }),
    })
      .then(async (res) => {
        if (!res.ok) {
          const txt = await res.text();

          // Parse error messages
          let errorMsg = "";
          try {
            const json = JSON.parse(txt);
            errorMsg = json.error || txt;
          } catch {
            errorMsg = txt;
          }

          // Handle specific errors
          if (res.status === 400 || errorMsg.toLowerCase().includes("already exists")) {
            throw new Error(t.register_username_exists || "Cet identifiant existe déjà");
          } else if (res.status === 400 || errorMsg.toLowerCase().includes("invalid")) {
            throw new Error(t.register_invalid_data || "Données invalides");
          } else if (res.status >= 500) {
            throw new Error(t.register_server_error || "Erreur serveur. Veuillez réessayer.");
          } else {
            throw new Error(errorMsg || "Erreur d'inscription");
          }
        }
        return res.json();
      })
      .then((data) => {
        setError("");
        // Show success message
        alert(t.register_success || "Inscription réussie! Vous pouvez maintenant vous connecter.");
        switchToLogin();
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
        {t.register_title}
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
            {t.register_username}
            <span className="text-gray-500 text-xs ml-1">(3-20 chars)</span>
          </label>
          <input
            type="text"
            value={username}
            onChange={(e) => setUsername(e.target.value)}
            disabled={isLoading}
            required
            placeholder="ex: john_doe"
            maxLength={20}
            className="w-full px-4 py-2 rounded-lg bg-white dark:bg-[#2A2A3D] text-black dark:text-white border border-gray-300 dark:border-gray-600 focus:outline-none focus:ring-2 focus:ring-blue-500 disabled:opacity-50 disabled:cursor-not-allowed"
          />
        </div>

        {/* Password */}
        <div>
          <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
            {t.register_password}
            <span className="text-gray-500 text-xs ml-1">(min. 8 chars)</span>
          </label>
          <input
            type="password"
            value={password}
            onChange={(e) => setPassword(e.target.value)}
            disabled={isLoading}
            required
            placeholder="••••••••"
            className="w-full px-4 py-2 rounded-lg bg-white dark:bg-[#2A2A3D] text-black dark:text-white border border-gray-300 dark:border-gray-600 focus:outline-none focus:ring-2 focus:ring-blue-500 disabled:opacity-50 disabled:cursor-not-allowed"
          />
        </div>

        {/* Confirm Password */}
        <div>
          <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
            {t.register_confirm_password}
          </label>
          <input
            type="password"
            value={confirmPassword}
            onChange={(e) => setConfirmPassword(e.target.value)}
            disabled={isLoading}
            required
            placeholder="••••••••"
            className="w-full px-4 py-2 rounded-lg bg-white dark:bg-[#2A2A3D] text-black dark:text-white border border-gray-300 dark:border-gray-600 focus:outline-none focus:ring-2 focus:ring-blue-500 disabled:opacity-50 disabled:cursor-not-allowed"
          />
        </div>

        {/* Submit Button */}
        <button
          type="submit"
          disabled={isLoading}
          className="w-full py-2 rounded-lg bg-blue-600 hover:bg-blue-700 text-white font-semibold transition disabled:bg-gray-500 disabled:cursor-not-allowed flex items-center justify-center gap-2"
        >
          {isLoading && <span className="animate-spin">⏳</span>}
          {isLoading ? "Inscription..." : t.register_submit}
        </button>
      </form>

      {/* Footer */}
      <p className="text-center text-sm text-gray-500 dark:text-gray-400 mt-4">
        {t.register_already}{" "}
        <button
          onClick={switchToLogin}
          disabled={isLoading}
          className="text-blue-600 dark:text-cyan-400 underline disabled:opacity-50"
        >
          {t.register_login_link}
        </button>
      </p>
    </div>
  );
}
