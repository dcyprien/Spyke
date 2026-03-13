"use client";

import { createContext, useContext, useState, useEffect, ReactNode } from "react";
import { translations, Lang } from "../lib/i18n";

interface LangState {
  lang: Lang;
  t: typeof translations[Lang];
  toggleLang: () => void;
}

const LangContext = createContext<LangState | undefined>(undefined);

export function LangProvider({ children }: { children: ReactNode }) {
  const [lang, setLang] = useState<Lang>("fr");

  useEffect(() => {
    const stored = localStorage.getItem("lang") as Lang | null;
    if (stored === "en" || stored === "fr") {
      setLang(stored);
    }
  }, []);

  const toggleLang = () => {
    setLang(prev => {
      const next: Lang = prev === "fr" ? "en" : "fr";
      if (typeof window !== "undefined") localStorage.setItem("lang", next);
      return next;
    });
  };

  const t = translations[lang];

  return (
    <LangContext.Provider value={{ lang, t, toggleLang }}>
      {children}
    </LangContext.Provider>
  );
}

export function useLang() {
  const ctx = useContext(LangContext);
  if (!ctx) throw new Error("useLang must be used within a LangProvider");
  return ctx;
}
