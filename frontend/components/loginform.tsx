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
  const { t } = useLang();
  const { refreshUserData, connectWs } = useAuth(); 

 const handleSubmit = (e: React.FormEvent) => {
  e.preventDefault();

  const apiUrl = process.env.NEXT_PUBLIC_API_URL || "http://localhost:3000";

  fetch(`${apiUrl}/auth/login`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ username, password }),
  })
    .then(async (res) => {
      if (!res.ok) {
        const text = await res.text();
        throw new Error(text || "Login failed");
      }
      return res.json();
    })
    .then(async (data) => {
      // expected: { access_token, refresh_token, id, username }

      if (!data.access_token) {
        throw new Error("No access token received");
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
      alert(t.login_error + err.message);
    });
};

  const router = useRouter();

  return (
    <div className="max-w-md mx-auto p-6 bg-white dark:bg-[#1E1E2E] rounded-lg shadow-lg">
      <h1 className="text-2xl font-bold text-center text-heading dark:text-white mb-6">
        {t.login_title}
      </h1>

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
            required
            className="w-full px-4 py-2 rounded-lg bg-white dark:bg-[#2A2A3D] text-black dark:text-white border border-gray-300 dark:border-gray-600 focus:outline-none focus:ring-2 focus:ring-blue-500"
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
            required
            className="w-full px-4 py-2 rounded-lg bg-white dark:bg-[#2A2A3D] text-black dark:text-white border border-gray-300 dark:border-gray-600 focus:outline-none focus:ring-2 focus:ring-blue-500"
          />
        </div>

        {/* Bouton */}
        <button
          type="submit"
          className="w-full py-2 rounded-lg bg-blue-600 hover:bg-blue-700 text-white font-semibold transition"
        >
          {t.login_submit}
        </button>
      </form>

      {/* Footer */}
      <p className="text-center text-sm text-gray-500 dark:text-gray-400 mt-4">
        {t.login_no_account}{" "}
        <button
          onClick={switchToRegister}
          className="text-blue-600 dark:text-cyan-400 underline"
        >
          {t.login_register_link}
        </button>
      </p>
    </div>
  );
}
