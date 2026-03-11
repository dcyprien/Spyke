"use client";
import { useState } from "react";
import Image from "next/image";
import LoginForm from "../components/loginform";
import RegisterForm from "../components/registerform";
import { useLang } from "./langContext";

export default function LoginPage() {
  const [showRegister, setShowRegister] = useState(false);
  const { t, toggleLang } = useLang();

  return (
    <div className="min-h-screen flex flex-col items-center justify-center px-4">
      {/* Bouton langue en haut à droite */}
      <div className="fixed top-4 right-4">
        <button
          onClick={toggleLang}
          className="px-4 py-2 text-sm font-bold text-blue-400 bg-blue-500/10 rounded-lg border border-blue-500/30 hover:bg-blue-600 hover:text-white transition"
        >
          🌐 {t.nav_lang_switch}
        </button>
      </div>

      {/* Logo et nom */}
      <div className="flex flex-row items-center mb-8">
        <Image
          src="/images/spyke-logo.png"
          alt="Spyke Logo"
          width={64}
          height={64}
          className="rounded-full mb-4"
          priority
        />
        <h1 className="text-3xl font-bold text-heading dark:text-white">
          Pyke
        </h1>
      </div>

      {/* Formulaire */}
      <div className="w-full max-w-md">
        {showRegister ? (
          <RegisterForm switchToLogin={() => setShowRegister(false)} />
        ) : (
          <LoginForm switchToRegister={() => setShowRegister(true)} />
        )}
      </div>
    </div>
  );
}
