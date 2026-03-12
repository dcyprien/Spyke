"use client";

import Image from "next/image";
import { useEffect, useState } from "react";
import { useRouter } from "next/navigation";
import { Server } from "./chatbar";
import { useLang } from "../app/langContext";

type Props = {
  selectedServer?: Server | null;
};

export default function Navbar({ selectedServer }: Props) {
  const [darkMode, setDarkMode] = useState(true);
  const router = useRouter();
  const { t, toggleLang } = useLang();

  const handleSignOut = async () => {
    try {
      const token = localStorage.getItem("access_token");
      
      if (token) {
        await fetch("http://localhost:3000/auth/logout", {
          method: "POST",
          headers: {
            "Content-Type": "application/json",
            "Authorization": `Bearer ${token}`,
          },
          body: JSON.stringify({}),
        });
      }
    } catch (error) {
      console.error("Erreur lors de la déconnexion backend", error);
    } finally {
      localStorage.removeItem("access_token");
      localStorage.removeItem("username");
      
      window.location.href = "http://localhost:3001";
    }
  };
  
  return (
    <nav className="fixed z-50 w-full shadow-md bg-cream dark:bg-blue-mid transition-colors">
      <div className="flex items-center justify-between px-4 py-3">

        {/* Logo */}
        <a className="flex items-center gap-2">
          <Image src="/images/spyke-logo.png" alt="Spyke Logo" width={33} height={32} className="rounded-full" />
          <span className="text-xl text-heading font-semibold">pyke</span>
        </a>

        {/* Channel name – centre, desktop only */}
        <div className="hidden md:block text-heading text-sm font-medium">
          {selectedServer ? `# ${selectedServer.name}` : t.nav_no_server}
        </div>

        {/* Actions */}
        <div className="flex items-center gap-2">
          {/* Langue */}
          <button
            onClick={toggleLang}
            className="relative overflow-hidden px-3 py-2 text-sm font-bold text-blue-400 bg-blue-500/10 rounded-lg border border-blue-500/30 transition-all hover:bg-blue-600 hover:text-white hover:scale-105 active:scale-95 group"
          >
            <span className="absolute inset-0 bg-gradient-to-r from-transparent via-white/10 to-transparent -translate-x-full group-hover:animate-[shimmer_1.5s_infinite]" />
            <span className="relative flex items-center gap-1">
              🌐 <span className="hidden sm:inline">{t.nav_lang_switch}</span>
            </span>
          </button>

          {/* Sign out */}
          <button
            onClick={handleSignOut}
            className="relative overflow-hidden px-3 py-2 text-sm font-bold text-red-400 bg-red-500/10 rounded-lg border border-red-500/30 transition-all hover:bg-red-600 hover:text-white hover:scale-105 active:scale-95 group"
          >
            <span className="absolute inset-0 bg-gradient-to-r from-transparent via-white/10 to-transparent -translate-x-full group-hover:animate-[shimmer_1.5s_infinite]" />
            <span className="relative flex items-center gap-1">
              <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth="2" d="M17 16l4-4m0 0l-4-4m4 4H7m6 4v1a3 3 0 01-3 3H6a3 3 0 01-3-3V7a3 3 0 013-3h4a3 3 0 013 3v1" />
              </svg>
              <span className="hidden sm:inline">{t.nav_signout}</span>
            </span>
          </button>
        </div>
      </div>
    </nav>
  );
}
