"use client";

import { createContext, useContext, useState, useEffect, ReactNode } from "react";
import { Server } from "../components/chatbar"; // Assurez-vous que le chemin est bon

export interface User {
  id: string;
  username: string;
  display_name?: string;
  avatar_url?: string;
  serverList?: Server[];
}

interface AuthState {
  user: User | null;
  servers: Server[];
  isLoading: boolean;
  socket: WebSocket | null;
  banNotification: { message: string; serverId: number } | null;
  clearBanNotification: () => void;
  addServer: (server: Server) => void;
  setServers: (servers: Server[]) => void; 
  logout: () => void;
  refreshUserData: () => Promise<void>; // Utile pour recharger sans F5
}

const AuthContext = createContext<AuthState | undefined>(undefined);

export function AuthProvider({ children }: { children: ReactNode }) {
  const [user, setUser] = useState<User | null>(null);
  const [servers, setServers] = useState<Server[]>([]);
  const [isLoading, setIsLoading] = useState(true);
  const [socket, setSocket] = useState<WebSocket | null>(null);
  const [banNotification, setBanNotification] = useState<{ message: string; serverId: number } | null>(null);

  const clearBanNotification = () => setBanNotification(null);

  // Ref pour accéder à l'état user courant dans le callback WS sans le recréer
  const userRef = { current: user };

  // Fonction unique de récupération de données
  const refreshUserData = async () => {
    const token = localStorage.getItem("access_token");
    if (!token) {
      setIsLoading(false);
      return;
    }

    try {
      const res = await fetch("http://localhost:3000/me", {
        headers: { Authorization: `Bearer ${token}` },
      });

      if (res.ok) {
        const data = await res.json();

        setUser({
          id: data.id,
          username: data.username,
          display_name: data.display_name,
          avatar_url: data.avatar_url,
        });
        // Stocker l'ID pour la d\u00e9tection des events WS (kick/ban)
        localStorage.setItem("current_user_id", data.id);

        const mappedServers = (data.servers || []).map((s: any) => ({
          ...s,
          invitcode: s.invitcode ? String(s.invitcode) : "",
        }));
        setServers(mappedServers);
      } else {
        localStorage.removeItem("access_token");
      }
    } catch (error) {
      console.error("Auth init failed", error);
    } finally {
      setIsLoading(false);
    }
  };

  useEffect(() => {
    // Eviter de créer un socket si on n'est pas sur le client
    if (typeof window === "undefined") return;

    // FIX 1: Déclarer ws ici pour qu'il soit accessible au cleanup ET à connectWs
    let ws: WebSocket | null = null;

    const connectWs = () => {
        const wsUrl = "ws://localhost:3000/ws";
        
        // FIX 2: Ne pas utiliser 'const', on assigne la variable du scope parent
        ws = new WebSocket(wsUrl);

        ws.onopen = () => {
            console.log("🟢 WS Connecté");
            const token = localStorage.getItem("access_token");
            if (token) {
                ws?.send(JSON.stringify({ type: "auth", token: token }));
            }
        };

        ws.onclose = () => console.log("🔴 WS Déconnecté");
        ws.onerror = (err) => console.error("⚠️ WS Erreur", err);

        ws.onmessage = async (event) => {
            try {
                const parsed = JSON.parse(event.data);
                
                if (parsed.type === "auth_success") {
                    console.log("🔒 Auth WS réussie, chargement des données...");
                    await refreshUserData(); 
                }

                if (!parsed || !parsed.data) return;
                const data = parsed.data;

                // --- GESTION DU STATUT EN TEMPS RÉEL (Vert/Gris) ---
                if (parsed.type === "user_status_change") {
                    console.log("⚡ Update Statut Reçu:", data); // Log de debug
                    setServers((prev: any[]) => prev.map(server => {
                        // Utiliser String() assure que 1 (number) == "1" (string)
                        if (String(server.id) === String(data.server_id)) {
                            return {
                                ...server,
                                members: (server.members || []).map((m: any) => {
                                    // Comparaison sécurisée user_id
                                    if (String(m.user_id) === String(data.user_id)) {
                                        return { ...m, status: data.status }; 
                                    }
                                    return m;
                                })
                            };
                        }
                        return server;
                    }));
                }
                // ---------------------------------------------------

                if (parsed.type === "user_kicked") {
                    // Supprimer le membre de la liste ET notifier si c'est moi
                    const currentUserId = localStorage.getItem("current_user_id");
                    if (currentUserId && String(data.user_id) === String(currentUserId)) {
                        setBanNotification({ message: "Vous avez été expulsé de ce serveur.", serverId: data.server_id });
                        setServers((prev: any[]) => prev.filter((s: any) => s.id !== data.server_id));
                    } else {
                        setServers((prev: any[]) => prev.map((server: any) => {
                            if (server.id === data.server_id) {
                                return { ...server, members: (server.members || []).filter((m: any) => String(m.id) !== String(data.member_id)) };
                            }
                            return server;
                        }));
                    }
                }

                if (parsed.type === "user_banned") {
                    const currentUserId = localStorage.getItem("current_user_id");
                    if (currentUserId && String(data.user_id) === String(currentUserId)) {
                        let msg: string;
                        if (!data.banned_until) {
                            msg = "❌ Vous avez été banni définitivement de ce serveur. Vous ne pouvez plus le rejoindre.";
                        } else {
                            // Calculer la différence pour détecter un kick (10s)
                            const until = new Date(data.banned_until);
                            const diffSecs = Math.round((until.getTime() - Date.now()) / 1000);
                            if (diffSecs <= 15) {
                                msg = "🚪 Vous avez été expulsé de ce serveur. Vous pourrez rejoindre dans quelques secondes.";
                            } else {
                                const hours = Math.floor(diffSecs / 3600);
                                const days = Math.floor(diffSecs / 86400);
                                if (days >= 1) msg = `⏳ Vous avez été banni temporairement pour ${days} jour(s).`;
                                else if (hours >= 1) msg = `⏳ Vous avez été banni temporairement pour ${hours} heure(s).`;
                                else msg = `⏳ Vous avez été banni temporairement pour ${Math.ceil(diffSecs / 60)} minute(s).`;
                            }
                        }
                        setBanNotification({ message: msg, serverId: data.server_id });
                        setServers((prev: any[]) => prev.filter((s: any) => s.id !== data.server_id));
                    } else {
                        // Retirer le membre banni de la liste pour les autres
                        setServers((prev: any[]) => prev.map((server: any) => {
                            if (server.id === data.server_id) {
                                return { ...server, members: (server.members || []).filter((m: any) => String(m.id) !== String(data.member_id)) };
                            }
                            return server;
                        }));
                    }
                }

                if (parsed.type === "user_joined") {
                    setServers((prevServers: any[]) => {
                        return prevServers.map((server) => {
                            if (server.id === data.server_id) {
                                const currentMembers = server.members || [];
                                // Fix: Comparer user_id pour éviter doublons sur reconnexion
                                const exists = currentMembers.some((m: any) => String(m.user_id) === String(data.member.user_id));
                                
                                if (!exists) {
                                    return {
                                        ...server,
                                        members: [...currentMembers, data.member]
                                    };
                                }
                            }
                            return server;
                        });
                    });
                }
                
                // MODIFICATION ICI : On recharge tout pour gérer les changements de rôles (Admins/Owner)
                if (parsed.type === "member_updated") {
                    console.log("🔄 Update Member reçue, rafraîchissement des permissions...");
                    await refreshUserData();
                }

            } catch (error) {
                console.error("Erreur parsing WS context", error);
            }
        };

        setSocket(ws);
    };

    connectWs();

    // Cleanup: ws est maintenant accessible ici grâce au FIX 1
    return () => {
        if (ws) ws.close();
    };
  }, []); 

  const logout = async () => {
    const token = localStorage.getItem("access_token");
    
    // 1. Notifier le backend (qui notifiera les autres via WebSocket)
    if (token) {
        try {
            // Adaptez l'URL si votre route est différente (/auth/logout ou /logout)
            await fetch("http://localhost:3000/auth/logout", {
                method: "POST",
                headers: { 
                    "Authorization": `Bearer ${token}` 
                    // Pas besoin de Content-Type si pas de body
                },
            });
        } catch (err) {
            console.error("Erreur appel API logout", err);
        }
    }

    // 2. Nettoyage Frontend
    if (socket) {
        socket.close();
        setSocket(null);
    }
    localStorage.removeItem("access_token");
    localStorage.removeItem("username");
    localStorage.removeItem("current_user_id");
    setUser(null);
    setServers([]);
    
    // 3. Redirection
    window.location.href = "http://localhost:3001";
  };

  const addServer = (server: Server) => {
    setServers((prev) => [...prev, server]);
  };

  return (
    <AuthContext.Provider value={{ user, servers, isLoading, socket, banNotification, clearBanNotification, setServers, addServer, logout, refreshUserData }}>
      {children}
    </AuthContext.Provider>
  );
}

export function useAuth() {
  const context = useContext(AuthContext);
  if (context === undefined) {
    throw new Error("useAuth must be used within an AuthProvider");
  }
  return context;
}