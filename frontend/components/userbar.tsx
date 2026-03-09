"use client";

import Image from "next/image";
import { useState, useEffect, useRef } from "react";
import { authFetch } from "../lib/authFetch";

type UserStatus = "online" | "offline" | "invisible";

// Seulement 2 choix MANUELS : en ligne ou invisible
// "offline" est géré automatiquement (fermeture / onglet caché)
const manualOptions: { key: "online" | "invisible"; label: string; color: string }[] = [
  { key: "online",    label: "En ligne",  color: "bg-green" },
  { key: "invisible", label: "Invisible", color: "bg-grey-light" },
];

const dotColor: Record<UserStatus, string> = {
  online:    "bg-green",
  offline:   "bg-red",
  invisible: "bg-grey-light",
};

type Props = {
  username?: string;
  onStatusChange?: (status: UserStatus) => void;
};

export default function UserControlPanel({ username: initialUsername, onStatusChange }: Props) {
  // "invisible" = choix manuel persisté ; sinon on bascule online/offline automatiquement
  const [invisible, setInvisible] = useState(false);
  // État affiché (calculé)
  const [status, setStatus] = useState<UserStatus>("online");
  const [statusOpen, setStatusOpen] = useState(false);
  const [avatar, setAvatar] = useState("/images/user.png");
  const [username, setUsername] = useState<string>(initialUsername || "");

  // Helper : envoie le statut au backend en mode keepalive (fonctionne même à la fermeture)
  const sendStatus = (newStatus: UserStatus) => {
    const token = typeof window !== "undefined" ? localStorage.getItem("access_token") : null;
    if (!token) return;
    fetch("http://localhost:3000/auth/status", {
      method: "PUT",
      headers: { "Authorization": `Bearer ${token}`, "Content-Type": "application/json" },
      body: JSON.stringify({ status: newStatus }),
      keepalive: true,
    }).catch(() => {});
  };

  const applyStatus = (newStatus: UserStatus) => {
    setStatus(newStatus);
    sendStatus(newStatus);
    if (onStatusChange) onStatusChange(newStatus);
  };

  useEffect(() => {
    // Récupérer username
    if (!username) {
      const stored = localStorage.getItem("username");
      if (stored) setUsername(stored);
    }

    // Restaurer préférence invisible
    const wasInvisible = localStorage.getItem("userInvisible") === "true";
    setInvisible(wasInvisible);

    // Au montage : on est connecté → appliquer le bon statut initial
    const initialStatus: UserStatus = wasInvisible ? "invisible" : "online";
    applyStatus(initialStatus);

    // --- Gestion automatique online/offline (si pas invisible) ---
    const handleVisibility = () => {
      if (localStorage.getItem("userInvisible") === "true") return;
      if (document.visibilityState === "hidden") {
        applyStatus("offline");
      } else {
        applyStatus("online");
      }
    };

    // Fermeture/rechargement de l'onglet
    const handlePageHide = () => {
      if (localStorage.getItem("userInvisible") === "true") return;
      sendStatus("offline");
    };

    document.addEventListener("visibilitychange", handleVisibility);
    window.addEventListener("pagehide", handlePageHide);

    return () => {
      document.removeEventListener("visibilitychange", handleVisibility);
      window.removeEventListener("pagehide", handlePageHide);
    };
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  return (
 <div className="
  fixed z-50 bottom-2 left-3
  bg-dark-navy
  border border-grey-light
  rounded-xl px-3 py-2 flex items-center space-x-2
  shadow-[0_4px_8px_rgba(0,0,0,0.4)]
">


      {/* Avatar + Status */}
      <div className="flex items-center space-x-2">

        {/* Avatar */}
        <div
          className="relative cursor-pointer"
        >
          <Image
            src={avatar}
            alt="User"
            width={48}
            height={48}
            className="rounded-full"
          />

        </div>

        {/* Bouton statut */}
        <div className="relative">
          <button
            onClick={() => setStatusOpen(!statusOpen)}
            className="
              w-10 h-10
              rounded-xl
              flex items-center justify-center
              transition
              hover:bg-blue-mid
              hover:shadow-md
            "
          >
            <span className={`w-3 h-3 rounded-full ${dotColor[status]}`} />
          </button>

          {/* Dropdown statut — seulement En ligne / Invisible */}
          {statusOpen && (
            <div
            className="
                absolute
                bottom-full
                right-15
                mb-2
                w-44
                bg-grey
                rounded-xl
                shadow-lg
                border border-white/20
                z-50
                "
            >
              <ul className="p-2 space-y-1">
                {manualOptions.map((opt) => (
                  <li key={opt.key}>
                    <button
                      onClick={() => {
                        const isNowInvisible = opt.key === "invisible";
                        setInvisible(isNowInvisible);
                        localStorage.setItem("userInvisible", String(isNowInvisible));
                        applyStatus(opt.key);
                        setStatusOpen(false);
                      }}
                      className="
                        w-full flex items-center gap-2
                        px-3 py-2
                        rounded-lg
                        text-white text-sm
                        hover:bg-grey-light
                        transition
                      "
                    >
                      <span className={`w-3 h-3 rounded-full ${opt.color}`} />
                      {opt.label}
                    </button>
                  </li>
                ))}
              </ul>
            </div>
          )}
        </div>

      </div>

      {/* Username */}
      <div className="flex flex-col ml-2">
         <span className="text-sm font-semibold text-[cream]">
            {username || "User"}
        </span>
    </div>
    </div>
  );
}
