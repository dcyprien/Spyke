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
  const [dropdownOpen, setDropdownOpen] = useState(false);
  const [menuOpen, setMenuOpen] = useState(false);
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

      <div className="max-w-screen-xl flex flex-wrap items-center justify-between mx-auto p-4">
        
        {/* Logo */}
        <a className="flex items-center rtl:space-x-reverse">
          <Image
            src="/images/spyke-logo.png"
            alt="Spyke Logo"
            width={32}
            height={32}
            className="rounded-full"
          />
          <span className="self-center text-xl text-heading font-semibold whitespace-nowrap">
            pyke
          </span>
        </a>

        {/* Boutons utilisateur */}
        <div className="flex items-center md:order-2 space-x-3 md:space-x-2 rtl:space-x-reverse">


      <div className="flex items-center gap-4">    

          {/* Bouton langue */}
          <button
          onClick={toggleLang}
          className="
            relative overflow-hidden
            px-4 py-2
            text-sm font-bold text-blue-400
            bg-blue-500/10 
            rounded-lg
            border border-blue-500/30
            transition-all duration-300 ease-out
            hover:bg-blue-600 hover:text-white hover:scale-105 hover:shadow-[0_0_15px_rgba(59,130,246,0.4)]
            active:scale-95
            group
          "
        >
          {/* Effet de reflet qui passe sur le bouton au survol */}
          <span className="absolute inset-0 w-full h-full bg-gradient-to-r from-transparent via-white/10 to-transparent -translate-x-full group-hover:animate-[shimmer_1.5s_infinite] transition-transform"></span>
          
          <span className="relative flex items-center gap-2">
            🌐 {t.nav_lang_switch}
          </span>
        </button>

        {/* Bouton Sign Out direct avec effets */}
        <button
          onClick={handleSignOut}
          className="
            relative overflow-hidden
            px-4 py-2
            text-sm font-bold text-red-400
            bg-red-500/10 
            rounded-lg
            border border-red-500/30
            transition-all duration-300 ease-out
            hover:bg-red-600 hover:text-white hover:scale-105 hover:shadow-[0_0_15px_rgba(239,68,68,0.4)]
            active:scale-95
            group
          "
        >
          {/* Effet de reflet qui passe sur le bouton au survol */}
          <span className="absolute inset-0 w-full h-full bg-gradient-to-r from-transparent via-white/10 to-transparent -translate-x-full group-hover:animate-[shimmer_1.5s_infinite] transition-transform"></span>
          
          <span className="relative flex items-center gap-2">
            <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24" xmlns="http://www.w3.org/2000/svg">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth="2" d="M17 16l4-4m0 0l-4-4m4 4H7m6 4v1a3 3 0 01-3 3H6a3 3 0 01-3-3V7a3 3 0 013-3h4a3 3 0 013 3v1" />
            </svg>
            {t.nav_signout}
          </span>
        </button>
      </div>

          {/* Menu burger pour mobile */}
          <button
            type="button"
            className="inline-flex items-center p-2 w-10 h-10 justify-center text-sm text-body rounded-base md:hidden hover:bg-neutral-secondary-soft hover:text-heading focus:outline-none focus:ring-2 focus:ring-neutral-tertiary"
            onClick={() => setMenuOpen(!menuOpen)}
          >
            <span className="sr-only">Open main menu</span>
            <svg className="w-6 h-6" aria-hidden="true" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke="currentColor">
              <path strokeLinecap="round" strokeWidth="2" d="M5 7h14M5 12h14M5 17h14" />
            </svg>
          </button>
        </div>

        {/* Menu principal */}
        <div className={`${menuOpen ? "flex" : "hidden"} items-center justify-between w-full md:flex md:w-auto md:order-1`} id="navbar-user">
          <ul className="font-medium flex flex-col p-4 md:p-0 mt-4 rounded-base bg-neutral-secondary-soft md:flex-row md:space-x-8 md:mt-0 md:border-0 md:bg-neutral-primary">
            <li>
              <a href="#" className="block py-2 px-3 text-heading rounded hover:bg-neutral-tertiary md:hover:bg-transparent md:border-0 md:hover:text-fg-brand md:p-0">
                {selectedServer ? `#${selectedServer.name}` : t.nav_no_server}
              </a>
            </li>
          </ul>
        </div>

      </div>
    </nav>
  );
}
