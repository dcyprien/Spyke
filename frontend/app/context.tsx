"use client";

import { createContext, useContext, useState, useEffect, useRef, ReactNode } from "react";
import { Server } from "../components/chatbar";
import { useLang } from "./langContext";

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
  banNotifications: { message: string; serverId: number }[];
  dismissBanNotification: () => void;
  addServer: (server: Server) => void;
  setServers: (servers: Server[]) => void; 
  logout: () => void;
  refreshUserData: () => Promise<void>;
}

const AuthContext = createContext<AuthState | undefined>(undefined);

export function AuthProvider({ children }: { children: ReactNode }) {
  const [user, setUser] = useState<User | null>(null);
  const [servers, setServers] = useState<Server[]>([]);
  const [isLoading, setIsLoading] = useState(true);
  const [socket, setSocket] = useState<WebSocket | null>(null);
  const [banNotifications, setBanNotifications] = useState<{ message: string; serverId: number }[]>([]);

  const { t } = useLang();
  const tRef = useRef(t);
  useEffect(() => { tRef.current = t; }, [t]);

  const dismissBanNotification = () => setBanNotifications(prev => prev.slice(1));

  // Helper to push a notification into the queue (deduplicated by serverId)
  const pushBanNotification = (notif: { message: string; serverId: number }) => {
    setBanNotifications(prev => {
      if (prev.some(n => n.serverId === notif.serverId && n.message === notif.message)) return prev;
      return [...prev, notif];
    });
  };

  // Ref stable pour lire le user_id courant depuis la closure WS (stale closure fix)
  const currentUserIdRef = useRef<string | null>(null);

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
        // Stocker l'ID pour la détection des events WS (kick/ban)
        currentUserIdRef.current = String(data.id);
        localStorage.setItem("current_user_id", data.id);

        const mappedServers = (data.servers || []).map((s: any) => ({
          ...s,
          invitcode: s.invitcode ? String(s.invitcode) : "",
        }));
        setServers(mappedServers);

        // Show notifications for bans that happened while offline
        if (data.pending_bans && data.pending_bans.length > 0) {
          for (const ban of data.pending_bans) {
            let msg: string;
            if (!ban.banned_until) {
              msg = tRef.current.ban_offline_perm(ban.server_name);
            } else {
              const until = new Date(ban.banned_until);
              const diffSecs = Math.round((until.getTime() - Date.now()) / 1000);
              if (diffSecs <= 15) {
                msg = tRef.current.ban_offline_kicked(ban.server_name);
              } else {
                const days = Math.floor(diffSecs / 86400);
                const hours = Math.floor(diffSecs / 3600);
                if (days >= 1) msg = tRef.current.ban_offline_days(ban.server_name, days);
                else if (hours >= 1) msg = tRef.current.ban_offline_hours(ban.server_name, hours);
                else msg = tRef.current.ban_offline_minutes(ban.server_name, Math.ceil(diffSecs / 60));
              }
            }
            pushBanNotification({ message: msg, serverId: ban.server_id });
          }
        }
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
                    // Stocker l'ID immédiatement pour éviter la race condition
                    if (parsed.data?.user_id) {
                        currentUserIdRef.current = String(parsed.data.user_id);
                        localStorage.setItem("current_user_id", String(parsed.data.user_id));
                    }
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
                    const currentUserId = currentUserIdRef.current || localStorage.getItem("current_user_id");
                    if (currentUserId && String(data.user_id) === String(currentUserId)) {
                        pushBanNotification({ message: tRef.current.ban_kicked, serverId: data.server_id });
                        setServers((prev: any[]) => prev.filter((s: any) => String(s.id) !== String(data.server_id)));
                    } else {
                        setServers((prev: any[]) => prev.map((server: any) => {
                            if (String(server.id) === String(data.server_id)) {
                                return { ...server, members: (server.members || []).filter((m: any) => String(m.user_id) !== String(data.user_id)) };
                            }
                            return server;
                        }));
                    }
                }

                if (parsed.type === "user_banned") {
                    const currentUserId = currentUserIdRef.current || localStorage.getItem("current_user_id");
                    if (currentUserId && String(data.user_id) === String(currentUserId)) {
                        let msg: string;
                        if (!data.banned_until) {
                            msg = tRef.current.ban_permanent;
                        } else {
                            const until = new Date(data.banned_until);
                            const diffSecs = Math.round((until.getTime() - Date.now()) / 1000);
                            if (diffSecs <= 15) {
                                msg = tRef.current.ban_kicked;
                            } else {
                                const hours = Math.floor(diffSecs / 3600);
                                const days = Math.floor(diffSecs / 86400);
                                if (days >= 1) msg = tRef.current.ban_temp_days(days);
                                else if (hours >= 1) msg = tRef.current.ban_temp_hours(hours);
                                else msg = tRef.current.ban_temp_minutes(Math.ceil(diffSecs / 60));
                            }
                        }
                        pushBanNotification({ message: msg, serverId: data.server_id });
                        setServers((prev: any[]) => prev.filter((s: any) => String(s.id) !== String(data.server_id)));
                    } else {
                        // Retirer le membre banni de la liste pour les autres en temps réel
                        setServers((prev: any[]) => prev.map((server: any) => {
                            if (String(server.id) === String(data.server_id)) {
                                return { ...server, members: (server.members || []).filter((m: any) => String(m.user_id) !== String(data.user_id)) };
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
    <AuthContext.Provider value={{ user, servers, isLoading, socket, banNotifications, dismissBanNotification, setServers, addServer, logout, refreshUserData }}>
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