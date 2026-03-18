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
  const { t } = useLang();

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    
    // Validate password match
    if (password !== confirmPassword) {
      alert(t.register_pw_mismatch);
      return;
    }
    
    // Validate password length
    if (password.length < 8) {
      alert(t.register_pw_short);
      return;
    }
    
    // Validate username
    if (!username.trim()) {
      alert(t.register_username_required);
      return;
    }

    // backend SignupRequest expects { username, password }
    fetch(buildApiUrl("/auth/signup"), {
      method: "POST",
      headers: { "Content-Type": "application/json" ,},
      body: JSON.stringify({ username, password }),
    })
      .then(async (res) => {
        if (!res.ok) {
          const txt = await res.text();
          throw new Error(txt || "Signup failed");
        }
        return res.json();
      })
      .then((data) => {
        alert(t.register_success);
        switchToLogin();
      })
      .catch((err) => {
        alert(t.register_error + err.message);
      });
  };

  const router = useRouter();

  return (
    <div className="max-w-md mx-auto p-6 bg-white dark:bg-[#1E1E2E] rounded-lg shadow-lg">
      <h1 className="text-2xl font-bold text-center text-heading dark:text-white mb-6">
        {t.register_title}
      </h1>

      <form onSubmit={handleSubmit} className="space-y-4">
        {/* Username */}
        <div>
          <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
            {t.register_username}
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
            {t.register_password}
          </label>
          <input
            type="password"
            value={password}
            onChange={(e) => setPassword(e.target.value)}
            required
            className="w-full px-4 py-2 rounded-lg bg-white dark:bg-[#2A2A3D] text-black dark:text-white border border-gray-300 dark:border-gray-600 focus:outline-none focus:ring-2 focus:ring-blue-500"
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
            required
            className="w-full px-4 py-2 rounded-lg bg-white dark:bg-[#2A2A3D] text-black dark:text-white border border-gray-300 dark:border-gray-600 focus:outline-none focus:ring-2 focus:ring-blue-500"
          />
        </div>

        {/* Submit Button */}
        <button
          type="submit"
          className="w-full py-2 rounded-lg bg-blue-600 hover:bg-blue-700 text-white font-semibold transition"
        >
          {t.register_submit}
        </button>
      </form>

      {/* Footer */}
      <p className="text-center text-sm text-gray-500 dark:text-gray-400 mt-4">
        {t.register_already}{" "}
        <button
          onClick={switchToLogin}
          className="text-blue-600 dark:text-cyan-400 underline"
        >
          {t.register_login_link}
        </button>
      </p>
    </div>
  );
}
