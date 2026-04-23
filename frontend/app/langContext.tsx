"use client";

import { createContext, useContext, useState, useEffect, ReactNode, useMemo } from "react";
import { translations, Lang } from "../lib/i18n";

interface LangState {
  lang: Lang;
  t: typeof translations.fr;
  toggleLang: () => void;
}

const LangContext = createContext<LangState | undefined>(undefined);

export function LangProvider({ children }: { children: ReactNode }) {
  const [lang, setLang] = useState<Lang>("fr");
  const [mounted, setMounted] = useState(false);

  // Initialiser la langue depuis localStorage uniquement côté client
  useEffect(() => {
    const stored = localStorage.getItem("lang") as Lang | null;
    if (stored === "en" || stored === "fr") {
      setLang(stored);
    }
    setMounted(true);
  }, []);

  const toggleLang = () => {
    setLang(prev => {
      const next: Lang = prev === "fr" ? "en" : "fr";
      if (typeof window !== "undefined") localStorage.setItem("lang", next);
      return next;
    });
  };

  // Mémoriser la traduction et la valeur du contexte
  const t = useMemo(() => translations[lang], [lang]);
  const value = useMemo(() => ({ lang, t, toggleLang }), [lang, t, toggleLang]);

  // Éviter le flash lors de l'hydratation
  if (!mounted) {
    return (
      <LangContext.Provider value={{ lang: "fr", t: translations.fr, toggleLang }}>
        {children}
      </LangContext.Provider>
    );
  }

  return (
    <LangContext.Provider value={value}>
      {children}
    </LangContext.Provider>
  );
}

export function useLang() {
  const ctx = useContext(LangContext);
  if (!ctx) throw new Error("useLang must be used within a LangProvider");
  return ctx;
}
