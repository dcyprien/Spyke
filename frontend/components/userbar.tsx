"use client";

import Image from "next/image";
import { useState, useEffect, useRef } from "react";
import { useLang } from "../app/langContext";
import { authFetch } from "../lib/authFetch";
import { useAuth } from "../app/context";

type UserStatus = "online" | "offline" | "invisible";
// Seulement 2 choix MANUELS : en ligne ou invisible
// "offline" est géré automatiquement (fermeture / onglet caché)

const dotColor: Record<UserStatus, string> = {
  online:    "bg-green",
  offline:   "bg-grey-light",
  invisible: "bg-grey-light",
};

type Props = {
  username?: string;
  onStatusChange?: (status: UserStatus) => void;
  mobileTab?: string;
};

export default function UserControlPanel({ username: initialUsername, onStatusChange, mobileTab }: Props) {
  // "invisible" = choix manuel persisté ; sinon on bascule online/offline automatiquement
  const [invisible, setInvisible] = useState(false);
  const { t } = useLang();
  const { user, refreshUserData } = useAuth();
  // État affiché (calculé)
  const [status, setStatus] = useState<UserStatus>("online");
  const [statusOpen, setStatusOpen] = useState(false);
  const [avatar, setAvatar] = useState("/images/user.png");
  const [username, setUsername] = useState<string>(initialUsername || "");
  const [uploadOpen, setUploadOpen] = useState(false);
  const [uploading, setUploading] = useState(false);
  const [uploadError, setUploadError] = useState("");
  const [selectedFile, setSelectedFile] = useState<File | null>(null);
  const [preview, setPreview] = useState<string>("");
  const fileInputRef = useRef<HTMLInputElement>(null);

  const manualOptions: { key: "online" | "invisible"; label: string; color: string }[] = [
    { key: "online",    label: t.user_status_online,  color: "bg-green" },
    { key: "invisible", label: t.user_status_invisible, color: "bg-grey-light" },
  ];
  // Helper : envoie le statut au backend en mode keepalive (fonctionne même à la fermeture)
  const sendStatus = (newStatus: UserStatus) => {
    const token = typeof window !== "undefined" ? localStorage.getItem("access_token") : null;
    if (!token) return;
    fetch(`${process.env.NEXT_PUBLIC_API_URL}/auth/status`, {
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

    // Récupérer l'avatar depuis le contexte
    if (user?.avatar_url) {
      // Si l'URL est relative, construire l'URL complète avec le backend API
      if (user.avatar_url.startsWith("/uploads")) {
        setAvatar(`${process.env.NEXT_PUBLIC_API_URL}${user.avatar_url}`);
      } else {
        setAvatar(user.avatar_url);
      }
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
  }, [user?.avatar_url]);

  const handleSignOut = async () => {
    try {
      const token = localStorage.getItem("access_token");
      if (token) {
        await fetch(`${process.env.NEXT_PUBLIC_API_URL}/auth/logout`, {
          method: "POST",
          headers: { "Content-Type": "application/json", "Authorization": `Bearer ${token}` },
          body: JSON.stringify({}),
        });
      }
    } catch {}
    finally {
      localStorage.removeItem("access_token");
      localStorage.removeItem("username");
      window.location.href = `${process.env.NEXT_PUBLIC_HOME_URL || "http://localhost:3001"}`;
    }
  };

  const handleAvatarUpload = async (e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0];
    if (!file) return;

    // Validation du fichier
    if (!file.type.startsWith("image/")) {
      setUploadError("Veuillez sélectionner une image");
      return;
    }

    if (file.size > 5 * 1024 * 1024) { // 5MB max
      setUploadError("L'image ne doit pas dépasser 5MB");
      return;
    }

    // Créer la prévisualisation
    const reader = new FileReader();
    reader.onload = (e) => {
      setPreview(e.target?.result as string);
    };
    reader.readAsDataURL(file);

    setSelectedFile(file);
    setUploadError("");
  };

  const handleConfirmUpload = async () => {
    if (!selectedFile) return;

    setUploading(true);
    setUploadError("");

    const formData = new FormData();
    formData.append("avatar", selectedFile);

    try {
      const token = localStorage.getItem("access_token");
      const response = await fetch(`${process.env.NEXT_PUBLIC_API_URL}/auth/avatar`, {
        method: "POST",
        headers: { "Authorization": `Bearer ${token}` },
        body: formData,
      });

      if (!response.ok) {
        const error = await response.json().catch(() => ({ error: "Erreur lors de l'upload" }));
        setUploadError(error.error || "Erreur lors de l'upload");
        return;
      }

      const data = await response.json();
      // Construire l'URL complète du backend
      const fullAvatarUrl = data.avatar_url.startsWith("http")
        ? data.avatar_url
        : `${process.env.NEXT_PUBLIC_API_URL}${data.avatar_url}`;
      setAvatar(fullAvatarUrl);
      await refreshUserData();
      resetUpload();
    } catch (error) {
      setUploadError("Erreur de connexion");
    } finally {
      setUploading(false);
    }
  };

  const resetUpload = () => {
    setUploadOpen(false);
    setSelectedFile(null);
    setPreview("");
    setUploadError("");
    if (fileInputRef.current) fileInputRef.current.value = "";
  };

  return (
    <>
      {/* ── Upload Modal ── */}
      {uploadOpen && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/60" onClick={() => !uploading && resetUpload()}>
          <div className="bg-[#001839] border border-[#3D3D3D] rounded-xl p-6 w-96 shadow-lg" onClick={e => e.stopPropagation()}>
            <h3 className="text-white font-bold text-lg mb-4">Changer la photo de profil</h3>

            {/* Avatar preview */}
            <div className="flex justify-center mb-6">
              {preview ? (
                <img
                  src={preview}
                  alt="Preview"
                  className="w-32 h-32 rounded-full object-cover border-2 border-blue-500"
                />
              ) : (
                <Image
                  src={avatar}
                  alt="Current Avatar"
                  width={128}
                  height={128}
                  className="rounded-full object-cover border-2 border-grey-light"
                />
              )}
            </div>

            {/* File input */}
            <input
              ref={fileInputRef}
              type="file"
              accept="image/*"
              onChange={handleAvatarUpload}
              disabled={uploading}
              className="w-full px-3 py-2 bg-[#11111b] border border-[#3D3D3D] rounded text-white text-sm mb-3 cursor-pointer"
            />

            {/* Error message */}
            {uploadError && <p className="text-red-400 text-sm mb-3">{uploadError}</p>}

            {/* Upload status */}
            {uploading && <p className="text-blue-400 text-sm mb-3">Upload en cours...</p>}

            {/* Buttons */}
            <div className="flex gap-2">
              <button
                onClick={resetUpload}
                disabled={uploading}
                className="flex-1 px-3 py-2 rounded bg-[#11111b] text-gray-400 hover:text-white text-sm transition disabled:opacity-50"
              >
                Annuler
              </button>
              {selectedFile && (
                <button
                  onClick={handleConfirmUpload}
                  disabled={uploading}
                  className="flex-1 px-3 py-2 rounded bg-blue-600 hover:bg-blue-700 text-white text-sm transition disabled:opacity-50 font-semibold"
                >
                  {uploading ? "Upload..." : "Envoyer"}
                </button>
              )}
            </div>
          </div>
        </div>
      )}

      {/* ── Desktop widget (bottom-left corner) ── */}
      <div className="hidden md:flex fixed z-50 bottom-2 left-3 bg-dark-navy border border-grey-light rounded-xl px-3 py-2 items-center space-x-2 shadow-[0_4px_8px_rgba(0,0,0,0.4)]">
        <div className="flex items-center space-x-2">
          <button
            onClick={() => setUploadOpen(true)}
            className="relative cursor-pointer group"
            title="Changer la photo de profil"
          >
            <Image src={avatar} alt="User" width={48} height={48} className="rounded-full" />
            <div className="absolute inset-0 rounded-full bg-black/0 group-hover:bg-black/40 transition flex items-center justify-center">
              <span className="opacity-0 group-hover:opacity-100 text-white text-lg transition">📷</span>
            </div>
          </button>
          <div className="relative">
            <button
              onClick={() => setStatusOpen(!statusOpen)}
              className="w-10 h-10 rounded-xl flex items-center justify-center transition hover:bg-blue-mid hover:shadow-md"
            >
              <span className={`w-3 h-3 rounded-full ${dotColor[status]}`} />
            </button>
            {statusOpen && (
              <div className="absolute bottom-full right-15 mb-2 w-44 bg-grey rounded-xl shadow-lg border border-white/20 z-50">
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
                        className="w-full flex items-center gap-2 px-3 py-2 rounded-lg text-white text-sm hover:bg-grey-light transition"
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
        <div className="flex flex-col ml-2">
          <span className="text-sm font-semibold text-[cream]">{username || "User"}</span>
        </div>
      </div>

      {/* ── Mobile profile panel (shown on profile tab) ── */}
      {mobileTab === "profile" && (
        <div className="md:hidden fixed top-16 bottom-16 inset-x-0 z-10 bg-[#001839] overflow-y-auto flex flex-col items-center px-6 pt-10 pb-6 gap-6">

          {/* Avatar + name */}
          <div className="flex flex-col items-center gap-3">
            <button
              onClick={() => setUploadOpen(true)}
              className="relative group"
              title="Changer la photo de profil"
            >
              <Image src={avatar} alt="User" width={88} height={88} className="rounded-full border-4 border-blue-500 shadow-lg" />
              <div className="absolute inset-0 rounded-full bg-black/0 group-hover:bg-black/40 transition flex items-center justify-center">
                <span className="opacity-0 group-hover:opacity-100 text-white text-2xl transition">📷</span>
              </div>
            </button>
            <h2 className="text-white text-xl font-bold">{username || "User"}</h2>
            <span className={`flex items-center gap-2 text-sm font-medium px-3 py-1 rounded-full border ${
              status === "online" ? "border-green-500 text-green-400 bg-green-500/10" :
              "border-gray-500 text-gray-400 bg-gray-500/10"
            }`}>
              <span className={`w-2 h-2 rounded-full ${dotColor[status]}`} />
              {status === "online" ? t.user_status_online : status === "invisible" ? t.user_status_invisible : t.user_status_offline}
            </span>
          </div>

          {/* Status selector */}
          <div className="w-full max-w-sm space-y-2">
            <p className="text-gray-400 text-xs uppercase font-bold tracking-wider mb-3">Statut</p>
            {manualOptions.map((opt) => (
              <button
                key={opt.key}
                onClick={() => {
                  const isNowInvisible = opt.key === "invisible";
                  setInvisible(isNowInvisible);
                  localStorage.setItem("userInvisible", String(isNowInvisible));
                  applyStatus(opt.key);
                }}
                className={`w-full flex items-center gap-3 px-4 py-3 rounded-xl border transition ${
                  status === opt.key
                    ? "border-blue-500 bg-blue-500/20 text-white"
                    : "border-[#3D3D3D] bg-[#1E1E2E] text-gray-300 hover:bg-[#2A2A3D]"
                }`}
              >
                <span className={`w-3 h-3 rounded-full ${opt.color} flex-shrink-0`} />
                <span className="text-sm font-medium">{opt.label}</span>
                {status === opt.key && <span className="ml-auto text-blue-400 text-xs">✓</span>}
              </button>
            ))}
          </div>

          {/* Sign out */}
          <div className="w-full max-w-sm mt-auto">
            <button
              onClick={handleSignOut}
              className="w-full flex items-center justify-center gap-2 px-4 py-3 rounded-xl border border-gray-500/30 bg-gray-500/10 text-gray-400 font-semibold hover:bg-gray-600 hover:text-white transition"
            >
              <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M17 16l4-4m0 0l-4-4m4 4H7m6 4v1a3 3 0 01-3 3H6a3 3 0 01-3-3V7a3 3 0 013-3h4a3 3 0 013 3v1" />
              </svg>
              {t.nav_signout}
            </button>
          </div>
        </div>
      )}
    </>
  );
}
